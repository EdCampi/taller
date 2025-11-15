use crate::cluster::types::{KnownNode, NodeId};
use crate::command::types::Command;
use crate::network::resp_message::RespMessage;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

/// Error que puede ocurrir durante el manejo de pub/sub distribuido.
#[derive(Debug, Clone, PartialEq)]
pub enum DistributedPubSubError {
    /// Error al recibir mensaje del receptor
    ReceiveError(String),
    /// Error al enviar respuesta al cliente
    SendResponseError(String),
    /// Error al enviar mensaje a suscriptor local
    SendToSubscriberError(String),
    /// Error al enviar mensaje a nodo remoto
    SendToRemoteNodeError(String),
    /// Error al suscribir cliente a canal
    SubscribeError(String),
    /// Error al desuscribir cliente de canal
    UnsubscribeError(String),
    /// Error al publicar mensaje en canal
    PublishError(String),
    /// Error de comando no soportado
    UnsupportedCommandError(String),
    /// Error de canal no encontrado
    ChannelNotFoundError(String),
    /// Error de cliente no encontrado
    ClientNotFoundError(String),
    /// Error de red al conectar con nodo remoto
    NetworkError(String),
    /// Error de serialización/deserialización
    SerializationError(String),
}

impl fmt::Display for DistributedPubSubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DistributedPubSubError::ReceiveError(msg) => {
                write!(f, "Error al recibir mensaje: {}", msg)
            }
            DistributedPubSubError::SendResponseError(msg) => {
                write!(f, "Error al enviar respuesta: {}", msg)
            }
            DistributedPubSubError::SendToSubscriberError(msg) => {
                write!(f, "Error al enviar mensaje a suscriptor local: {}", msg)
            }
            DistributedPubSubError::SendToRemoteNodeError(msg) => {
                write!(f, "Error al enviar mensaje a nodo remoto: {}", msg)
            }
            DistributedPubSubError::SubscribeError(msg) => {
                write!(f, "Error al suscribir cliente: {}", msg)
            }
            DistributedPubSubError::UnsubscribeError(msg) => {
                write!(f, "Error al desuscribir cliente: {}", msg)
            }
            DistributedPubSubError::PublishError(msg) => {
                write!(f, "Error al publicar mensaje: {}", msg)
            }
            DistributedPubSubError::UnsupportedCommandError(msg) => {
                write!(f, "Comando no soportado: {}", msg)
            }
            DistributedPubSubError::ChannelNotFoundError(msg) => {
                write!(f, "Canal no encontrado: {}", msg)
            }
            DistributedPubSubError::ClientNotFoundError(msg) => {
                write!(f, "Cliente no encontrado: {}", msg)
            }
            DistributedPubSubError::NetworkError(msg) => {
                write!(f, "Error de red: {}", msg)
            }
            DistributedPubSubError::SerializationError(msg) => {
                write!(f, "Error de serialización: {}", msg)
            }
        }
    }
}

impl std::error::Error for DistributedPubSubError {}

/// Tipos de mensajes para comunicación entre nodos
#[derive(Debug, Clone)]
pub enum PubSubMessage {
    /// Suscripción a un canal
    Subscribe {
        channel: String,
        source_node: NodeId,
    },
    /// Desuscripción de un canal
    Unsubscribe {
        channel: String,
        source_node: NodeId,
    },
    /// Publicación de mensaje
    Publish {
        channel: String,
        message: String,
        source_node: NodeId,
    },
}

/// Gestor de pub/sub distribuido para el cluster.
///
/// Maneja la suscripción y desuscripción de clientes a canales,
/// así como la publicación de mensajes a los suscriptores tanto
/// locales como remotos en el cluster.
///
/// # Comportamiento
///
/// - Si se hace SUBSCRIBE de un canal que no existe, se crea automáticamente
/// - Un cliente puede hacer PUBLISH aunque no esté suscrito a un canal
/// - Si un canal se queda sin suscriptores, se elimina automáticamente
/// - Los mensajes se propagan a todos los nodos del cluster
pub struct DistributedPubSubManager {
    /// Receptor de mensajes locales
    receiver: Receiver<(String, Command, Sender<String>, Sender<RespMessage>)>,
    /// Receptor de mensajes de otros nodos
    cluster_receiver: Receiver<PubSubMessage>,
    /// Mapa de canales locales: channel_id -> { client_id -> sender al cliente }
    local_channels: HashMap<String, HashMap<String, Sender<RespMessage>>>,
    /// Mapa de suscriptores remotos: channel_id -> HashSet<NodeId>
    remote_subscribers: HashMap<String, HashSet<NodeId>>,
    /// ID del nodo local
    local_node_id: NodeId,
    /// Lista de nodos conocidos en el cluster
    known_nodes: Arc<RwLock<HashMap<NodeId, KnownNode>>>,
    /// Sender para enviar mensajes a otros nodos
    cluster_sender: Sender<(NodeId, PubSubMessage)>,
}

