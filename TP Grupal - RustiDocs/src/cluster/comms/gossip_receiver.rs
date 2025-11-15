use crate::cluster::cluster_node::GOSSIP_SECTION_ENTRIES;
use crate::cluster::comms::gossip_message::{GossipEntry, GossipMessage, NO_PING_ID};
use crate::cluster::comms::gossip_sender::{create_gossip_msg, set_gossip_data};
use crate::cluster::state::flags::{CONNECTED, FAIL, HANDSHAKE, NodeFlags, PFAIL};
use crate::cluster::state::node_data::NodeData;
use crate::cluster::time_tracker::TimeTracker;
use crate::cluster::types::{KnownNode, NodeId, NodeMessage};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};

// ID
pub fn process_gossip_msg(
    message: NodeMessage,
    node_data_lock: &Arc<RwLock<NodeData>>,
    output_sender: &Sender<(NodeId, SocketAddr, Option<Vec<u8>>)>,
    known_nodes_lock: &Arc<RwLock<HashMap<NodeId, KnownNode>>>,
    tracker_lock: &Arc<RwLock<TimeTracker>>,
) -> Result<(), String> {
    let gossip_msg = GossipMessage::from_bytes(&message.get_payload())
        .map_err(|_| "Error when processing the gossip message".to_string())?;

    //println!("# GSP MSG {:?}", gossip_msg);
    let mut tracker = tracker_lock.write().unwrap();
    //println!("# TRACKER {:?}", tracker);
    let pong_id = gossip_msg.get_pong_id();
    if pong_id != 0 {
        //println!("[PONG] Recibido pong con ID: {} de nodo: {}", pong_id, message.get_src_id());
        tracker.remove_entry(pong_id);
        //println!("[PONG] Entrada removida del TimeTracker para pong_id: {}", pong_id);
    }
    drop(tracker);

    process_gossip_entries(
        &known_nodes_lock,
        &node_data_lock,
        gossip_msg.get_entries(),
        message.get_src_id(),
    );
    if gossip_msg.get_ping_id() != NO_PING_ID {
        //println!("[PING] Enviando pong con ID: {} a nodo: {}", gossip_msg.get_ping_id(), message.get_src_id());
        send_pong(
            message.get_src_id(),
            message.get_addr(),
            gossip_msg.get_ping_id(),
            node_data_lock,
            known_nodes_lock,
            output_sender,
        )
        .map_err(|_| "Error when sending pong".to_string())?;
    }
    Ok(())
}

pub fn process_gossip_entries(
    known_nodes_lock: &Arc<RwLock<HashMap<NodeId, KnownNode>>>,
    node_data_lock: &Arc<RwLock<NodeData>>,
    entries: Vec<GossipEntry>,
    sender_id: NodeId,
) {
    let mut known_nodes = known_nodes_lock.write().unwrap();

    if let Some(sender) = known_nodes.get_mut(&sender_id) {
        if NodeFlags::state_contains(sender.get_state(), HANDSHAKE) {
            sender.set_connected();
        }
    }

    let node_data = node_data_lock.read().unwrap();
    let node_id = node_data.get_id();
    for entry in entries {
        if entry.get_id() == node_id {
            // No me voy a agregar a mí mismo en la lista de nodos conocidos.
            continue;
        }

        if let Some(sender_node) = known_nodes.get_mut(&sender_id) {
            if sender_node.get_id() == entry.get_id() {
                // El sender siempre manda su data actualizada.
                sender_node.force_update(entry.clone());
            }

            if NodeFlags::state_contains(entry.get_state(), PFAIL) {
                sender_node.recognize_as_pfail(entry.get_id());
            } else if sender_node.recognized_as_pfail(&entry.get_id())
                && (NodeFlags::state_contains(entry.get_state(), CONNECTED)
                    || NodeFlags::state_contains(entry.get_state(), FAIL))
            // Si falló, lo saco de la lista
            {
                sender_node.recognize_as_alive(&entry.get_id());
            }
        }

        if let Some(known_node) = known_nodes.get_mut(&entry.get_id()) {
            known_node.update(entry);
        } else {
            let aux = KnownNode::new_from_entry(&entry);
            known_nodes.insert(entry.get_id(), aux.clone());
        }
    }
    drop(node_data);
    drop(known_nodes);
}

pub fn send_pong(
    dst_id: NodeId,
    dst_addr: SocketAddr,
    pong_id: u64,
    node_data_lock: &Arc<RwLock<NodeData>>,
    known_nodes_lock: &Arc<RwLock<HashMap<NodeId, KnownNode>>>,
    data_sender: &Sender<(NodeId, SocketAddr, Option<Vec<u8>>)>,
) -> Result<(), String> {
    let (pong_msg_entries, _) =
        set_gossip_data(&node_data_lock, &known_nodes_lock, GOSSIP_SECTION_ENTRIES).unwrap();
    let msg = create_gossip_msg(NO_PING_ID, pong_id, node_data_lock, pong_msg_entries);

    if let Err(_) = data_sender.send((dst_id, dst_addr, Some(msg.serialize()))) {
        return Err("Error when sending pong message to node_output".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cluster::comms::gossip_message::GossipEntry;
    use crate::cluster::state::flags::CONNECTED;
    use crate::cluster::state::node_data::NodeData;
    use crate::config::node_configs::NodeConfigs;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_pfail_cleanup_bug_fix() {
        // Crear configuración para el nodo A desde un archivo temporal
        let config = NodeConfigs::new("tests/utils/g_r_test.conf").unwrap();

        // Crear nodo A
        let node_data = Arc::new(RwLock::new(NodeData::new(config)));

        let known_nodes = Arc::new(RwLock::new(HashMap::new()));

        // Agregar nodo B a la lista de nodos conocidos del nodo A
        {
            let mut nodes = known_nodes.write().unwrap();
            let node_b = KnownNode::new("node_b".to_string(), "0.0.0.0".to_string(), 7002);
            nodes.insert("node_b".to_string(), node_b);
        }

        // Simular que el nodo A marca al nodo B como PFAIL
        {
            let mut nodes = known_nodes.write().unwrap();
            if let Some(node_a) = nodes.get_mut("node_a") {
                node_a.recognize_as_pfail("node_b".to_string());
            }
        }

        // Verificar que el nodo A reconoce al nodo B como PFAIL
        {
            let nodes = known_nodes.read().unwrap();
            if let Some(node_a) = nodes.get("node_a") {
                assert!(node_a.recognized_as_pfail(&"node_b".to_string()));
            }
        }

        // Crear un mensaje de gossip donde el nodo B aparece como CONNECTED (vivo)
        let mut flags = crate::cluster::state::flags::NodeFlags::new();
        flags.set(CONNECTED);

        let gossip_entry = GossipEntry::new(
            "node_b".to_string(),
            "0.0.0.0".to_string(),
            7002,
            (0, 100),
            1,
            flags,
            1234567890,
            None,
            -1,
            false,
        );

        // Procesar el mensaje de gossip
        process_gossip_entries(
            &known_nodes,
            &node_data,
            vec![gossip_entry],
            "node_c".to_string(), // sender_id (no importa para este test)
        );

        // Verificar que el nodo A ya NO reconoce al nodo B como PFAIL
        {
            let nodes = known_nodes.read().unwrap();
            if let Some(node_a) = nodes.get("node_a") {
                assert!(
                    !node_a.recognized_as_pfail(&"node_b".to_string()),
                    "El nodo A debería haber limpiado el flag PFAIL del nodo B"
                );
            }
        }
    }
}
