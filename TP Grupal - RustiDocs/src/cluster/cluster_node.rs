//! Módulo encargado de manejar el ciclo de vida del nodo dentro del cluster.

use std::{
    collections::HashMap,
    error::Error,
    io::Write,
    net::{SocketAddr, TcpStream},
    sync::{
        Arc, RwLock,
        mpsc::{Receiver, Sender, channel},
    },
    thread,
};

use crate::cluster::{
    comms::{
        gossip_sender::GossipSender,
        join_message::JoinMessage,
        node_input::{NODAL_COMMS_PORT, NodeInputEncryptionType, start_listening_with_encryption},
        node_output::{NodeEncryptionType, NodeOutput},
        psync_sender::psync_sender,
    },
    state::node_data::NodeData,
    time_tracker::TimeTracker,
    types::{JOIN_TYPE, KnownNode, NodeId, NodeMessage, SlotRange},
};

use crate::command::{command_executor::CommandExecutor, instruction::Instruction, types::Command};

use crate::{config::node_configs::NodeConfigs, logs::aof_logger::AofLogger};

use crate::network::{connection_handler::Handler, resp_message::RespMessage};

use crate::pubsub::{
    cluster_communication::ClusterCommunicationManager,
    distributed_manager::{DistributedPubSubManager, PubSubMessage},
};

use crate::security::{
    TlsClientConfig,
    users::{acl::load_users_from_acl, user_base::UserBase},
};

use crate::storage::{
    data_store::DataStore, disk_loader::DiskLoader, snapshot_manager::SnapshotManager,
};

pub static NODE_TIMEOUT: u64 = 10000; // Tiempo en ms hasta timeout para ping/pong.
pub static PING_INTERVAL: u64 = 750; // Tiempo en ms hasta el próximo ping.
pub static GOSSIP_SECTION_ENTRIES: u64 = 3;
pub static SLOTS_RANGE: SlotRange = (0, 16383);

pub struct ClusterNode {
    configs: NodeConfigs,
    node_data: Arc<RwLock<NodeData>>,
    logger: Arc<AofLogger>,
    known_nodes: Arc<RwLock<HashMap<NodeId, KnownNode>>>,
    pub tls_server_name: Option<String>,
}