impl DistributedPubSubManager {
    /// Crea un nuevo gestor de pub/sub distribuido.
    ///
    /// # Arguments
    ///
    /// * `receiver` - Receptor de mensajes locales
    /// * `cluster_receiver` - Receptor de mensajes de otros nodos
    /// * `local_node_id` - ID del nodo local
    /// * `known_nodes` - Lista de nodos conocidos en el cluster
    /// * `cluster_sender` - Sender para enviar mensajes a otros nodos
    ///
    /// # Returns
    ///
    /// `DistributedPubSubManager` - Una nueva instancia del gestor
    pub fn new(
        receiver: Receiver<(String, Command, Sender<String>, Sender<RespMessage>)>,
        cluster_receiver: Receiver<PubSubMessage>,
        local_node_id: NodeId,
        known_nodes: Arc<RwLock<HashMap<NodeId, KnownNode>>>,
        cluster_sender: Sender<(NodeId, PubSubMessage)>,
    ) -> Self {
        Self {
            receiver,
            cluster_receiver,
            local_channels: HashMap::new(),
            remote_subscribers: HashMap::new(),
            local_node_id,
            known_nodes,
            cluster_sender,
        }
    }

    /// Ejecuta el bucle principal del manager.
    ///
    /// Este método maneja tanto mensajes locales como mensajes de otros nodos
    /// del cluster, manteniendo sincronizados los canales pub/sub distribuidos.
    ///
    /// # Returns
    ///
    /// `Result<(), DistributedPubSubError>` - Resultado de la ejecución
    pub fn run(&mut self) -> Result<(), DistributedPubSubError> {
        println!("[DISTRIBUTED_PUBSUB] Manager iniciado y ejecutándose...");

        loop {
            // Procesar mensajes locales (de clientes)
            match self.receiver.try_recv() {
                Ok((client_id, command, response_sender, client_sender)) => {
                    println!(
                        "[DISTRIBUTED_PUBSUB] Mensaje local recibido: {:?} de cliente {}",
                        command, client_id
                    );
                    if let Err(e) =
                        self.handle_command(client_id, command, response_sender, client_sender)
                    {
                        eprintln!("Error manejando comando local: {}", e);
                    }
                }
                Err(TryRecvError::Empty) => {
                    // No hay mensajes locales, continuar
                }
                Err(TryRecvError::Disconnected) => {
                    return Err(DistributedPubSubError::ReceiveError(
                        "Canal de mensajes locales desconectado".to_string(),
                    ));
                }
            }

            // Procesar mensajes de otros nodos
            match self.cluster_receiver.try_recv() {
                Ok(message) => {
                    println!(
                        "[DISTRIBUTED_PUBSUB] Mensaje de cluster recibido: {:?}",
                        message
                    );
                    if let Err(e) = self.handle_cluster_message(message) {
                        eprintln!("Error manejando mensaje de cluster: {}", e);
                    }
                }
                Err(TryRecvError::Empty) => {
                    // No hay mensajes de cluster, continuar
                }
                Err(TryRecvError::Disconnected) => {
                    return Err(DistributedPubSubError::ReceiveError(
                        "Canal de mensajes de cluster desconectado".to_string(),
                    ));
                }
            }

            // Pequeña pausa para evitar uso excesivo de CPU
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Maneja un comando local.
    ///
    /// # Arguments
    ///
    /// * `client_id` - ID del cliente
    /// * `command` - Comando a ejecutar
    /// * `response_sender` - Sender para enviar respuesta
    /// * `client_sender` - Sender para enviar mensajes al cliente
    ///
    /// # Returns
    ///
    /// `Result<(), DistributedPubSubError>` - Resultado del manejo
    fn handle_command(
        &mut self,
        client_id: String,
        command: Command,
        response_sender: Sender<String>,
        client_sender: Sender<RespMessage>,
    ) -> Result<(), DistributedPubSubError> {
        match command {
            Command::Subscribe(channel_id) => {
                self.handle_subscribe(client_id, channel_id, response_sender, client_sender)
            }
            Command::Unsubscribe(channel_id) => {
                self.handle_unsubscribe(client_id, channel_id, response_sender)
            }
            Command::Publish(channel_id, message) => {
                self.handle_publish(channel_id, message, response_sender)
            }
            _ => Err(DistributedPubSubError::UnsupportedCommandError(format!(
                "Comando no soportado: {:?}",
                command
            ))),
        }
    }

    /// Maneja el comando de suscripción.
    ///
    /// # Arguments
    ///
    /// * `client_id` - ID del cliente que se suscribe
    /// * `channel_id` - ID del canal al que suscribirse
    /// * `response_sender` - Sender para enviar respuesta
    /// * `client_sender` - Sender para enviar mensajes al cliente
    ///
    /// # Returns
    ///
    /// `Result<(), DistributedPubSubError>` - Resultado de la suscripción
    fn handle_subscribe(
        &mut self,
        client_id: String,
        channel_id: String,
        response_sender: Sender<String>,
        client_sender: Sender<RespMessage>,
    ) -> Result<(), DistributedPubSubError> {
        println!(
            "[DISTRIBUTED_PUBSUB] handle_subscribe: cliente={}, canal={}",
            client_id, channel_id
        );

        // Crear el canal si no existe
        self.local_channels
            .entry(channel_id.clone())
            .or_insert_with(HashMap::new);

        // Verificar si el cliente ya está suscrito
        if self.local_channels[&channel_id].contains_key(&client_id) {
            println!(
                "[DISTRIBUTED_PUBSUB] Cliente {} ya está suscrito al canal {}",
                client_id, channel_id
            );
            return self.send_response(
                response_sender,
                "Ya estás suscripto a ese canal".to_string(),
            );
        }

        // Agregar el cliente al canal local
        self.local_channels
            .get_mut(&channel_id)
            .ok_or_else(|| {
                DistributedPubSubError::SubscribeError("No se pudo acceder al canal".to_string())
            })?
            .insert(client_id.clone(), client_sender);

        println!(
            "[DISTRIBUTED_PUBSUB] Cliente {} agregado al canal {} local. Total suscriptores locales: {}",
            client_id,
            channel_id,
            self.local_channels[&channel_id].len()
        );

        // Propagar la suscripción a otros nodos
        println!(
            "[DISTRIBUTED_PUBSUB] Propagando suscripción al canal {} a otros nodos...",
            channel_id
        );
        if let Err(e) = self.propagate_subscribe(&channel_id) {
            println!("[DISTRIBUTED_PUBSUB] Error propagando suscripción: {}", e);
            return Err(e);
        }
        println!("[DISTRIBUTED_PUBSUB] Suscripción propagada exitosamente");

        // Enviar confirmación de éxito
        self.send_response(response_sender, "".to_string())
    }

    /// Maneja el comando de desuscripción.
    ///
    /// # Arguments
    ///
    /// * `client_id` - ID del cliente que se desuscribe
    /// * `channel_id` - ID del canal del que desuscribirse
    /// * `response_sender` - Sender para enviar respuesta
    ///
    /// # Returns
    ///
    /// `Result<(), DistributedPubSubError>` - Resultado de la desuscripción
    fn handle_unsubscribe(
        &mut self,
        client_id: String,
        channel_id: String,
        response_sender: Sender<String>,
    ) -> Result<(), DistributedPubSubError> {
        // Verificar si el canal existe
        if !self.local_channels.contains_key(&channel_id) {
            return self.send_response(response_sender, "Canal no encontrado".to_string());
        }

        // Verificar si el cliente está suscrito
        if !self.local_channels[&channel_id].contains_key(&client_id) {
            return self.send_response(
                response_sender,
                "No estás suscripto a ese canal".to_string(),
            );
        }

        // Remover el cliente del canal
        self.local_channels
            .get_mut(&channel_id)
            .unwrap()
            .remove(&client_id);

        // Si el canal se queda vacío, eliminarlo
        if self.local_channels[&channel_id].is_empty() {
            self.local_channels.remove(&channel_id);
        }

        // Propagar la desuscripción a otros nodos
        self.propagate_unsubscribe(&channel_id)?;

        // Enviar confirmación de éxito
        self.send_response(response_sender, "".to_string())
    }

    /// Maneja el comando de publicación.
    ///
    /// # Arguments
    ///
    /// * `channel_id` - ID del canal donde publicar
    /// * `message` - Mensaje a publicar
    /// * `response_sender` - Sender para enviar respuesta
    ///
    /// # Returns
    ///
    /// `Result<(), DistributedPubSubError>` - Resultado de la publicación
    fn handle_publish(
        &mut self,
        channel_id: String,
        message: RespMessage,
        response_sender: Sender<String>,
    ) -> Result<(), DistributedPubSubError> {
        let mut subscriber_count = 0;

        // Crear el canal local si no existe (para que otros nodos puedan reenviar mensajes)
        self.local_channels
            .entry(channel_id.clone())
            .or_insert_with(HashMap::new);

        // Enviar mensaje a suscriptores locales
        if let Some(subscribers) = self.local_channels.get(&channel_id) {
            for (client_id, sender) in subscribers {
                if let Err(e) = sender.send(message.clone()) {
                    eprintln!("Error enviando mensaje a cliente {}: {}", client_id, e);
                } else {
                    subscriber_count += 1;
                }
            }
        }

        // Propagar el mensaje a otros nodos (siempre, incluso si no hay suscriptores locales)
        if let Err(e) = self.propagate_publish(&channel_id, &message) {
            eprintln!("Error propagando mensaje a otros nodos: {}", e);
            // No fallar por errores de propagación, solo loggear
        }

        // Enviar respuesta con el número de suscriptores (siempre un número)
        self.send_response(response_sender, subscriber_count.to_string())
    }

    /// Propaga una suscripción a otros nodos del cluster.
    ///
    /// # Arguments
    ///
    /// * `channel_id` - ID del canal
    ///
    /// # Returns
    ///
    /// `Result<(), DistributedPubSubError>` - Resultado de la propagación
    fn propagate_subscribe(&self, channel_id: &str) -> Result<(), DistributedPubSubError> {
        println!(
            "[DISTRIBUTED_PUBSUB] propagate_subscribe: canal={}, nodo_local={}",
            channel_id, self.local_node_id
        );

        let message = PubSubMessage::Subscribe {
            channel: channel_id.to_string(),
            source_node: self.local_node_id.clone(),
        };

        println!(
            "[DISTRIBUTED_PUBSUB] Mensaje Subscribe creado: {:?}",
            message
        );

        let result = self.broadcast_to_cluster(message);
        match &result {
            Ok(_) => println!(
                "[DISTRIBUTED_PUBSUB] Mensaje Subscribe enviado exitosamente a todos los nodos"
            ),
            Err(e) => println!(
                "[DISTRIBUTED_PUBSUB] Error enviando mensaje Subscribe: {}",
                e
            ),
        }

        result
    }

    /// Propaga una desuscripción a otros nodos del cluster.
    ///
    /// # Arguments
    ///
    /// * `channel_id` - ID del canal
    ///
    /// # Returns
    ///
    /// `Result<(), DistributedPubSubError>` - Resultado de la propagación
    fn propagate_unsubscribe(&self, channel_id: &str) -> Result<(), DistributedPubSubError> {
        let message = PubSubMessage::Unsubscribe {
            channel: channel_id.to_string(),
            source_node: self.local_node_id.clone(),
        };

        self.broadcast_to_cluster(message)
    }

    /// Propaga una publicación a otros nodos del cluster.
    ///
    /// # Arguments
    ///
    /// * `channel_id` - ID del canal
    /// * `message` - Mensaje a propagar
    ///
    /// # Returns
    ///
    /// `Result<(), DistributedPubSubError>` - Resultado de la propagación
    fn propagate_publish(
        &self,
        channel_id: &str,
        message: &RespMessage,
    ) -> Result<(), DistributedPubSubError> {
        let message_str = match message {
            RespMessage::BulkString(Some(bytes)) => String::from_utf8_lossy(bytes).to_string(),
            RespMessage::SimpleString(s) => s.clone(),
            RespMessage::Integer(i) => i.to_string(),
            _ => {
                return Err(DistributedPubSubError::SerializationError(
                    "Tipo de mensaje no soportado para propagación".to_string(),
                ));
            }
        };

        let pubsub_message = PubSubMessage::Publish {
            channel: channel_id.to_string(),
            message: message_str,
            source_node: self.local_node_id.clone(),
        };

        self.broadcast_to_cluster(pubsub_message)
    }

    /// Envía un mensaje a todos los nodos del cluster excepto al local.
    ///
    /// # Arguments
    ///
    /// * `message` - Mensaje a enviar
    ///
    /// # Returns
    ///
    /// `Result<(), DistributedPubSubError>` - Resultado del envío
    fn broadcast_to_cluster(&self, message: PubSubMessage) -> Result<(), DistributedPubSubError> {
        let known_nodes = self.known_nodes.read().map_err(|e| {
            DistributedPubSubError::NetworkError(format!("Error obteniendo nodos conocidos: {}", e))
        })?;

        for (node_id, _node_data) in known_nodes.iter() {
            if node_id != &self.local_node_id {
                if let Err(e) = self.cluster_sender.send((node_id.clone(), message.clone())) {
                    eprintln!("Error enviando mensaje a nodo {}: {}", node_id, e);
                }
            }
        }

        Ok(())
    }

    /// Maneja mensajes recibidos de otros nodos del cluster.
    ///
    /// # Arguments
    ///
    /// * `message` - Mensaje recibido
    ///
    /// # Returns
    ///
    /// `Result<(), DistributedPubSubError>` - Resultado del procesamiento
    pub fn handle_cluster_message(
        &mut self,
        message: PubSubMessage,
    ) -> Result<(), DistributedPubSubError> {
        println!(
            "[DISTRIBUTED_PUBSUB] Procesando mensaje de cluster: {:?}",
            message
        );
        match message {
            PubSubMessage::Subscribe {
                channel,
                source_node,
            } => {
                println!(
                    "[DISTRIBUTED_PUBSUB] Recibido Subscribe: canal={}, source={}",
                    channel, source_node
                );
                // Registrar que el nodo remoto tiene suscriptores en este canal
                if source_node != self.local_node_id {
                    self.remote_subscribers
                        .entry(channel.clone())
                        .or_insert_with(HashSet::new)
                        .insert(source_node.clone());

                    // Crear el canal local si no existe
                    self.local_channels
                        .entry(channel.clone())
                        .or_insert_with(HashMap::new);

                    println!(
                        "[DISTRIBUTED_PUBSUB] Canal '{}' creado/registrado con suscriptores remotos: {:?}",
                        channel,
                        self.remote_subscribers.get(&channel)
                    );
                }
            }
            PubSubMessage::Unsubscribe {
                channel,
                source_node,
            } => {
                println!(
                    "[DISTRIBUTED_PUBSUB] Recibido Unsubscribe: canal={}, source={}",
                    channel, source_node
                );
                // Remover el nodo remoto de los suscriptores del canal
                if source_node != self.local_node_id {
                    if let Some(subscribers) = self.remote_subscribers.get_mut(&channel) {
                        subscribers.remove(&source_node);
                        if subscribers.is_empty() {
                            self.remote_subscribers.remove(&channel);
                        }
                    }
                    println!(
                        "[DISTRIBUTED_PUBSUB] Suscriptores remotos actualizados para canal '{}': {:?}",
                        channel,
                        self.remote_subscribers.get(&channel)
                    );
                }
            }
            PubSubMessage::Publish {
                channel,
                message,
                source_node,
            } => {
                let channel_clone = channel.clone();
                println!(
                    "[DISTRIBUTED_PUBSUB] Recibido Publish: canal={}, mensaje={}, source={}",
                    channel_clone, message, source_node
                );
                // Reenviar el mensaje a suscriptores locales
                if source_node != self.local_node_id {
                    // Registrar que el nodo remoto tiene suscriptores en este canal
                    self.remote_subscribers
                        .entry(channel.clone())
                        .or_insert_with(HashSet::new)
                        .insert(source_node.clone());

                    // Crear el canal local si no existe
                    self.local_channels
                        .entry(channel.clone())
                        .or_insert_with(HashMap::new);

                    let resp_message = RespMessage::SimpleString(message);
                    if let Some(subscribers) = self.local_channels.get(&channel) {
                        println!(
                            "[DISTRIBUTED_PUBSUB] Encontrados {} suscriptores locales para canal '{}'",
                            subscribers.len(),
                            channel
                        );
                        for (client_id, sender) in subscribers {
                            if let Err(e) = sender.send(resp_message.clone()) {
                                eprintln!(
                                    "Error enviando mensaje propagado a cliente {}: {}",
                                    client_id, e
                                );
                            } else {
                                println!(
                                    "[DISTRIBUTED_PUBSUB] Mensaje enviado exitosamente a cliente {}",
                                    client_id
                                );
                            }
                        }
                    } else {
                        println!(
                            "[DISTRIBUTED_PUBSUB] NO se encontraron suscriptores locales para canal '{}'",
                            channel
                        );
                        println!(
                            "[DISTRIBUTED_PUBSUB] Canales locales disponibles: {:?}",
                            self.local_channels.keys().collect::<Vec<_>>()
                        );
                        println!(
                            "[DISTRIBUTED_PUBSUB] Suscriptores remotos: {:?}",
                            self.remote_subscribers
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Envía una respuesta al cliente.
    ///
    /// # Arguments
    ///
    /// * `response_sender` - Sender para enviar la respuesta
    /// * `message` - Mensaje de respuesta
    ///
    /// # Returns
    ///
    /// `Result<(), DistributedPubSubError>` - Resultado del envío
    fn send_response(
        &self,
        response_sender: Sender<String>,
        message: String,
    ) -> Result<(), DistributedPubSubError> {
        response_sender.send(message).map_err(|e| {
            DistributedPubSubError::SendResponseError(format!("Error enviando respuesta: {}", e))
        })
    }

    // Métodos de utilidad para testing y monitoreo
    pub fn channel_count(&self) -> usize {
        self.local_channels.len()
    }

    pub fn subscriber_count(&self, channel_id: &str) -> Option<usize> {
        self.local_channels
            .get(channel_id)
            .map(|subscribers| subscribers.len())
    }

    pub fn is_subscribed(&self, channel_id: &str, client_id: &str) -> bool {
        self.local_channels
            .get(channel_id)
            .map(|subscribers| subscribers.contains_key(client_id))
            .unwrap_or(false)
    }

    pub fn get_channels(&self) -> Vec<String> {
        self.local_channels.keys().cloned().collect()
    }

    pub fn get_subscribers(&self, channel_id: &str) -> Option<Vec<String>> {
        self.local_channels
            .get(channel_id)
            .map(|subscribers| subscribers.keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn create_test_manager() -> (
        DistributedPubSubManager,
        mpsc::Sender<(
            String,
            Command,
            mpsc::Sender<String>,
            mpsc::Sender<RespMessage>,
        )>,
        mpsc::Sender<PubSubMessage>,
        mpsc::Sender<(NodeId, PubSubMessage)>,
    ) {
        let (local_tx, local_rx) = mpsc::channel();
        let (cluster_tx, cluster_rx) = mpsc::channel();
        let (cluster_sender_tx, _cluster_sender_rx) = mpsc::channel();
        let known_nodes = Arc::new(RwLock::new(HashMap::new()));

        let manager = DistributedPubSubManager::new(
            local_rx,
            cluster_rx,
            "test_node".to_string(),
            known_nodes,
            cluster_sender_tx.clone(),
        );

        (manager, local_tx, cluster_tx, cluster_sender_tx)
    }

    #[test]
    fn test_distributed_pubsub_manager_new() {
        let (manager, _, _, _) = create_test_manager();
        assert_eq!(manager.channel_count(), 0);
    }

    #[test]
    fn test_channel_count() {
        let (mut manager, tx, _, _) = create_test_manager();
        let (response_tx, _response_rx) = mpsc::channel();
        let (client_tx, _client_rx) = mpsc::channel();

        // Suscribir a un canal
        tx.send((
            "client1".to_string(),
            Command::Subscribe("test_channel".to_string()),
            response_tx,
            client_tx,
        ))
        .unwrap();

        // Procesar el mensaje manualmente
        if let Ok((client_id, command, response_sender, client_sender)) = manager.receiver.recv() {
            let _ = manager.handle_command(client_id, command, response_sender, client_sender);
        }

        assert_eq!(manager.channel_count(), 1);
    }

    #[test]
    fn test_error_display() {
        let error = DistributedPubSubError::NetworkError("connection failed".to_string());
        assert!(error.to_string().contains("Error de red"));
    }
}
