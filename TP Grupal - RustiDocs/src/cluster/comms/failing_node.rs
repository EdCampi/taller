use crate::cluster::cluster_node::NODE_TIMEOUT;
use crate::cluster::comms::gossip_message::GossipEntry;
use crate::cluster::comms::gossip_receiver::process_gossip_entries;
use crate::cluster::comms::gossip_sender::set_gossip_data;
use crate::cluster::comms::replica_promotion::start_promotion;
use crate::cluster::state::flags::PFAIL;
use crate::cluster::state::node_data::NodeData;
use crate::cluster::types::{Epoch, FAIL_TYPE, KnownNode, NodeId, NodeMessage};
use crate::cluster::utils::{read_string_from_buffer, read_u16_from_buffer, read_u64_from_buffer};
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

static FAIL_REPORT_VALIDITY_MULT: u64 = 2;

pub fn start_pfail_procedure(
    pfail_id: NodeId,
    sender_data_lock: Arc<RwLock<NodeData>>,
    known_nodes_lock: Arc<RwLock<HashMap<NodeId, KnownNode>>>,
    broadcast_sender: Sender<Vec<u8>>,
) {
    let mut known_nodes = known_nodes_lock.write().unwrap();
    let failing_node = known_nodes.get_mut(&pfail_id).unwrap();
    if failing_node.is_fail() || failing_node.is_pfail() {
        return;
    }
    failing_node.get_flags_mut().set(PFAIL);
    drop(known_nodes);

    println!(
        "[PFAIL_PROCEDURE] Iniciando procedimiento PFAIL para nodo: {}",
        pfail_id
    );
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(
            NODE_TIMEOUT * FAIL_REPORT_VALIDITY_MULT,
        ));
        println!(
            "[PFAIL_PROCEDURE] Verificando mayoría para nodo: {}",
            pfail_id
        );
        let known_nodes = known_nodes_lock.read().unwrap();
        let mut masters = 0;
        let mut recognizes_pfail = 1; // Me cuento a mi mismo

        for node in known_nodes.iter() {
            if node.1.is_master() {
                masters += 1;
                if node.1.recognized_as_pfail(&pfail_id) {
                    recognizes_pfail += 1;
                    println!(
                        "[VOTING] Master {} reconoce como PFAIL a {}",
                        node.0, pfail_id
                    );
                } else {
                    println!(
                        "[VOTING] Master {} NO reconoce como PFAIL a {}",
                        node.0, pfail_id
                    );
                }
            }
        }
        drop(known_nodes);

        println!(
            "[VOTING] Total masters: {}, Reconocen PFAIL: {}",
            masters, recognizes_pfail
        );
        println!("[VOTING] Mayoría requerida: > {}", masters / 2);

        if recognizes_pfail > masters / 2 {
            println!("[FAIL] Mayoría alcanzada! Marcando {} como FAIL", pfail_id);
            start_fail_procedure(
                pfail_id,
                sender_data_lock,
                known_nodes_lock,
                broadcast_sender,
            );
        } else {
            println!(
                "[FAIL] Mayoría NO alcanzada. {} permanece como PFAIL",
                pfail_id
            );
        }
    });
}

fn start_fail_procedure(
    pfail_id: NodeId,
    sender_data_lock: Arc<RwLock<NodeData>>,
    known_nodes_lock: Arc<RwLock<HashMap<NodeId, KnownNode>>>,
    broadcast_sender: Sender<Vec<u8>>,
) {
    println!(
        "💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀 NODO {} DECLARADO COMO FAIL DEFINITIVO 💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀💀",
        pfail_id
    );
    println!("[FAIL] Node {} is declared as FAIL", pfail_id);
    let mut known_nodes = known_nodes_lock.write().unwrap();
    let failing_node = known_nodes.get_mut(&pfail_id).unwrap();
    println!(
        "[FAIL] Cambiando flags de PFAIL a FAIL para nodo: {}",
        pfail_id
    );
    failing_node.set_fail();
    println!(
        "[FAIL] Flags actualizados para nodo {}: {:?}",
        pfail_id,
        failing_node.get_flags()
    );
    drop(known_nodes);

    let fail_message = FailMessage::new(pfail_id.clone(), &sender_data_lock, &known_nodes_lock);
    let bytes = fail_message.serialize();
    let sender_data = sender_data_lock.read().unwrap();
    let broadcast_message = NodeMessage::new(
        sender_data.get_id(),
        sender_data.get_ip(),
        sender_data.get_port(),
        FAIL_TYPE,
        bytes.len() as u16,
        bytes,
    );
    drop(sender_data);

    println!(
        "[FAIL] Enviando mensaje de FAIL broadcast para nodo: {}",
        pfail_id
    );
    if let Err(_) = broadcast_sender.send(broadcast_message.serialize()) {
        println!("[NODE] Error when sending the broadcast message");
    } else {
        println!("[FAIL] Mensaje de FAIL broadcast enviado exitosamente");
    }

    // Iniciar proceso de promoción de réplicas
    println!(
        "[FAIL] Iniciando proceso de promoción de réplicas para nodo: {}",
        pfail_id
    );
    start_promotion(
        pfail_id,
        sender_data_lock,
        known_nodes_lock,
        broadcast_sender,
    );
}

