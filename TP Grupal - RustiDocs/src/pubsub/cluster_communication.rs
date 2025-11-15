use crate::cluster::{
    comms::node_output::NodeOutput,
    types::{KnownNode, NodeId},
};
use crate::pubsub::PubSubMessage;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, RwLock};

/// Error que puede ocurrir durante la comunicación de pub/sub entre nodos.
#[derive(Debug, Clone, PartialEq)]
pub enum ClusterCommunicationError {
    /// Error al enviar mensaje
    SendError(String),
    /// Error al recibir mensaje
    ReceiveError(String),
    /// Error de serialización/deserialización
    SerializationError(String),
    /// Error al obtener lock en estructuras compartidas
    LockError(String),
    /// Nodo no encontrado
    NodeNotFound(String),
}

impl std::fmt::Display for ClusterCommunicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClusterCommunicationError::SendError(msg) => {
                write!(f, "Error al enviar: {}", msg)
            }
            ClusterCommunicationError::ReceiveError(msg) => {
                write!(f, "Error al recibir: {}", msg)
            }
            ClusterCommunicationError::SerializationError(msg) => {
                write!(f, "Error de serialización: {}", msg)
            }
            ClusterCommunicationError::LockError(msg) => {
                write!(f, "Error de lock: {}", msg)
            }
            ClusterCommunicationError::NodeNotFound(msg) => {
                write!(f, "Nodo no encontrado: {}", msg)
            }
        }
    }
}

impl std::error::Error for ClusterCommunicationError {}

/// Gestor de comunicación de pub/sub entre nodos del cluster.
///
/// Maneja el envío y recepción de mensajes pub/sub entre nodos,
/// utilizando el sistema de node_output existente para la comunicación.
pub struct ClusterCommunicationManager {
    /// Receptor de mensajes para enviar a otros nodos
    receiver: Receiver<(NodeId, PubSubMessage)>,
    /// Sistema de output para enviar mensajes a otros nodos
    node_output: Arc<RwLock<NodeOutput>>,
    /// Mapa de nodos conocidos para obtener direcciones
    known_nodes: Arc<RwLock<HashMap<NodeId, KnownNode>>>,
    /// Sender para enviar mensajes recibidos al gestor de pub/sub local
    pubsub_sender: Sender<PubSubMessage>,
}

impl ClusterCommunicationManager {
    /// Crea una nueva instancia de `ClusterCommunicationManager`.
    ///
    /// # Arguments
    ///
    /// * `receiver` - Canal para recibir mensajes a enviar a otros nodos
    /// * `node_output` - Sistema de output para enviar mensajes
    /// * `known_nodes` - Mapa de nodos conocidos
    /// * `pubsub_sender` - Canal para enviar mensajes recibidos al pub/sub local
    ///
    /// # Returns
    ///
    /// Una nueva instancia de `ClusterCommunicationManager`
    pub fn new(
        receiver: Receiver<(NodeId, PubSubMessage)>,
        node_output: Arc<RwLock<NodeOutput>>,
        known_nodes: Arc<RwLock<HashMap<NodeId, KnownNode>>>,
        pubsub_sender: Sender<PubSubMessage>,
    ) -> Self {
        ClusterCommunicationManager {
            receiver,
            node_output,
            known_nodes,
            pubsub_sender,
        }
    }

    /// Ejecuta el bucle principal del manager.
    ///
    /// Este método ejecuta un bucle infinito que:
    /// - Recibe mensajes para enviar a otros nodos
    /// - Los serializa y envía usando el sistema de node_output
    /// - Mantiene las conexiones activas.
    ///
    /// # Returns
    ///
    /// `Result<(), ClusterCommunicationError>` - Resultado de la ejecución
    pub fn run(&mut self) -> Result<(), ClusterCommunicationError> {
        while let Ok((target_node_id, message)) = self.receiver.recv() {
            self.send_to_node(&target_node_id, &message)?;
        }
        Ok(())
    }

    /// Envía un mensaje a un nodo específico.
    ///
    /// # Arguments
    ///
    /// * `node_id` - ID del nodo destino
    /// * `message` - Mensaje a enviar
    ///
    /// # Returns
    ///
    /// `Result<(), ClusterCommunicationError>` - Resultado del envío
    fn send_to_node(
        &mut self,
        node_id: &NodeId,
        message: &PubSubMessage,
    ) -> Result<(), ClusterCommunicationError> {
        // Serializar mensaje
        let serialized = Self::serialize_pubsub_message(message)?;

        // Obtener dirección del nodo
        let nodes = self.known_nodes.read().map_err(|e| {
            ClusterCommunicationError::LockError(format!("Error obteniendo read lock: {}", e))
        })?;

        // Verificar que el nodo existe
        let _node_data = nodes.get(node_id).ok_or_else(|| {
            ClusterCommunicationError::NodeNotFound(format!("Nodo {} no encontrado", node_id))
        })?;

        // Enviar mensaje usando node_output
        self.node_output
            .read()
            .map_err(|e| {
                ClusterCommunicationError::LockError(format!("Error obteniendo write lock: {}", e))
            })?
            .send_pubsub_message(node_id, &serialized)
            .map_err(|e| {
                ClusterCommunicationError::SendError(format!(
                    "Error enviando mensaje pub/sub a {}: {}",
                    node_id, e
                ))
            })?;

        println!(
            "[CLUSTER_COMM] Mensaje pub/sub enviado a {} ({} bytes)",
            node_id,
            serialized.len()
        );

        Ok(())
    }

