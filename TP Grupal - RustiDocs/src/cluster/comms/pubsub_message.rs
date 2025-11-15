use crate::cluster::{
    state::node_data::NodeData,
    types::{KnownNode, NodeId, NodeMessage},
};
use crate::pubsub::{PubSubMessage, cluster_communication::ClusterCommunicationManager};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock, mpsc::Sender};

/// Procesa mensajes pub/sub recibidos de otros nodos del cluster.
///
/// # Arguments
///
/// * `message` - Mensaje recibido
/// * `_node_data` - Datos del nodo local
/// * `_known_nodes` - Nodos conocidos en el cluster
/// * `_output_sender` - Sender para enviar respuestas
/// * `pubsub_sender` - Sender para enviar mensajes al gestor de pub/sub local
///
/// # Returns
///
/// `Result<(), String>` - Resultado del procesamiento
pub fn process_pubsub_msg(
    message: NodeMessage,
    _node_data: &Arc<RwLock<NodeData>>,
    _known_nodes: &Arc<RwLock<HashMap<NodeId, KnownNode>>>,
    _output_sender: &Sender<(NodeId, SocketAddr, Option<Vec<u8>>)>,
    pubsub_sender: &Sender<PubSubMessage>,
) -> Result<(), String> {
    println!(
        "[PUBSUB] Procesando mensaje pub/sub de {} ({} bytes)",
        message.get_src_id(),
        message.get_payload().len()
    );

    // Deserializar el mensaje pub/sub
    let pubsub_message =
        ClusterCommunicationManager::deserialize_pubsub_message(&message.get_payload())
            .map_err(|e| format!("Error deserializando mensaje pub/sub: {}", e))?;

    // Enviar el mensaje al gestor de pub/sub local
    pubsub_sender
        .send(pubsub_message)
        .map_err(|e| format!("Error enviando mensaje al gestor pub/sub local: {}", e))?;

    println!("[PUBSUB] Mensaje pub/sub procesado exitosamente");
    Ok(())
}

/// Crea un mensaje pub/sub para enviar a otros nodos.
///
/// # Arguments
///
/// * `node_data` - Datos del nodo local
/// * `pubsub_message` - Mensaje pub/sub a enviar
///
/// # Returns
///
/// `NodeMessage` - Mensaje listo para enviar
#[allow(dead_code)]
pub fn create_pubsub_node_message(
    node_data: &NodeData,
    pubsub_message: &PubSubMessage,
) -> Result<NodeMessage, String> {
    let serialized = ClusterCommunicationManager::serialize_pubsub_message(pubsub_message)
        .map_err(|e| format!("Error serializando mensaje pub/sub: {}", e))?;

    let message = NodeMessage::new(
        node_data.get_id(),
        node_data.get_ip(),
        node_data.get_port(),
        crate::cluster::types::PUBSUB_TYPE,
        serialized.len() as u16,
        serialized,
    );

    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cluster::state::node_data::NodeData;
    use crate::config::node_configs::NodeConfigs;

    fn create_test_node_data() -> NodeData {
        let configs = NodeConfigs::new(&"./tests/utils/test.conf".to_string())
            .expect("No se pudo cargar la configuración de prueba");
        NodeData::new(configs)
    }

    #[test]
    fn test_create_pubsub_node_message() {
        let node_data = create_test_node_data();
        let pubsub_message = PubSubMessage::Subscribe {
            channel: "test_channel".to_string(),
            source_node: "test_node".to_string(),
        };

        let result = create_pubsub_node_message(&node_data, &pubsub_message);
        assert!(result.is_ok());

        let node_message = result.unwrap();
        assert_eq!(
            node_message.get_request_type(),
            crate::cluster::types::PUBSUB_TYPE
        );
        assert_eq!(node_message.get_src_id(), "test_node");
    }
}
