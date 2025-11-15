use crate::cluster::state::flags::{CONNECTED, FAIL, HANDSHAKE, MASTER, NOADDR, PFAIL, SLAVE};
use crate::cluster::utils::{
    read_payload_from_buffer, read_string_from_buffer, read_u8_from_buffer, read_u16_from_buffer,
    system_time_to_i64,
};
use crate::cluster::{comms::gossip_message::GossipEntry, state::flags::NodeFlags};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

pub type NodeId = String;
pub type NodeIp = String;
pub type SlotRange = (u16, u16);
pub type TimeStamp = i64;
pub type Epoch = u64;
pub const GOSSIP_TYPE: u8 = 0; // Tipo de mensaje para gossip
pub const JOIN_TYPE: u8 = 1;
pub const REHASH_TYPE: u8 = 2;
pub const FAIL_TYPE: u8 = 3;
pub const PUBSUB_TYPE: u8 = 4;
pub const PROMOTION_TYPE: u8 = 5; // Tipo de mensaje para promoción de réplicas
pub const REQUEST_PSYNC_TYPE: u8 = 6; // Tipo de mensaje para solicitud de PSYNC
pub const NEW_MASTER_TYPE: u8 = 7;
pub const CONNECTION_CLOSE_TYPE: u8 = 0xFF;
pub const MESSAGE_DELIMITER: &[u8; 5] = b"<END>";
pub const DEFAULT_BUFFER_SIZE: usize = 8192;

pub struct NodeMessage {
    src_id: NodeId,
    src_ip: NodeIp,
    src_port: u16,
    request_type: u8,
    payload_len: u16, // Opcional, para enviar datos adicionales
    payload: Vec<u8>, // Opcional, para enviar datos adicionales
}

impl NodeMessage {
    pub fn new(
        src_id: NodeId,
        src_ip: NodeIp,
        src_port: u16,
        request_type: u8,
        payload_len: u16,
        payload: Vec<u8>,
    ) -> Self {
        NodeMessage {
            src_id,
            src_ip,
            src_port,
            request_type,
            payload_len,
            payload,
        }
    }

    pub fn get_src_id(&self) -> NodeId {
        self.src_id.to_string()
    }

    pub fn get_request_type(&self) -> u8 {
        self.request_type
    }

    pub fn get_addr(&self) -> SocketAddr {
        format!("{}:{}", self.src_ip, self.src_port)
            .parse()
            .unwrap()
    }

    pub fn get_payload(&self) -> Vec<u8> {
        self.payload.clone()
    }

    pub fn create_close_connection_msg() -> Self {
        NodeMessage::new(
            String::new(),
            String::new(),
            0,
            CONNECTION_CLOSE_TYPE,
            0,
            Vec::new(),
        )
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut res = Vec::new();

        let id_bytes = self.src_id.as_bytes();
        let id_bytes_len = id_bytes.len() as u16;
        res.extend_from_slice(&id_bytes_len.to_be_bytes());
        res.extend_from_slice(id_bytes);

        let ip_bytes = self.src_ip.as_bytes();
        let ip_bytes_len = ip_bytes.len() as u16;
        res.extend_from_slice(&ip_bytes_len.to_be_bytes());
        res.extend_from_slice(ip_bytes);

        res.extend_from_slice(&self.src_port.to_be_bytes());
        res.push(self.request_type);
        res.extend_from_slice(&self.payload_len.to_be_bytes());
        res.extend_from_slice(&self.payload);
        res.extend_from_slice(MESSAGE_DELIMITER);
        res
    }

    pub fn from_bytes<R: Read>(reader: &mut R) -> Result<Self, String> {
        let node_id_len = read_u16_from_buffer(reader)?;
        let src_id = read_string_from_buffer(reader, node_id_len as usize)?;

        let node_ip_len = read_u16_from_buffer(reader)?;
        let src_ip = read_string_from_buffer(reader, node_ip_len as usize)?;

        let src_port = read_u16_from_buffer(reader)?;

        let request_type = read_u8_from_buffer(reader)?;

        let payload_len = read_u16_from_buffer(reader)?;

        let payload = read_payload_from_buffer(reader, payload_len as usize)?;

        Ok(NodeMessage {
            src_id,
            src_ip,
            src_port,
            request_type,
            payload_len,
            payload,
        })
    }
}

