use crate::cluster::time_tracker::TimeTracker;
use crate::cluster::types::NodeMessage;
use crate::cluster::types::{NodeId, PUBSUB_TYPE};
use crate::security::tls_lite::{TlsClientConfig, TlsClientStream};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

// Trait para streams que pueden ser usados en comunicación nodo-nodo
trait NodeStream: Read + Write + Send {}

impl NodeStream for TcpStream {}
impl NodeStream for TlsClientStream {}

/// Errores específicos del manejo de conexiones salientes.
#[derive(Debug, PartialEq)]
pub enum NodeOutputError {
    /// Error de conexión de red
    NetworkError(String),
    /// Error al obtener lock en estructuras compartidas
    LockError(String),
    /// Error al enviar datos
    SendError(String),
    /// Nodo no encontrado en el pool de conexiones
    NodeNotFound(String),
    /// Error de encriptación
    EncryptionError(String),
}

impl std::fmt::Display for NodeOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeOutputError::NetworkError(msg) => write!(f, "Error de red: {}", msg),
            NodeOutputError::LockError(msg) => write!(f, "Error de lock: {}", msg),
            NodeOutputError::SendError(msg) => write!(f, "Error de envío: {}", msg),
            NodeOutputError::NodeNotFound(msg) => write!(f, "Nodo no encontrado: {}", msg),
            NodeOutputError::EncryptionError(msg) => write!(f, "Error de encriptación: {}", msg),
        }
    }
}

impl std::error::Error for NodeOutputError {}

/// Recibe un paquete de datos y a que nodo enviarselo
/// a diferencia de ClientOutput que cada instancia contenia la salida de un solo cliente.
/// node output contiene el socket de todos los nodos del cluster.
pub struct NodeOutput {
    node_sockets: Arc<Mutex<HashMap<NodeId, Box<dyn NodeStream>>>>,
    tracker: Arc<RwLock<TimeTracker>>,
    encryption_enabled: bool,
    encryption_type: NodeEncryptionType,
}

#[derive(Debug, Clone)]
pub enum NodeEncryptionType {
    None,
    Tls,
}

impl NodeOutput {
    pub fn new(
        node_receiver: Receiver<(NodeId, SocketAddr, Option<Vec<u8>>)>,
        tracker: Arc<RwLock<TimeTracker>>,
    ) -> Self {
        let mut res = NodeOutput {
            node_sockets: Arc::new(Mutex::new(HashMap::new())),
            tracker,
            encryption_enabled: false,
            encryption_type: NodeEncryptionType::None,
        };
        res.run(node_receiver);
        res
    }

    /// Crea un NodeOutput con TLS
    pub fn new_with_tls(
        node_receiver: Receiver<(NodeId, SocketAddr, Option<Vec<u8>>)>,
        tracker: Arc<RwLock<TimeTracker>>,
        _: String,
    ) -> Self {
        let mut res = NodeOutput {
            node_sockets: Arc::new(Mutex::new(HashMap::new())),
            tracker,
            encryption_enabled: true,
            encryption_type: NodeEncryptionType::Tls,
        };
        res.run(node_receiver);
        res
    }

