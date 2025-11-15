//! Módulo encargado de la preparación de los mensajes tipo gossip.

// IMPORTS
use crate::cluster::comms::failing_node::start_pfail_procedure;
use crate::cluster::time_tracker::TimeTracker;
use crate::cluster::types::GOSSIP_TYPE;
use crate::cluster::utils::print_slots;
use crate::cluster::{
    comms::{
        gossip_message::{GossipEntry, GossipMessage},
        node_output::NodeOutput,
    },
    state::node_data::NodeData,
    types::{KnownNode, NodeId, NodeMessage},
};
use std::sync::RwLockReadGuard;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

pub struct GossipSender {
    node_output: Arc<RwLock<NodeOutput>>,
    tracker: Arc<RwLock<TimeTracker>>,
    ping_id: u64,
}

impl GossipSender {
    pub fn new(node_output: Arc<RwLock<NodeOutput>>, tracker: Arc<RwLock<TimeTracker>>) -> Self {
        GossipSender {
            node_output,
            tracker,
            ping_id: 0,
        }
    }

    pub fn ping(
        &mut self,
        node_data: Arc<RwLock<NodeData>>,
        known_nodes: Arc<RwLock<HashMap<NodeId, KnownNode>>>,
        k: u64,
        t: u64,
    ) {
        loop {
            self._ping(node_data.clone(), known_nodes.clone(), k);
            thread::sleep(Duration::from_millis(t));
        }
    }

    fn _ping(
        &mut self,
        node_data: Arc<RwLock<NodeData>>,
        known_nodes: Arc<RwLock<HashMap<NodeId, KnownNode>>>,
        k: u64,
    ) {
        let mut tracker = self.tracker.write().unwrap();
        if let Some(node_id) = tracker.verify_timeout() {
            println!(
                "🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨 TIMEOUT DETECTADO - NODO {} NO RESPONDE 🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨🚨",
                node_id
            );
            println!(
                "\x1b[33m[PFAIL_PROCEDURE] Iniciando procedimiento PFAIL para {}\x1b[0m",
                node_id
            );
            start_pfail_procedure(
                node_id,
                node_data.clone(),
                known_nodes.clone(),
                self.node_output.write().unwrap().set_broadcast_channel(),
            );
        }
        drop(tracker);

        if let Some(gossip_data) = set_gossip_data(&node_data, &known_nodes, k) {
            self.ping_id += 1;
            let message = create_gossip_msg(self.ping_id, 0, &node_data, gossip_data.0);

            let mut aux = known_nodes.write().unwrap();
            let dst_node = aux.get_mut(&gossip_data.1).unwrap();
            if !dst_node.has_addr() {
                self.node_output
                    .write()
                    .unwrap()
                    .open_connection_with(dst_node.get_id(), dst_node.get_addr());
                dst_node.addr_is_set();
            }

            self.node_output.write().unwrap().send_to_node(
                &gossip_data.1,
                message,
                Some(self.ping_id),
            );
        }
    }
}

pub fn set_gossip_data(
    node_data: &Arc<RwLock<NodeData>>,
    known_nodes: &Arc<RwLock<HashMap<NodeId, KnownNode>>>,
    k: u64,
) -> Option<(Vec<GossipEntry>, NodeId)> {
    // Nodo no manda gossip a B si B está en la lista de los gossippeados.
    let known_nodes_aux = known_nodes.read().unwrap();

    if known_nodes_aux.len() == 0 {
        return None;
    }
    let ids: Vec<_> = known_nodes_aux.keys().cloned().collect();
    let node_data_aux = node_data.read().unwrap();

    print_slots(&known_nodes_aux, &node_data_aux);

    // La selección del dst no tiene que estar sesgada, para poder enviarle al que creo fallado, por si revive
    let available_ids: Vec<_> = known_nodes_aux
        .iter()
        .filter(|(_, node)| !node.is_fail() && node_data_aux.get_id() != node.get_id()) // Solo ignoro los FAIL
        .map(|(id, _)| id.clone())
        .collect();
    if available_ids.len() == 0 {
        return None;
    }
    let dst = select_dst_node(&available_ids);
    let mut gossip_data = select_nodes_to_gossip(&known_nodes_aux, &ids, &dst, k);
    gossip_data.push(node_data_aux.get_own_gossip_entry());
    Some((gossip_data, dst))
}

fn select_nodes_to_gossip(
    known_nodes: &RwLockReadGuard<HashMap<NodeId, KnownNode>>,
    ids: &Vec<NodeId>,
    dst: &NodeId,
    k: u64,
) -> Vec<GossipEntry> {
    let mut gossip_res = vec![];
    let mut node_id_res = vec![];

    let mut i = 0;
    for _ in 0..k {
        if i == k {
            break;
        }
        let chosen = {
            let random = rand::random::<usize>() % ids.len();
            ids[random].clone()
        };
        let selected_node = known_nodes.get(&chosen).unwrap();
        if selected_node.get_id() == *dst {
            continue;
        }

        i += 1;
        let entry = selected_node.get_gossip_entry();
        gossip_res.push(entry);
        node_id_res.push(selected_node.get_id().clone());
    }
    gossip_res
}

pub fn create_gossip_msg(
    ping_id: u64,
    pong_id: u64,
    node_data_lock: &Arc<RwLock<NodeData>>,
    gossip_data: Vec<GossipEntry>,
) -> NodeMessage {
    let node_data = node_data_lock.read().unwrap();
    let aux = GossipMessage::new(ping_id, pong_id, node_data.get_flags(), gossip_data);
    let payload = aux.serialize();

    NodeMessage::new(
        node_data.get_id(),
        node_data.get_ip(),
        node_data.get_port(),
        GOSSIP_TYPE,
        payload.len() as u16,
        payload,
    )
}

fn select_dst_node(ids: &Vec<NodeId>) -> NodeId {
    let random = rand::random::<usize>() % ids.len();
    ids[random].clone()
}