#[derive(Clone, Debug)]
pub struct KnownNode {
    node_id: NodeId,
    node_ip: String,
    node_port: u16,
    slots: SlotRange,
    config_epoch: Epoch,
    flags: NodeFlags,
    last_ping_sent: TimeStamp,
    last_pong_received: TimeStamp,
    last_ds_updated: TimeStamp,
    master_id: Option<NodeId>,
    replicas_ids: Vec<NodeId>,
    pfail_ids: HashSet<NodeId>,
    replaced: bool,
}

impl KnownNode {
    pub fn new(node_id: String, node_ip: String, node_port: u16) -> Self {
        KnownNode {
            node_id,
            node_ip,
            node_port,
            slots: (0u16, 0u16),
            config_epoch: 0,
            flags: NodeFlags::new(),
            last_ping_sent: -1,
            last_pong_received: -1,
            last_ds_updated: -1,
            master_id: None,
            replicas_ids: vec![],
            pfail_ids: HashSet::new(),
            replaced: false,
        }
    }

    pub fn new_from_entry(entry: &GossipEntry) -> Self {
        let mut flags = NodeFlags::new();
        flags.clear();
        flags.set(entry.get_state());
        flags.set(NOADDR);
        KnownNode {
            node_id: entry.get_id(),
            node_ip: entry.get_ip(),
            node_port: entry.get_port(),
            slots: entry.get_slots(),
            config_epoch: entry.get_config_epoch(),
            flags,
            last_ping_sent: -1,
            last_pong_received: entry.get_last_pong_time(),
            last_ds_updated: entry.get_last_update_time(),
            master_id: entry.get_master_id(),
            replicas_ids: vec![], // TODO: entry.get_replicas(),
            pfail_ids: HashSet::new(),
            replaced: entry.was_replaced(),
        }
    }

    pub fn owns_slot(&self, slot: u16) -> bool {
        if self.slots.0 < slot && self.slots.1 > slot {
            return true;
        };
        false
    }

    pub fn update(&mut self, latest: GossipEntry) {
        if self.config_epoch > latest.get_config_epoch()
            || self.last_pong_received > latest.get_last_pong_time()
            || (self.flags.is_set(FAIL) && !latest.was_replaced())
        // Un nodo FAIL solo se puede recuperar a manualmente
        {
            return;
        }
        self.force_update(latest);
    }

    pub fn force_update(&mut self, latest: GossipEntry) {
        if self.config_epoch > latest.get_config_epoch() {
            return;
        }
        self.slots = latest.get_slots();
        self.flags.set(latest.get_state());
        self.config_epoch = latest.get_config_epoch();
        self.last_pong_received = latest.get_last_pong_time();
        self.master_id = latest.get_master_id();
        self.last_ds_updated = latest.get_last_update_time();
        self.replaced = latest.was_replaced();
    }

    pub fn get_gossip_entry(&self) -> GossipEntry {
        let res = GossipEntry::new(
            self.node_id.clone(),
            self.node_ip.clone(),
            self.node_port,
            self.slots.clone(),
            self.config_epoch,
            self.flags.clone(),
            self.last_pong_received,
            self.master_id.clone(),
            self.last_ds_updated,
            self.replaced,
        );
        res
    }

    pub fn get_id(&self) -> NodeId {
        self.node_id.clone()
    }

    pub fn get_slots(&self) -> SlotRange {
        self.slots.clone()
    }

    pub fn get_slots_len(&self) -> u16 {
        self.slots.1 - self.slots.0
    }

    pub fn contains(&self, slot: &u16) -> bool {
        if self.slots.0 < *slot && self.slots.1 > *slot {
            return true;
        };
        false
    }

    pub fn get_addr(&self) -> SocketAddr {
        let aux = format!("{}:{}", self.node_ip, self.node_port);
        aux.parse().unwrap()
    }

    pub fn set_hash_slots(&mut self, slots: SlotRange) {
        self.slots = slots;
    }

    pub fn get_flags(&self) -> &NodeFlags {
        &self.flags
    }

    pub fn get_state(&self) -> u8 {
        self.flags.get_state()
    }

    pub fn flags_detail(&self) -> String {
        self.flags.print()
    }

    pub fn get_flags_mut(&mut self) -> &mut NodeFlags {
        &mut self.flags
    }

    pub fn is_master(&self) -> bool {
        self.flags.is_set(MASTER)
    }

    pub fn is_slave(&self) -> bool {
        self.flags.is_set(SLAVE)
    }