    pub fn run(&mut self, node_receiver: Receiver<(NodeId, SocketAddr, Option<Vec<u8>>)>) {
        let aux = self.node_sockets.clone();
        let encryption_type = self.encryption_type.clone();
        thread::spawn(move || {
            loop {
                match node_receiver.try_recv() {
                    Ok(data) => {
                        NodeOutput::add_node_socket(
                            aux.clone(),
                            data.0.clone(),
                            data.1,
                            &encryption_type,
                        );
                        if let Some(payload) = data.2 {
                            let mut connected_nodes = aux.lock().unwrap();
                            if let Some(stream) = connected_nodes.get_mut(&data.0) {
                                if let Err(e) = write_complete(stream, &payload) {
                                    println!(
                                        "[NO-CLUSTER] Error al mandar datos a {}, ADDR {:?}, {:?}",
                                        data.0, data.1, e
                                    );
                                }
                            } else {
                                println!("[NO-CLUSTER] No hay conexión nodo_src -> nodo_dst");
                            }
                        }
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        });
    }

    pub fn set_broadcast_channel(&self) -> Sender<Vec<u8>> {
        let (sender, receiver) = channel::<Vec<u8>>();
        let node_sockets = self.node_sockets.clone();
        thread::spawn(move || {
            while let Ok(data) = receiver.recv() {
                let mut sockets = node_sockets.lock().unwrap();
                for stream in sockets.values_mut() {
                    if let Err(e) = write_complete(stream, &data) {
                        eprintln!("Error sending broadcast: {:?}", e);
                    }
                }
            }
        });
        sender
    }

    /// Agrega un socket para un nodo específico con encriptación opcional.
    fn add_node_socket(
        map: Arc<Mutex<HashMap<NodeId, Box<dyn NodeStream>>>>,
        node_id: NodeId,
        node_addr: SocketAddr,
        encryption_type: &NodeEncryptionType,
    ) {
        let mut known_streams = map.lock().unwrap();
        if known_streams.contains_key(&node_id) {
            return;
        }

        let max_retries = 5;
        let mut attempt = 0;

        while attempt < max_retries {
            match TcpStream::connect(node_addr) {
                Ok(stream) => {
                    println!(
                        "[NO-CLUSTER] Nueva conexión con {:?} en {:?}",
                        node_id, node_addr
                    );

                    // Aplicar encriptación según el tipo configurado
                    let encrypted_stream: Box<dyn NodeStream> = match encryption_type {
                        NodeEncryptionType::None => {
                            println!("[NO-CLUSTER] Conexión sin encriptación");
                            Box::new(stream)
                        }
                        NodeEncryptionType::Tls => {
                            println!("[NO-CLUSTER] Aplicando TLS");
                            let client_config = TlsClientConfig::new("localhost".to_string());
                            match TlsClientStream::new(stream, client_config) {
                                Ok(tls_stream) => Box::new(tls_stream),
                                Err(e) => {
                                    println!("[NO-CLUSTER] Error en handshake TLS: {}", e);
                                    attempt += 1;
                                    thread::sleep(Duration::from_millis(500));
                                    continue;
                                }
                            }
                        }
                    };

                    known_streams.insert(node_id, encrypted_stream);
                    return; // Éxito, salimos
                }
                Err(e) => {
                    println!(
                        "[NO-CLUSTER] Intento {} de {}: error al conectar con {}: {}",
                        attempt + 1,
                        max_retries,
                        node_addr,
                        e,
                    );
                    attempt += 1;
                    thread::sleep(Duration::from_millis(500));
                }
            }
        }

        println!(
            "[NO-CLUSTER] No se pudo establecer conexión con nodo {} en {}",
            node_id, node_addr
        );
    }

    pub fn open_connection_with(&mut self, node_id: NodeId, node_addr: SocketAddr) {
        NodeOutput::add_node_socket(
            self.node_sockets.clone(),
            node_id,
            node_addr,
            &self.encryption_type,
        );
    }

    pub fn send_to_node(&mut self, node_id: &NodeId, msg: NodeMessage, ping_id: Option<u64>) {
        let mut map = self.node_sockets.lock().unwrap();

        if let Some(stream) = map.get_mut(node_id) {
            match write_complete(stream, &msg.serialize()) {
                Ok(_) => {
                    if let Some(id) = ping_id {
                        let mut tracker = self.tracker.write().unwrap();
                        tracker.add_entry(node_id.clone(), id);
                    }
                }
                Err(e) => {
                    println!(
                        "Error al enviar stream hacia {} desde output: {}",
                        node_id, e
                    );
                    if let Some(id) = ping_id {
                        let mut tracker = self.tracker.write().unwrap();
                        tracker.remove_entry(id);
                    }
                }
            }
        } else {
            println!("No hay conexión con el nodo: {}", node_id);
            if let Some(id) = ping_id {
                let mut tracker = self.tracker.write().unwrap();
                tracker.remove_entry(id);
            }
        }
    }

    /// Envía un mensaje pub/sub a un nodo específico.
    ///
    /// # Arguments
    ///
    /// * `node_id` - ID del nodo destino
    /// * `data` - Datos del mensaje pub/sub a enviar
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Mensaje enviado exitosamente
    /// * `Err(NodeOutputError)` - Error durante el envío
    pub fn send_pubsub_message(
        &self,
        node_id: &NodeId,
        data: &[u8],
    ) -> Result<(), NodeOutputError> {
        let mut sockets = self.node_sockets.lock().map_err(|e| {
            NodeOutputError::LockError(format!(
                "Error obteniendo write lock para nodo {}: {}",
                node_id, e
            ))
        })?;

        let stream = sockets.get_mut(node_id).ok_or_else(|| {
            NodeOutputError::NodeNotFound(format!("Nodo {} no encontrado en el pool", node_id))
        })?;

        // Crear un NodeMessage completo con el header correcto
        let node_message = NodeMessage::new(
            "".to_string(),
            "".to_string(),
            0,
            PUBSUB_TYPE,
            data.len() as u16,
            data.to_vec(),
        );

        // Serializar y enviar el mensaje completo
        let serialized_message = node_message.serialize();
        write_complete(stream, &serialized_message).map_err(|e| {
            NodeOutputError::SendError(format!(
                "Error enviando mensaje pub/sub a {}: {}",
                node_id, e
            ))
        })?;

        println!(
            "[NODE_OUTPUT] Mensaje pub/sub enviado a {} ({} bytes)",
            node_id,
            serialized_message.len()
        );

        Ok(())
    }

    /// Verifica si la encriptación está habilitada
    pub fn is_encryption_enabled(&self) -> bool {
        self.encryption_enabled
    }

    /// Obtiene el tipo de encriptación
    pub fn get_encryption_type(&self) -> &NodeEncryptionType {
        &self.encryption_type
    }
}

fn write_complete(stream: &mut Box<dyn NodeStream>, data: &[u8]) -> std::io::Result<()> {
    let mut written = 0;
    while written < data.len() {
        match stream.write(&data[written..]) {
            Ok(n) => written += n,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(1));
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    stream.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cluster::time_tracker::TimeTracker;
    use crate::cluster::types::{GOSSIP_TYPE, NodeMessage};
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_timeout_cleanup_on_send_error() {
        // Crear un TimeTracker
        let tracker = Arc::new(RwLock::new(TimeTracker::new(5000)));

        // Crear un NodeOutput con el tracker
        let (_, receiver) = channel();
        let mut node_output = NodeOutput::new(receiver, tracker.clone());

        // Crear un mensaje de ping
        let ping_msg = NodeMessage::new(
            "test_node".to_string(),
            "0.0.0.0".to_string(),
            7001,
            GOSSIP_TYPE,
            0,
            vec![],
        );

        // Intentar enviar a un nodo que no existe (simular error)
        node_output.send_to_node(&"nonexistent_node".to_string(), ping_msg, Some(123));

        // Verificar que la entrada se removió del TimeTracker
        let mut tracker_guard = tracker.write().unwrap();
        // El TimeTracker debería estar vacío porque se removió la entrada al fallar el envío
        assert_eq!(tracker_guard.verify_timeout(), None);
    }
}
