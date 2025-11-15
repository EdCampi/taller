//! Funciones relacionadas con el procesamiento de las conexiones
//! y los mensajes propios de la comunicación internodal.

// IMPORTS
use crate::cluster::comms::failing_node::process_node_fail_msg;
use crate::cluster::comms::gossip_receiver::process_gossip_msg;
use crate::cluster::comms::join_message::process_join_msg;
use crate::cluster::comms::psync_reciever::process_psync_message;
use crate::cluster::comms::pubsub_message::process_pubsub_msg;
use crate::cluster::comms::replica_promotion::process_promotion_msg;
use crate::cluster::sharding::rehash_message::process_rehash_msg;
use crate::cluster::state::node_data::NodeData;
use crate::cluster::time_tracker::TimeTracker;
use crate::cluster::types::{
    CONNECTION_CLOSE_TYPE, DEFAULT_BUFFER_SIZE, FAIL_TYPE, GOSSIP_TYPE, JOIN_TYPE, KnownNode,
    NodeId, NodeMessage, PROMOTION_TYPE, PUBSUB_TYPE, REHASH_TYPE, REQUEST_PSYNC_TYPE,
};
use crate::pubsub::distributed_manager::PubSubMessage;
use crate::security::tls_lite::{TlsServerConfig, TlsServerStream};
use crate::storage::data_store::DataStore;
use std::io::Read;
use std::time::Duration;
use std::{
    collections::HashMap,
    io,
    io::{BufRead, BufReader, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{Arc, RwLock, mpsc::Sender},
    thread,
};

pub static NODAL_COMMS_PORT: u16 = 10000;
static PEEK_INTERVAL: u64 = 100;

// Trait para streams que pueden ser usados en comunicación nodo-nodo
trait NodeInputStream: Read + Write + Send {}

impl NodeInputStream for TcpStream {}
impl NodeInputStream for TlsServerStream {}

#[derive(Clone)]
pub enum NodeInputEncryptionType {
    None,
    Tls,
}

pub fn start_listening(
    node_data_lock: Arc<RwLock<NodeData>>,
    output_sender: Sender<(NodeId, SocketAddr, Option<Vec<u8>>)>,
    known_nodes: Arc<RwLock<HashMap<NodeId, KnownNode>>>,
    tracker_lock: Arc<RwLock<TimeTracker>>,
    pubsub_sender: Sender<PubSubMessage>,
    data_store: Arc<RwLock<DataStore>>,
) {
    start_listening_with_encryption(
        node_data_lock,
        output_sender,
        known_nodes,
        tracker_lock,
        pubsub_sender,
        data_store,
        NodeInputEncryptionType::None,
    );
}

pub fn start_listening_with_encryption(
    node_data_lock: Arc<RwLock<NodeData>>,
    output_sender: Sender<(NodeId, SocketAddr, Option<Vec<u8>>)>,
    known_nodes: Arc<RwLock<HashMap<NodeId, KnownNode>>>,
    tracker_lock: Arc<RwLock<TimeTracker>>,
    pubsub_sender: Sender<PubSubMessage>,
    data_store: Arc<RwLock<DataStore>>,
    encryption_type: NodeInputEncryptionType,
) {
    let node_data_aux = node_data_lock.clone();
    let output_sender_aux = output_sender.clone();
    let known_nodes_aux = known_nodes.clone();
    let tracker_lock_aux = tracker_lock.clone();
    let pubsub_sender_aux = pubsub_sender.clone();
    let data_store_aux = data_store.clone();
    let encryption_type_aux = encryption_type.clone();

    let node_data = node_data_lock.read().unwrap();
    let addr = node_data.get_addr();
    let port = addr.port() + NODAL_COMMS_PORT;
    let ip = addr.ip();
    drop(node_data);

    let node_ip_str = format!("{}:{}", ip, port);
    let node_addr: SocketAddr = node_ip_str.parse().unwrap();

    thread::spawn(move || {
        let listener = TcpListener::bind(node_addr).unwrap();
        println!("[NI-NODE] Listening node communication on {}", node_addr);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!(
                        "[NI-CLUSTER] New connection from {:?}",
                        stream.peer_addr().unwrap()
                    );
                    let node_data_aux = node_data_aux.clone();
                    let output_sender_aux = output_sender_aux.clone();
                    let known_nodes_aux = known_nodes_aux.clone();
                    let tracker_lock_aux = tracker_lock_aux.clone();
                    let pubsub_sender_aux = pubsub_sender_aux.clone();
                    let data_store_aux = data_store_aux.clone();
                    let encryption_type_aux = encryption_type_aux.clone();

                    thread::spawn(move || {
                        handle_connection(
                            stream,
                            node_data_aux,
                            output_sender_aux,
                            known_nodes_aux,
                            tracker_lock_aux,
                            pubsub_sender_aux,
                            data_store_aux,
                            encryption_type_aux,
                        );
                    });
                }
                Err(e) => println!("[NI-CLUSTER] Connection failed: {}", e),
            }
        }
    });
}