    pub fn get_master_id(&self) -> Option<&NodeId> {
        self.master_id.as_ref()
    }

    /// Promueve una réplica a master
    pub fn promote_to_master(&mut self, slots: SlotRange, config_epoch: Epoch) {
        // Cambiar flags de SLAVE a MASTER
        self.flags.unset(SLAVE);
        self.flags.set(MASTER);

        // Asignar slots
        let min_slot = slots.0;
        let max_slot = slots.1;
        self.slots = (min_slot, max_slot);

        // Actualizar epoch de configuración
        self.config_epoch = config_epoch;

        // Limpiar referencia al master anterior
        self.master_id = None;
    }

    /// Limpia los slots asignados (para nodos fallidos)
    pub fn clear_slots(&mut self) {
        self.slots = (0, 0);
    }

    /// Obtiene el offset de replicación (simulado)
    /// En una implementación real, esto vendría del estado de replicación
    pub fn get_replication_offset(&self) -> u64 {
        // Simulamos que las réplicas más recientes tienen mayor offset
        self.last_pong_received as u64
    }

    pub fn set_pfail(&mut self) {
        self.flags.set(PFAIL);
    }

    pub fn is_pfail(&self) -> bool {
        NodeFlags::state_contains(self.get_state(), PFAIL)
    }

    pub fn set_fail(&mut self) {
        self.flags.set(FAIL);
    }

    pub fn is_fail(&self) -> bool {
        NodeFlags::state_contains(self.get_state(), FAIL)
    }

    pub fn disconnect(&mut self) {
        self.flags.unset(CONNECTED);
    }

    pub fn set_handshake(&mut self) {
        self.flags.set(HANDSHAKE);
    }

    pub fn set_connected(&mut self) {
        self.flags.set(CONNECTED);
    }

    pub fn set_master(&mut self, master_id: Option<NodeId>) {
        self.master_id = master_id;
    }

    pub fn add_replica(&mut self, replica_id: NodeId) {
        self.replicas_ids.push(replica_id);
    }

    pub fn get_replicas(&self) -> &Vec<NodeId> {
        &self.replicas_ids
    }

    pub fn replica_count(&self) -> usize {
        self.replicas_ids.len()
    }

    pub fn set_last_ping_time(&mut self) {
        self.last_ping_sent = system_time_to_i64(SystemTime::now());
    }

    pub fn set_last_pong_time(&mut self, time: Option<TimeStamp>) {
        if let Some(last_ping_time) = time {
            self.last_pong_received = last_ping_time;
        } else {
            self.last_pong_received = system_time_to_i64(SystemTime::now());
        }
    }

    pub fn recognize_as_pfail(&mut self, node_id: NodeId) {
        self.pfail_ids.insert(node_id);
    }

    pub fn recognize_as_alive(&mut self, node_id: &NodeId) {
        self.pfail_ids.remove(node_id);
    }

    pub fn recognized_as_pfail(&self, node_id: &NodeId) -> bool {
        self.pfail_ids.contains(node_id)
    }

    pub fn get_pfails(&self) -> HashSet<NodeId> {
        self.pfail_ids.clone()
    }

    pub fn has_addr(&mut self) -> bool {
        !self.flags.is_set(NOADDR)
    }

    pub fn addr_is_set(&mut self) {
        self.flags.unset(NOADDR)
    }

    pub fn get_last_update_time(&self) -> TimeStamp {
        self.last_ds_updated
    }

    pub fn set_as_replaced(&mut self) {
        self.replaced = true;
    }

    pub fn is_replaced(&self) -> bool {
        self.replaced
    }

    pub fn add_cepoch(&mut self) {
        self.config_epoch += 1;
    }
}

pub fn get_node_ip_for_slot(
    slot: u16,
    known_nodes: &Arc<RwLock<HashMap<NodeId, KnownNode>>>,
) -> Option<SocketAddr> {
    let known_nodes_aux = known_nodes.read().unwrap();
    for (_node_id, neighbor) in known_nodes_aux.iter() {
        println!("[NODES] Conocido {:?}, slots {:?}", neighbor, slot);
        if neighbor.contains(&slot) {
            // Assuming KnownNode has `ip` and `port` fields similar to NodeSettings
            let addr_str = format!("{}:{}", neighbor.node_ip, neighbor.node_port);
            if let Ok(addr) = addr_str.parse() {
                return Some(addr);
            }
        }
    }
    None
}