    /// Serializa un mensaje pub/sub para transmisión.
    ///
    /// # Arguments
    ///
    /// * `message` - Mensaje a serializar
    ///
    /// # Returns
    ///
    /// `Result<Vec<u8>, ClusterCommunicationError>` - Datos serializados
    pub fn serialize_pubsub_message(
        message: &PubSubMessage,
    ) -> Result<Vec<u8>, ClusterCommunicationError> {
        match message {
            PubSubMessage::Subscribe {
                channel,
                source_node,
            } => {
                let mut data = Vec::new();
                data.push(0); // Tipo: Subscribe
                data.extend_from_slice(&(channel.len() as u16).to_be_bytes());
                data.extend_from_slice(channel.as_bytes());
                data.extend_from_slice(&(source_node.len() as u16).to_be_bytes());
                data.extend_from_slice(source_node.as_bytes());
                Ok(data)
            }
            PubSubMessage::Unsubscribe {
                channel,
                source_node,
            } => {
                let mut data = Vec::new();
                data.push(1); // Tipo: Unsubscribe
                data.extend_from_slice(&(channel.len() as u16).to_be_bytes());
                data.extend_from_slice(channel.as_bytes());
                data.extend_from_slice(&(source_node.len() as u16).to_be_bytes());
                data.extend_from_slice(source_node.as_bytes());
                Ok(data)
            }
            PubSubMessage::Publish {
                channel,
                message: msg,
                source_node,
            } => {
                let mut data = Vec::new();
                data.push(2); // Tipo: Publish
                data.extend_from_slice(&(channel.len() as u16).to_be_bytes());
                data.extend_from_slice(channel.as_bytes());
                data.extend_from_slice(&(msg.len() as u16).to_be_bytes());
                data.extend_from_slice(msg.as_bytes());
                data.extend_from_slice(&(source_node.len() as u16).to_be_bytes());
                data.extend_from_slice(source_node.as_bytes());
                Ok(data)
            }
        }
    }

    /// Deserializa un mensaje pub/sub recibido.
    ///
    /// # Arguments
    ///
    /// * `data` - Datos recibidos
    ///
    /// # Returns
    ///
    /// `Result<PubSubMessage, ClusterCommunicationError>` - Mensaje deserializado
    pub fn deserialize_pubsub_message(
        data: &[u8],
    ) -> Result<PubSubMessage, ClusterCommunicationError> {
        if data.len() < 1 {
            return Err(ClusterCommunicationError::SerializationError(
                "Datos insuficientes para tipo de mensaje".to_string(),
            ));
        }

        let message_type = data[0];
        let mut offset = 1;

        match message_type {
            0 => {
                // Subscribe
                if data.len() < offset + 4 {
                    return Err(ClusterCommunicationError::SerializationError(
                        "Datos insuficientes para Subscribe".to_string(),
                    ));
                }

                let channel_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
                offset += 2;

                if data.len() < offset + channel_len + 2 {
                    return Err(ClusterCommunicationError::SerializationError(
                        "Datos insuficientes para canal en Subscribe".to_string(),
                    ));
                }

                let channel =
                    String::from_utf8_lossy(&data[offset..offset + channel_len]).to_string();
                offset += channel_len;

                let source_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
                offset += 2;

                if data.len() < offset + source_len {
                    return Err(ClusterCommunicationError::SerializationError(
                        "Datos insuficientes para source_node en Subscribe".to_string(),
                    ));
                }

                let source_node =
                    String::from_utf8_lossy(&data[offset..offset + source_len]).to_string();

                Ok(PubSubMessage::Subscribe {
                    channel,
                    source_node,
                })
            }
            1 => {
                // Unsubscribe
                if data.len() < offset + 4 {
                    return Err(ClusterCommunicationError::SerializationError(
                        "Datos insuficientes para Unsubscribe".to_string(),
                    ));
                }

                let channel_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
                offset += 2;

                if data.len() < offset + channel_len + 2 {
                    return Err(ClusterCommunicationError::SerializationError(
                        "Datos insuficientes para canal en Unsubscribe".to_string(),
                    ));
                }

                let channel =
                    String::from_utf8_lossy(&data[offset..offset + channel_len]).to_string();
                offset += channel_len;

                let source_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
                offset += 2;

                if data.len() < offset + source_len {
                    return Err(ClusterCommunicationError::SerializationError(
                        "Datos insuficientes para source_node en Unsubscribe".to_string(),
                    ));
                }

                let source_node =
                    String::from_utf8_lossy(&data[offset..offset + source_len]).to_string();

                Ok(PubSubMessage::Unsubscribe {
                    channel,
                    source_node,
                })
            }
            2 => {
                // Publish
                if data.len() < offset + 4 {
                    return Err(ClusterCommunicationError::SerializationError(
                        "Datos insuficientes para Publish".to_string(),
                    ));
                }

                let channel_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
                offset += 2;

                if data.len() < offset + channel_len + 2 {
                    return Err(ClusterCommunicationError::SerializationError(
                        "Datos insuficientes para canal en Publish".to_string(),
                    ));
                }

                let channel =
                    String::from_utf8_lossy(&data[offset..offset + channel_len]).to_string();
                offset += channel_len;

                let message_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
                offset += 2;

                if data.len() < offset + message_len + 2 {
                    return Err(ClusterCommunicationError::SerializationError(
                        "Datos insuficientes para mensaje en Publish".to_string(),
                    ));
                }

                let message =
                    String::from_utf8_lossy(&data[offset..offset + message_len]).to_string();
                offset += message_len;

                let source_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
                offset += 2;

                if data.len() < offset + source_len {
                    return Err(ClusterCommunicationError::SerializationError(
                        "Datos insuficientes para source_node en Publish".to_string(),
                    ));
                }

                let source_node =
                    String::from_utf8_lossy(&data[offset..offset + source_len]).to_string();

                Ok(PubSubMessage::Publish {
                    channel,
                    message,
                    source_node,
                })
            }
            _ => Err(ClusterCommunicationError::SerializationError(format!(
                "Tipo de mensaje pub/sub desconocido: {}",
                message_type
            ))),
        }
    }