fn handle_connection(
    stream: TcpStream,
    node_data: Arc<RwLock<NodeData>>,
    output_sender: Sender<(NodeId, SocketAddr, Option<Vec<u8>>)>,
    known_nodes: Arc<RwLock<HashMap<NodeId, KnownNode>>>,
    tracker_lock: Arc<RwLock<TimeTracker>>,
    pubsub_sender: Sender<PubSubMessage>,
    data_store: Arc<RwLock<DataStore>>,
    encryption_type: NodeInputEncryptionType,
) {
    // Aplicar encriptación según el tipo configurado
    let aux = stream.peer_addr();
    if let Err(e) = stream.set_read_timeout(Some(Duration::from_secs(15))) {
        println!("Error when setting read timeout: {}", e);
    }
    let encrypted_stream: Box<dyn NodeInputStream> = match encryption_type {
        NodeInputEncryptionType::None => {
            println!("[NI-CLUSTER] Conexión sin encriptación");
            Box::new(stream)
        }
        NodeInputEncryptionType::Tls => {
            println!("[NI-CLUSTER] Aplicando TLS");
            let server_config = TlsServerConfig::new();
            match TlsServerStream::new(stream, server_config) {
                Ok(tls_stream) => Box::new(tls_stream),
                Err(e) => {
                    println!("[NI-CLUSTER] Error en handshake TLS: {}", e);
                    return;
                }
            }
        }
    };

    // Para streams encriptados, no podemos usar peek, así que leemos directamente
    let mut buffer = BufReader::new(encrypted_stream);
    loop {
        // Intentar leer un mensaje completo
        match read_stream(
            &mut buffer,
            &node_data,
            &output_sender,
            &known_nodes,
            &tracker_lock,
            &pubsub_sender,
            &data_store,
        ) {
            Ok(_) => {
                // Mensaje procesado exitosamente, continuar
            }
            Err(e) => {
                println!(
                    "[NI-CLUSTER] error al leer stream: {:?}. Cerrando conexión con {:?}.",
                    e, aux
                );
                break; // Cerramos la conexión ante cualquier error de lectura
            }
        }
    }
    println!("[NI-CLUSTER] Closing node connection");
}

fn read_stream(
    buffer: &mut BufReader<Box<dyn NodeInputStream>>,
    node_data: &Arc<RwLock<NodeData>>,
    output_sender: &Sender<(NodeId, SocketAddr, Option<Vec<u8>>)>,
    known_nodes: &Arc<RwLock<HashMap<NodeId, KnownNode>>>,
    tracker_lock: &Arc<RwLock<TimeTracker>>,
    pubsub_sender: &Sender<PubSubMessage>,
    data_store: &Arc<RwLock<DataStore>>,
) -> Result<(), String> {
    let mut line = Vec::new();

    match read_until_sequence(buffer, b"<END>", &mut line) {
        Ok(n) if n > 0 => {
            let message = NodeMessage::from_bytes(&mut io::Cursor::new(line))?;
            // No puedo hacer peek, cierro la conexión para que el stream no lea sin haber nada.
            if message.get_request_type() == CONNECTION_CLOSE_TYPE {
                println!("[NI-CLUSTER] Recibido mensaje de cierre de conexión");
                return Err("[FALSE] Connection closed by peer".to_string());
            }
            println!(
                "[NI-CLUSTER] Recibí mensaje tipo: {}",
                map_type_to_variable_name(message.get_request_type())
            );

            match message.get_request_type() {
                GOSSIP_TYPE => {
                    process_gossip_msg(message, node_data, output_sender, known_nodes, tracker_lock)
                }
                JOIN_TYPE => process_join_msg(message, node_data, output_sender, known_nodes),
                REHASH_TYPE => process_rehash_msg(message, node_data, known_nodes, output_sender),
                FAIL_TYPE => process_node_fail_msg(message, node_data, known_nodes),
                PROMOTION_TYPE => process_promotion_msg(message, node_data, known_nodes),
                PUBSUB_TYPE => process_pubsub_msg(
                    message,
                    node_data,
                    known_nodes,
                    output_sender,
                    pubsub_sender,
                ),
                REQUEST_PSYNC_TYPE => {
                    process_psync_message(message, node_data, data_store, output_sender)
                }
                _ => Err("[NI-CLUSTER] Wrong message type received".to_string()),
            }
        }
        Ok(_) => Err("[NI-CLUSTER] Connection closed".to_string()),
        // Catch WouldBlock errors separately if needed
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
            thread::sleep(Duration::from_millis(PEEK_INTERVAL));
            Ok(())
        }
        Err(e) => Err(format!("Error reading from stream: {}", e)),
    }
}