impl ClusterNode {
    pub fn new(configs: NodeConfigs) -> Result<Self, Box<dyn Error>> {
        let node_data = Arc::new(RwLock::new(NodeData::new(configs.clone())));
        let logger = AofLogger::new(configs.clone());
        let known_nodes = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            configs,
            node_data,
            logger,
            known_nodes,
            tls_server_name: Some("localhost".to_string()), // Habilitar TLS por defecto
        })
    }

    /// Crea un ClusterNode con TLS
    pub fn new_with_tls(configs: NodeConfigs, server_name: String) -> Result<Self, Box<dyn Error>> {
        let mut node = Self::new(configs)?;
        node.tls_server_name = Some(server_name);
        Ok(node)
    }

    /// Crea un ClusterNode sin encriptación
    pub fn new_without_encryption(configs: NodeConfigs) -> Result<Self, Box<dyn Error>> {
        let node_data = Arc::new(RwLock::new(NodeData::new(configs.clone())));
        let logger = AofLogger::new(configs.clone());
        let known_nodes = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            configs,
            node_data,
            logger,
            known_nodes,
            tls_server_name: None, // Sin encriptación
        })
    }

    pub fn start(&mut self, known_node: Option<String>) -> Result<(), Box<dyn Error>> {
        let ds = self.load_ds()?;
        self.start_snapshot(ds.clone());

        let (instruction_sender, instruction_receiver) =
            channel::<(String, Instruction, Sender<RespMessage>)>();
        let (pubsub_sender, pubsub_receiver) = channel();

        self.start_command_executor(ds.clone(), instruction_receiver, pubsub_sender);
        self.start_client_connections_handler(instruction_sender.clone());

        ClusterNode::connect_to_cluster(
            self.configs.clone(),
            known_node,
            Some(self.node_data.clone()),
        );
        println!(
            "[NODE] Node started, addr {} with ID {}",
            self.configs.get_addr(),
            self.configs.get_id()
        );

        let (output_sender, output_receiver) = channel::<(NodeId, SocketAddr, Option<Vec<u8>>)>();
        let tracker = Arc::new(RwLock::new(TimeTracker::new(NODE_TIMEOUT)));

        // Determinar tipo de encriptación para node_output
        let node_output_encryption = if self.tls_server_name.is_some() {
            NodeEncryptionType::Tls
        } else {
            NodeEncryptionType::None
        };

        // Determinar tipo de encriptación para node_input
        let _ = if self.tls_server_name.is_some() {
            NodeInputEncryptionType::Tls
        } else {
            NodeInputEncryptionType::None
        };

        // Crear node_output con encriptación
        let node_output = match node_output_encryption {
            NodeEncryptionType::None => Arc::new(RwLock::new(NodeOutput::new(
                output_receiver,
                tracker.clone(),
            ))),
            NodeEncryptionType::Tls => {
                let server_name = self.tls_server_name.as_ref().unwrap().clone();
                Arc::new(RwLock::new(NodeOutput::new_with_tls(
                    output_receiver,
                    tracker.clone(),
                    server_name,
                )))
            }
        };

        // Iniciar el pubsub manager con el NodeOutput compartido
        let cluster_pubsub_sender = self.start_pubsub_manager(pubsub_receiver, node_output.clone());

        // Iniciar la comunicación entre nodos
        self.start_node_port_comms(
            output_sender,
            cluster_pubsub_sender,
            tracker,
            node_output,
            ds,
        );

        loop {
            thread::park(); // TODO: Agregar consola y mantener vivo con eso
        }
    }

    fn load_ds(&self) -> Result<Arc<RwLock<DataStore>>, Box<dyn Error>> {
        let loader = DiskLoader::new(self.configs.clone(), self.logger.clone());
        loader.load().map_err(|e| e.into())
    }

    fn start_snapshot(&self, ds: Arc<RwLock<DataStore>>) {
        let snap_configs = self.configs.clone();
        let snap_logger = self.logger.clone();
        let mut snapshotter = SnapshotManager::new(ds, snap_configs, snap_logger);
        snapshotter.start();
    }

    fn start_command_executor(
        &self,
        ds: Arc<RwLock<DataStore>>,
        instruction_receiver: Receiver<(String, Instruction, Sender<RespMessage>)>,
        pubsub_sender: Sender<(String, Command, Sender<String>, Sender<RespMessage>)>,
    ) {
        let logger_clone = self.logger.clone();
        let ds_clone = ds.clone();
        let configs_clone = self.configs.clone();
        let known_nodes_clone = self.known_nodes.clone();
        let data_clone = self.node_data.clone();
        thread::spawn(move || {
            let mut executor = CommandExecutor::new(
                ds_clone,
                instruction_receiver,
                configs_clone,
                logger_clone,
                pubsub_sender,
                known_nodes_clone,
                data_clone,
            );
            executor.run();
        });
    }

    fn start_pubsub_manager(
        &self,
        pubsub_receiver: Receiver<(String, Command, Sender<String>, Sender<RespMessage>)>,
        node_output: Arc<RwLock<NodeOutput>>,
    ) -> Sender<PubSubMessage> {
        // Crear canales para comunicación distribuida
        let (cluster_pubsub_sender, cluster_pubsub_receiver) = channel::<PubSubMessage>();
        let (cluster_outgoing_sender, cluster_outgoing_receiver) =
            channel::<(NodeId, PubSubMessage)>();

        // Clonar referencias necesarias
        let known_nodes_clone = self.known_nodes.clone();
        let local_node_id = self.configs.get_id();

        let mut cluster_comm_manager = ClusterCommunicationManager::new(
            cluster_outgoing_receiver,
            node_output,
            known_nodes_clone.clone(),
            cluster_pubsub_sender.clone(),
        );

        // Iniciar el gestor de comunicación en un hilo separado
        thread::spawn(move || {
            if let Err(e) = cluster_comm_manager.run() {
                eprintln!("Error en ClusterCommunicationManager: {}", e);
            }
        });

        // Iniciar el gestor de pub/sub distribuido
        thread::spawn(move || {
            let mut distributed_manager = DistributedPubSubManager::new(
                pubsub_receiver,
                cluster_pubsub_receiver,
                local_node_id,
                known_nodes_clone,
                cluster_outgoing_sender,
            );

            if let Err(e) = distributed_manager.run() {
                eprintln!("Error en DistributedPubSubManager: {}", e);
            }
        });
        cluster_pubsub_sender
    }

    fn start_client_connections_handler(
        &self,
        instruction_sender: Sender<(String, Instruction, Sender<RespMessage>)>,
    ) {
        let user_base = load_users_from_acl("user.acl").unwrap_or(UserBase::new());
        // Handler
        let connection_handler = Handler::new(
            instruction_sender.clone(),
            self.configs.clone(),
            self.logger.clone(),
            user_base,
        );
        thread::spawn(move || {
            let _ = connection_handler.init();
        });
    }

    pub fn connect_to_cluster(
        configs: NodeConfigs,
        known_node: Option<String>,
        node_data_lock: Option<Arc<RwLock<NodeData>>>,
    ) {
        // Inicializar conexión con otro nodo si se pasa
        if let Some(addr) = known_node {
            println!("[CLUSTER] Trying to connect with {}", addr);
            let addr_clone = addr.clone();

            // Parsear la dirección para obtener IP y puerto
            let cluster_addr = if let Ok(socket_addr) = addr.parse::<SocketAddr>() {
                // Si es una dirección completa, convertir al puerto de comunicación entre nodos
                format!(
                    "{}:{}",
                    socket_addr.ip(),
                    socket_addr.port() + NODAL_COMMS_PORT
                )
            } else {
                // Si es solo un puerto, asumir localhost
                format!(
                    "0.0.0.0:{}",
                    addr.parse::<u16>().unwrap_or(7001) + NODAL_COMMS_PORT
                )
            };

            println!("[CLUSTER] Connecting to cluster port: {}", cluster_addr);

            if let Ok(stream) = TcpStream::connect(&cluster_addr) {
                // Crear stream encriptado si TLS está habilitado
                let mut encrypted_stream: Box<dyn Write> = {
                    // Por ahora asumimos que TLS está habilitado por defecto
                    println!("[CLUSTER] Aplicando TLS para conexión saliente");
                    let client_config = TlsClientConfig::new("localhost".to_string());
                    match crate::security::tls_lite::TlsClientStream::new(stream, client_config) {
                        Ok(tls_stream) => Box::new(tls_stream),
                        Err(e) => {
                            println!("[CLUSTER] Error en handshake TLS saliente: {}", e);
                            return;
                        }
                    }
                };

                let join_msg = JoinMessage::new(
                    configs.get_id(),
                    configs.get_node_ip(),
                    configs.get_node_port(),
                );
                let data = join_msg.to_bytes();
                let length = data.len() as u16;
                let aux = NodeMessage::new(
                    configs.get_id(),
                    configs.get_node_ip(),
                    configs.get_node_port(),
                    JOIN_TYPE,
                    length,
                    data,
                );

                if let Err(e) = encrypted_stream.write_all(&aux.serialize()) {
                    println!("[CLUSTER] Error enviando JoinMessage: {}", e);
                } else {
                    if let Err(_) = encrypted_stream.flush() {
                        println!("[CLUSTER] Error flushing JOIN stream");
                    }
                    send_close_message(&mut encrypted_stream);
                    drop(encrypted_stream);
                    println!(
                        "[CLUSTER] JoinMessage sent and connection closing with {} via TLS",
                        addr_clone
                    );
                }
            } else {
                println!("[CLUSTER] couldn't connect with {}", addr_clone);
            }
        } else {
            // Por default el nodo se inicia como master con todos los slots disponibles
            println!("[CLUSTER] First node of the cluster, taking every slot");
            if let Some(node_data) = node_data_lock {
                node_data.write().unwrap().set_slots(SLOTS_RANGE);
                node_data.write().unwrap().set_as_master();
            }
        }
    }

    fn start_node_port_comms(
        &self,
        output_sender: Sender<(NodeId, SocketAddr, Option<Vec<u8>>)>,
        pubsub_sender: Sender<PubSubMessage>,
        tracker: Arc<RwLock<TimeTracker>>,
        node_output: Arc<RwLock<NodeOutput>>,
        data_store: Arc<RwLock<DataStore>>,
    ) {
        let settings_listener_clone = self.node_data.clone();
        let nodes_ref_clone = self.known_nodes.clone();
        let tracker_clone = tracker.clone();
        let data_store_clone = data_store.clone();
        let output_sender_clone = output_sender.clone();

        // Determinar tipo de encriptación para node_input
        let node_input_encryption = if self.tls_server_name.is_some() {
            NodeInputEncryptionType::Tls
        } else {
            NodeInputEncryptionType::None
        };

        let _ = thread::Builder::new()
            .name("node_listener".to_string())
            .spawn(move || {
                start_listening_with_encryption(
                    settings_listener_clone,
                    output_sender_clone,
                    nodes_ref_clone,
                    tracker_clone,
                    pubsub_sender,
                    data_store_clone,
                    node_input_encryption,
                );
            });

        // Sección psync
        let psync_node_data = self.node_data.clone();
        let psync_data_store = data_store.clone();
        let psync_output = output_sender.clone();
        let psync_known_nodes = self.known_nodes.clone();
        let _ = thread::Builder::new()
            .name("node_listener".to_string())
            .spawn(move || {
                psync_sender(
                    psync_node_data,
                    psync_data_store,
                    psync_output,
                    psync_known_nodes,
                )
            });

        // Usar el NodeOutput compartido para el GossipSender
        let mut gossip_sender = GossipSender::new(node_output, tracker);
        gossip_sender.ping(
            self.node_data.clone(),
            self.known_nodes.clone(),
            GOSSIP_SECTION_ENTRIES,
            PING_INTERVAL,
        );
    }
}

pub fn send_close_message(stream: &mut Box<dyn Write>) {
    let msg = NodeMessage::create_close_connection_msg();
    stream.write_all(&msg.serialize()).unwrap();
}
