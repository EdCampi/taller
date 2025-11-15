//! Módulo encargado de mantener las propiedades del nodo respecto de
//! su ciclo de vida dentro del cluster.
//!
//! Diferencias con `NodeConfig`:
//! * Cambia dinámicamente;
//! * No posee información sobre las configuraciones locales.

use crate::cluster::comms::gossip_message::GossipEntry;
use crate::cluster::state::flags::*;
use crate::cluster::types::SlotRange;
use crate::cluster::types::{Epoch, NodeIp};
use crate::cluster::types::{NodeId, TimeStamp};
use crate::cluster::utils::system_time_to_i64;
use crate::config::node_configs::NodeConfigs;
use std::net::SocketAddr;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct NodeData {
    node_id: NodeId,
    port: u16,
    addr: SocketAddr,
    slot_range: SlotRange,
    current_epoch: Epoch,
    config_epoch: Epoch,
    node_flags: NodeFlags,
    master_id: Option<NodeId>,
    last_update_time: TimeStamp,
}

impl NodeData {
    pub fn new(configs: NodeConfigs) -> Self {
        let addr = configs.get_addr();
        let port = configs.get_node_port();
        let node_id = configs.get_id();
        let mut node_flags = NodeFlags::new();
        node_flags.set(ME);
        node_flags.set(CONNECTED);
        NodeData {
            node_id,
            port,
            addr,
            slot_range: configs.get_hash_slots(), // Rango vacío
            current_epoch: 0,
            config_epoch: 0,
            node_flags,
            master_id: None,
            last_update_time: -1,
        }
    }

    /// Define el nodo como replica, y asigna a su master.
    pub fn set_as_slave(&mut self, master_id: NodeId) {
        self.node_flags.set(SLAVE); // Marca este nodo como replica (slave)
        self.node_flags.unset(MASTER);
        self.master_id = Some(master_id);
    }

    // Marca este nodo como master
    pub fn set_as_master(&mut self) {
        self.node_flags.set(MASTER); // Marca este nodo como master
        self.master_id = None;
    }

    pub fn get_role(&self) -> u8 {
        if self.node_flags.is_set(MASTER) {
            return 0;
        };
        1
    }

    pub fn get_state(&self) -> u8 {
        self.node_flags.get_state()
    }
    pub fn get_id(&self) -> NodeId {
        self.node_id.to_string()
    }

    pub fn get_slots(&self) -> SlotRange {
        self.slot_range.clone()
    }

    pub fn get_slots_len(&self) -> u16 {
        self.slot_range.1 - self.slot_range.0
    }

    pub fn set_slots(&mut self, range: SlotRange) {
        self.slot_range = range;
    }

    //Esto ocurre cuando se crea un nuev nodo
    pub fn set_epoch(&mut self) {
        self.current_epoch = 0;
    }

    pub fn get_cepoch(&self) -> Epoch {
        self.config_epoch
    }

    pub fn add_cepoch(&mut self) {
        self.config_epoch += 1;
    }

    pub fn get_addr(&self) -> SocketAddr {
        self.addr.clone()
    }

    pub fn get_ip(&self) -> NodeIp {
        self.addr.ip().to_string()
    }

    pub fn get_port(&self) -> u16 {
        self.port
    }

    pub fn get_flags(&self) -> NodeFlags {
        self.node_flags.clone()
    }

    pub fn get_own_gossip_entry(&self) -> GossipEntry {
        GossipEntry::new(
            self.node_id.clone(),
            self.addr.ip().to_string(),
            self.port,
            self.slot_range,
            self.config_epoch,
            self.node_flags.clone(),
            system_time_to_i64(SystemTime::now()), // Como lo voy a usar en el gossip, la información de mi mismo está actulizada.
            self.master_id.clone(),
            self.last_update_time,
            false, // Si estoy mandando mensajes es porque no fallé
        )
    }

    pub fn owns_slot(&self, slot: u16) -> bool {
        if self.slot_range.0 < slot && self.slot_range.1 > slot {
            return true;
        }
        false
    }

    pub fn get_master_id(&self) -> Option<NodeId> {
        self.master_id.clone()
    }

    pub fn set_last_update_time(&mut self, time: TimeStamp) {
        self.last_update_time = time;
    }
}