/// Auxiliar para printear el tipo de mensaje recibido.
fn map_type_to_variable_name(msg_type: u8) -> &'static str {
    match msg_type {
        GOSSIP_TYPE => "GOSSIP_TYPE",
        JOIN_TYPE => "JOIN_TYPE",
        REHASH_TYPE => "REHASH_TYPE",
        FAIL_TYPE => "FAIL_TYPE",
        PROMOTION_TYPE => "PROMOTION_TYPE",
        PUBSUB_TYPE => "PUBSUB_TYPE",
        REQUEST_PSYNC_TYPE => "REQUEST_PSYNC_TYPE",
        _ => "UNKNOWN_TYPE",
    }
}

/// Para leer hasta el delimitador multibyte.
fn read_until_sequence(
    reader: &mut BufReader<Box<dyn NodeInputStream>>,
    delimiter: &[u8],
    buffer: &mut Vec<u8>,
) -> io::Result<usize> {
    let mut temp = [0u8; DEFAULT_BUFFER_SIZE];
    let mut total_read = 0;

    loop {
        let bytes_read = reader.read(&mut temp)?;
        if bytes_read == 0 {
            break;
        }

        buffer.extend_from_slice(&temp[..bytes_read]);
        total_read += bytes_read;

        if let Some(pos) = buffer.windows(delimiter.len()).position(|w| w == delimiter) {
            buffer.truncate(pos + delimiter.len());
            break;
        }
    }
    Ok(total_read)
}

#[allow(dead_code)]
pub fn start_replica_sync(
    local_node_id: NodeId,
    known_nodes: Arc<RwLock<HashMap<NodeId, KnownNode>>>,
) {
    thread::spawn(move || {
        let master_id;
        let master_addr;

        {
            let nodes = known_nodes.read().unwrap();
            let local_node = match nodes.get(&local_node_id) {
                Some(n) => n,
                None => {
                    println!(
                        "[REPLICA {}] No se encontró el nodo en known_nodes",
                        local_node_id
                    );
                    return;
                }
            };

            master_id = match local_node.get_master_id() {
                Some(id) => id.clone(),
                None => {
                    println!("[REPLICA {}] No tiene master asignado", local_node_id);
                    return;
                }
            };

            master_addr = match nodes.get(&master_id) {
                Some(m) => m.get_addr(),
                None => {
                    println!(
                        "[REPLICA {}] No se encontró al master {} en known_nodes",
                        local_node_id, master_id
                    );
                    return;
                }
            };
        }

        println!(
            "[REPLICA {}] Conectando a master {} en {}",
            local_node_id, master_id, master_addr
        );

        if let Ok(mut stream) = TcpStream::connect(master_addr) {
            let sync_command = b"PSYNC ? -1\n";
            if let Err(e) = stream.write_all(sync_command) {
                println!(
                    "[REPLICA {}] Error al enviar PSYNC al master {}: {}",
                    local_node_id, master_id, e
                );
                return;
            }

            println!(
                "[REPLICA {}] Enviado PSYNC ? -1 a master {}",
                local_node_id, master_id
            );

            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            while let Ok(n) = reader.read_line(&mut line) {
                if n == 0 {
                    break;
                }
                println!("[REPLICA {}] Recibido: {}", local_node_id, line.trim());
                line.clear();
            }
        } else {
            println!(
                "[REPLICA {}] No pudo conectar a master {} en {}",
                local_node_id, master_id, master_addr
            );
        }
    });
}

#[cfg(test)]
mod tests {

    pub fn save_data_from_vector(vec: Vec<u8>) -> Vec<u8> {
        let mut vec_serialized: Vec<u8> = Vec::new();

        let len_bits = (vec.len() * 8) as u8;
        let len_bits_bytes = len_bits.to_be_bytes();
        vec_serialized.extend_from_slice(&len_bits_bytes);

        for &element in &vec {
            let elem_len_bits = 8u8;
            vec_serialized.push(elem_len_bits);
            vec_serialized.push(element);
        }

        vec_serialized
    }

    pub fn reconstruct_vector(vec_serialized: Vec<u8>) -> Vec<u8> {
        let mut vec_reconstructed: Vec<u8> = Vec::new();

        if vec_serialized.len() < 2 {
            return vec_reconstructed;
        }

        let total_bits = vec_serialized[0] as usize;
        let len = total_bits / 8;

        let mut index = 1;
        for _ in 0..len {
            if index + 1 >= vec_serialized.len() {
                break;
            }
            let _elem_len_bits = vec_serialized[index];
            let elem = vec_serialized[index + 1];
            vec_reconstructed.push(elem);
            index += 2;
        }

        vec_reconstructed
    }

    #[test]
    fn test_save_data_from_vector() {
        let data = vec![1, 2, 3, 4, 5];
        let serialized = save_data_from_vector(data.clone());
        println!("Serialized data: {:?}", serialized);
    }

    #[test]
    fn test_reconstruct_vector() {
        let data = vec![1, 2, 3, 4, 59, 7];
        let serialized = save_data_from_vector(data.clone());
        let reconstructed = reconstruct_vector(serialized.clone());
        println!("Reconstructed data: {:?}", reconstructed);
        println!("Original data: {:?}", serialized);
    }
}