pub fn process_node_fail_msg(
    message: NodeMessage,
    node_data_lock: &Arc<RwLock<NodeData>>,
    known_nodes_lock: &Arc<RwLock<HashMap<NodeId, KnownNode>>>,
) -> Result<(), String> {
    let fail_msg = FailMessage::from_bytes(&message.get_payload())?;
    let failing_id = fail_msg.get_failing_id();
    let sender_id = fail_msg.get_sender_id();

    println!(
        "📢📢📢📢📢📢📢📢📢📢📢📢📢📢📢📢📢📢📢📢📢 RECIBIDO MENSAJE DE FAIL - NODO {} FALLÓ 📢📢📢📢📢📢📢📢📢📢📢📢📢📢📢📢📢📢📢📢📢📢📢📢",
        failing_id
    );
    println!(
        "\x1b[31m[[FAIL_MSG] Recibido mensaje de FAIL de {} para nodo: {}\x1b[0m",
        sender_id, failing_id
    );

    if failing_id == node_data_lock.read().unwrap().get_id() {
        println!("[CLUSTER] Error - Node isn't failing");
        return Ok(()); // Ignoro si me llego fail a mi mismo
    }

    let mut known_nodes = known_nodes_lock.write().unwrap();
    let failing_node = known_nodes.get_mut(&failing_id).unwrap();
    println!(
        "[FAIL_MSG] Marcando nodo {} como FAIL (recibido de {})",
        failing_id, sender_id
    );
    failing_node.set_fail();
    println!(
        "[FAIL_MSG] Flags actualizados para nodo {}: {:?}",
        failing_id,
        failing_node.get_flags()
    );
    drop(known_nodes);

    process_gossip_entries(
        &known_nodes_lock,
        &node_data_lock,
        fail_msg.get_gossip_entries(),
        fail_msg.get_sender_id(),
    );
    println!(
        "\x1b[31m[[FAIL_MSG] Procesamiento de mensaje FAIL completado para nodo: {}\x1b[0m",
        failing_id
    );
    Ok(())
}

pub struct FailMessage {
    sender_id: NodeId,
    sender_ip: String,
    sender_port: u16,
    sender_config_epoch: Epoch,
    failing_id: NodeId,
    gossip_entries: Vec<GossipEntry>,
}

impl FailMessage {
    pub fn new(
        failing_id: NodeId,
        sender_data: &Arc<RwLock<NodeData>>,
        known_nodes_lock: &Arc<RwLock<HashMap<NodeId, KnownNode>>>,
    ) -> Self {
        let node_data = sender_data.read().unwrap();
        let sender_id = node_data.get_id();
        let sender_addr = node_data.get_addr();
        let sender_ip = sender_addr.ip().to_string();
        let sender_port = sender_addr.port();
        let sender_config_epoch = node_data.get_cepoch();
        drop(node_data);

        let gossip_entries =
            if let Some((gossip_entries, _)) = set_gossip_data(&sender_data, known_nodes_lock, 3) {
                gossip_entries
            } else {
                vec![]
            };
        FailMessage {
            sender_id,
            sender_ip,
            sender_port,
            sender_config_epoch,
            failing_id,
            gossip_entries,
        }
    }

    pub fn get_sender_id(&self) -> NodeId {
        self.sender_id.clone()
    }

    pub fn get_failing_id(&self) -> NodeId {
        self.failing_id.clone()
    }

    pub fn get_gossip_entries(&self) -> Vec<GossipEntry> {
        self.gossip_entries.clone()
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = vec![];

        let sender_id_bytes = self.sender_id.as_bytes();
        let sender_id_len = sender_id_bytes.len() as u16;
        buffer.extend_from_slice(&sender_id_len.to_be_bytes());
        buffer.extend_from_slice(sender_id_bytes);

        let sender_ip_bytes = self.sender_ip.as_bytes();
        let sender_ip_len = sender_ip_bytes.len() as u16;
        buffer.extend_from_slice(&sender_ip_len.to_be_bytes());
        buffer.extend_from_slice(sender_ip_bytes);

        buffer.extend_from_slice(&self.sender_port.to_be_bytes());

        buffer.extend_from_slice(&self.sender_config_epoch.to_be_bytes());

        let pfail_id_bytes = self.failing_id.as_bytes();
        let pfail_id_len = pfail_id_bytes.len() as u16;
        buffer.extend_from_slice(&pfail_id_len.to_be_bytes());
        buffer.extend_from_slice(pfail_id_bytes);

        buffer.extend(GossipEntry::serialize_vector(&self.gossip_entries));

        buffer
    }

    pub fn from_bytes(mut data: &[u8]) -> Result<Self, String> {
        let sender_id_len = read_u16_from_buffer(&mut data)?;
        let sender_id = read_string_from_buffer(&mut data, sender_id_len as usize)?;

        let sender_ip_len = read_u16_from_buffer(&mut data)?;
        let sender_ip = read_string_from_buffer(&mut data, sender_ip_len as usize)?;

        let sender_port = read_u16_from_buffer(&mut data)?;

        let sender_config_epoch = read_u64_from_buffer(&mut data)?;

        let pfail_id_len = read_u16_from_buffer(&mut data)?;
        let failing_id = read_string_from_buffer(&mut data, pfail_id_len as usize)?;

        let entries_len = read_u16_from_buffer(&mut data)?;
        let mut gossip_entries = Vec::with_capacity(entries_len as usize);
        for _ in 0..entries_len {
            if let Ok(entry) = GossipEntry::from_bytes(&mut data) {
                gossip_entries.push(entry);
            }
        }

        Ok(FailMessage {
            sender_id,
            sender_ip,
            sender_port,
            sender_config_epoch,
            failing_id,
            gossip_entries,
        })
    }
}