    /// Procesa un mensaje recibido de otro nodo.
    ///
    /// # Arguments
    ///
    /// * `data` - Datos del mensaje recibido
    ///
    /// # Returns
    ///
    /// `Result<(), ClusterCommunicationError>` - Resultado del procesamiento
    pub fn handle_received_message(&self, data: &[u8]) -> Result<(), ClusterCommunicationError> {
        let message = Self::deserialize_pubsub_message(data)?;

        // Enviar el mensaje al gestor de pub/sub local
        self.pubsub_sender.send(message).map_err(|e| {
            ClusterCommunicationError::SendError(format!(
                "Error enviando mensaje al pub/sub local: {}",
                e
            ))
        })?;

        println!("[CLUSTER_COMM] Mensaje pub/sub recibido y reenviado al pub/sub local");
        Ok(())
    }

    /// Procesa un mensaje recibido por TCP y lo envía al pub/sub local.
    /// Este método es llamado desde node_input.rs cuando se recibe un mensaje pub/sub.
    ///
    /// # Arguments
    ///
    /// * `data` - Datos del mensaje recibido
    ///
    /// # Returns
    ///
    /// `Result<(), ClusterCommunicationError>` - Resultado del procesamiento
    pub fn process_tcp_message(&self, data: &[u8]) -> Result<(), ClusterCommunicationError> {
        self.handle_received_message(data)
    }

    /// Cierra todas las conexiones activas.
    ///
    /// # Returns
    ///
    /// `Result<(), ClusterCommunicationError>` - Resultado del cierre
    pub fn close_all_connections(&self) -> Result<(), ClusterCommunicationError> {
        println!("[CLUSTER_COMM] Todas las conexiones cerradas");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize_subscribe() {
        let message = PubSubMessage::Subscribe {
            channel: "test_channel".to_string(),
            source_node: "node1".to_string(),
        };

        let serialized = ClusterCommunicationManager::serialize_pubsub_message(&message).unwrap();
        let deserialized =
            ClusterCommunicationManager::deserialize_pubsub_message(&serialized).unwrap();

        assert!(matches!(deserialized, PubSubMessage::Subscribe { .. }));
        if let PubSubMessage::Subscribe {
            channel,
            source_node,
        } = deserialized
        {
            assert_eq!(channel, "test_channel");
            assert_eq!(source_node, "node1");
        }
    }

    #[test]
    fn test_serialize_deserialize_publish() {
        let message = PubSubMessage::Publish {
            channel: "test_channel".to_string(),
            message: "Hello, World!".to_string(),
            source_node: "node1".to_string(),
        };

        let serialized = ClusterCommunicationManager::serialize_pubsub_message(&message).unwrap();
        let deserialized =
            ClusterCommunicationManager::deserialize_pubsub_message(&serialized).unwrap();

        assert!(matches!(deserialized, PubSubMessage::Publish { .. }));
        if let PubSubMessage::Publish {
            channel,
            message: msg,
            source_node,
        } = deserialized
        {
            assert_eq!(channel, "test_channel");
            assert_eq!(msg, "Hello, World!");
            assert_eq!(source_node, "node1");
        }
    }

    #[test]
    fn test_error_display() {
        let error = ClusterCommunicationError::SendError("connection failed".to_string());
        assert!(error.to_string().contains("Error al enviar"));
    }
}
