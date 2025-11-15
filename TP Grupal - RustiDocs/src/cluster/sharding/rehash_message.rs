use crate::cluster::comms::gossip_message::{GossipMessage, NO_PING_ID, NO_PONG_ID};
use crate::cluster::state::flags::SLAVE;
use crate::cluster::state::node_data::NodeData;
use crate::cluster::types::{GOSSIP_TYPE, KnownNode, NodeId, NodeMessage, SlotRange};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock, RwLockWriteGuard};

pub fn process_rehash_msg(
    message: NodeMessage,
    node_data_lock: &Arc<RwLock<NodeData>>,
    known_nodes_lock: &Arc<RwLock<HashMap<NodeId, KnownNode>>>,
    output_sender: &Sender<(NodeId, SocketAddr, Option<Vec<u8>>)>,
) -> Result<(), String> {
    let rehash_msg = RehashMessage::from_bytes(&message.get_payload())
        .map_err(|_| "Error when processing the rehash message".to_string())?;

    let mut node_data = node_data_lock.write().unwrap();
    if rehash_msg.get_id() == node_data.get_id() {
        node_data.set_slots(rehash_msg.get_slots());
        println!(
            "\x1b[34m[CLUSTER] Slots actualizados del nodo {}: {:?}\x1b[0m",
            rehash_msg.get_id(),
            node_data.get_slots()
        );
        // Mando un clon para no tener el write abierto más de lo que debería.
        node_data.add_cepoch();
        if rehash_msg.get_role() == SLAVE {
            node_data.set_as_slave(rehash_msg.get_master_id());
        } else {
            node_data.set_as_master();
        }

        // Si está en handshake, mando primer gossip.
        let mut known_nodes = known_nodes_lock.write().unwrap();
        if !known_nodes.contains_key(&node_data.get_id()) {
            let src_addr = message.get_addr();
            let mut aux = KnownNode::new(
                message.get_src_id(),
                message.get_addr().ip().to_string(),
                message.get_addr().port(),
            );
            aux.set_handshake();
            known_nodes.insert(message.get_src_id(), aux);
            return send_first_gossip(rehash_msg.get_id(), src_addr, node_data, output_sender);
        }
    }
    println!(
        "RehashMessage wasn't meant for this node ({})",
        node_data.get_id()
    );
    Err("Wrong rehash msg".to_string())
}

fn send_first_gossip(
    src_id: NodeId,
    addr: SocketAddr,
    node_data: RwLockWriteGuard<NodeData>,
    node_output: &Sender<(NodeId, SocketAddr, Option<Vec<u8>>)>,
) -> Result<(), String> {
    let entry = node_data.get_own_gossip_entry();

    let gossip_msg = GossipMessage::new(NO_PING_ID, NO_PONG_ID, node_data.get_flags(), vec![entry]);
    let payload = gossip_msg.serialize();
    let message = NodeMessage::new(
        node_data.get_id(),
        node_data.get_ip(),
        node_data.get_port(),
        GOSSIP_TYPE,
        payload.len() as u16,
        payload,
    );
    if let Err(_) = node_output.send((src_id, addr, Some(message.serialize()))) {
        println!("Error when sending first gossip to node_output");
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct RehashMessage {
    dst_id: NodeId,
    role: u8,
    start_slot: u16,
    end_slot: u16,
    master_id: NodeId,
}

impl RehashMessage {
    pub fn new(
        dst_id: NodeId,
        role: u8,
        start_slot: u16,
        end_slot: u16,
        master_id: NodeId,
    ) -> Self {
        Self {
            dst_id,
            role,
            start_slot,
            end_slot,
            master_id,
        }
    }

    pub fn get_id(&self) -> NodeId {
        self.dst_id.clone()
    }

    pub fn get_role(&self) -> u8 {
        self.role
    }

    pub fn get_slots(&self) -> SlotRange {
        (self.start_slot, self.end_slot)
    }

    pub fn get_master_id(&self) -> NodeId {
        self.master_id.clone()
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        let id_bytes = self.dst_id.as_bytes();
        let id_len = id_bytes.len() as u16;
        result.extend_from_slice(&id_len.to_be_bytes());
        result.extend_from_slice(id_bytes);

        result.extend_from_slice(&self.role.to_be_bytes());
        result.extend_from_slice(&self.start_slot.to_be_bytes());
        result.extend_from_slice(&self.end_slot.to_be_bytes());

        let m_id_bytes = self.master_id.as_bytes();
        let m_id_len = m_id_bytes.len() as u16;
        result.extend_from_slice(&m_id_len.to_be_bytes());
        result.extend_from_slice(m_id_bytes);

        result
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        let mut offset = 0;

        if data.len() < offset + 2 {
            return Err("RehashMessage: data too short for id length".to_string());
        }
        let id_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;
        if data.len() < offset + id_len {
            return Err("RehashMessage: data too short for id".to_string());
        }
        let dst_id =
            String::from_utf8(data[offset..offset + id_len].to_vec()).map_err(|e| e.to_string())?;
        offset += id_len;

        if data.len() < offset + 1 {
            return Err("RehashMessage: data too short for role".to_string());
        }
        let role = data[offset];
        offset += 1;

        if data.len() < offset + 2 {
            return Err("RehashMessage: data too short for start_slot".to_string());
        }
        let start_slot = u16::from_be_bytes([data[offset], data[offset + 1]]);
        offset += 2;

        if data.len() < offset + 2 {
            return Err("RehashMessage: data too short for end_slot".to_string());
        }
        let end_slot = u16::from_be_bytes([data[offset], data[offset + 1]]);
        offset += 2;

        if data.len() < offset + 2 {
            return Err("RehashMessage: data too short for master_id length".to_string());
        }
        let m_id_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;
        if data.len() < offset + m_id_len {
            return Err("RehashMessage: data too short for master_id".to_string());
        }
        let master_id = String::from_utf8(data[offset..offset + m_id_len].to_vec())
            .map_err(|e| e.to_string())?;

        Ok(Self {
            dst_id,
            role,
            start_slot,
            end_slot,
            master_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cluster::state::flags::MASTER;

    #[test]
    fn test_serialize_and_from_bytes() {
        let msg = RehashMessage::new("node123".to_string(), MASTER, 10, 20, "".to_string());
        let bytes = msg.serialize();
        let parsed = RehashMessage::from_bytes(&bytes).expect("Failed to parse bytes");
        assert_eq!(msg.get_id(), parsed.get_id());
        assert_eq!(msg.get_slots(), parsed.get_slots());
    }
}
