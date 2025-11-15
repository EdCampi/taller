use crate::cluster::state::flags::NodeFlags;
use crate::cluster::types::{Epoch, NodeId, SlotRange, TimeStamp};
use crate::cluster::utils::{
    read_string_from_buffer, read_timestamp_from_buffer, read_u8_from_buffer, read_u16_from_buffer,
    read_u64_from_buffer,
};
use std::io::{Cursor, Read};

pub static NO_PING_ID: u64 = 0;
pub static NO_PONG_ID: u64 = 0;

#[derive(Debug)]
pub struct GossipMessage {
    ping_id: u64,
    pong_id: u64,
    flags: NodeFlags,
    gossip_entries: Vec<GossipEntry>,
}

impl GossipMessage {
    pub fn new(
        ping_id: u64,
        pong_id: u64,
        flags: NodeFlags,
        gossip_entries: Vec<GossipEntry>,
    ) -> Self {
        GossipMessage {
            ping_id,
            pong_id,
            flags,
            gossip_entries,
        }
    }

    pub fn get_ping_id(&self) -> u64 {
        self.ping_id
    }

    pub fn get_pong_id(&self) -> u64 {
        self.pong_id
    }

    pub fn get_entries(&self) -> Vec<GossipEntry> {
        self.gossip_entries.clone()
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut res = Vec::new();

        res.extend_from_slice(&self.ping_id.to_be_bytes());

        res.extend_from_slice(&self.pong_id.to_be_bytes());

        res.extend_from_slice(&self.flags.to_be_bytes());

        res.extend(GossipEntry::serialize_vector(&self.gossip_entries));

        res
    }

    /// Crea un GossipMessage a partir de un buffer serializado
    pub fn from_bytes(mut data: &[u8]) -> Result<Self, String> {
        let mut cursor = Cursor::new(&mut data);

        let ping_id = read_u64_from_buffer(&mut cursor)?;
        let pong_id = read_u64_from_buffer(&mut cursor)?;

        let flag = read_u8_from_buffer(&mut cursor)?;
        let mut flags = NodeFlags::new();
        flags.set(flag);

        let entries_len = read_u16_from_buffer(&mut cursor)?;

        let mut gossip_entries = Vec::with_capacity(entries_len as usize);
        for _ in 0..entries_len {
            let entry = GossipEntry::from_bytes(&mut cursor).map_err(|e| e)?;
            gossip_entries.push(entry);
        }

        Ok(GossipMessage {
            ping_id,
            pong_id,
            flags,
            gossip_entries,
        })
    }
}

#[derive(Debug, Clone)]
pub struct GossipEntry {
    node_id: NodeId,
    node_ip: String,
    node_port: u16,
    config_epoch: Epoch,
    slots: SlotRange,
    flags: NodeFlags,
    last_pong_received: TimeStamp,
    master_id: Option<NodeId>,
    last_update_time: TimeStamp,
    replaced: bool,
}

impl GossipEntry {
    pub fn new(
        node_id: String,
        node_ip: String,
        node_port: u16,
        slots: SlotRange,
        config_epoch: Epoch,
        flags: NodeFlags,
        last_pong_received: TimeStamp,
        master_id: Option<NodeId>,
        last_update_time: TimeStamp,
        replaced: bool,
    ) -> Self {
        GossipEntry {
            node_id,
            node_ip,
            node_port,
            slots,
            config_epoch,
            flags,
            last_pong_received,
            master_id,
            last_update_time,
            replaced,
        }
    }

    pub fn get_id(&self) -> NodeId {
        self.node_id.clone()
    }

    pub fn set_id(&mut self, node_id: NodeId) {
        self.node_id = node_id;
    }

    pub fn get_ip(&self) -> String {
        self.node_ip.clone()
    }

    pub fn set_ip(&mut self, ip: String) {
        self.node_ip = ip;
    }

    pub fn get_port(&self) -> u16 {
        self.node_port
    }

    pub fn set_port(&mut self, port: u16) {
        self.node_port = port;
    }

    pub fn set_slots(&mut self, slots: SlotRange) {
        self.slots = slots;
    }

    pub fn get_slots(&self) -> SlotRange {
        self.slots
    }

    pub fn set_flags(&mut self, role: u8) {
        self.flags.set(role);
    }

    pub fn set_config_epoch(&mut self, cepoch: Epoch) {
        self.config_epoch = cepoch;
    }

    pub fn get_config_epoch(&self) -> Epoch {
        self.config_epoch
    }

    pub fn get_state(&self) -> u8 {
        self.flags.get_state()
    }

    pub fn get_master_id(&self) -> Option<NodeId> {
        self.master_id.clone()
    }

    pub fn get_last_pong_time(&self) -> TimeStamp {
        self.last_pong_received
    }

    pub fn get_last_update_time(&self) -> TimeStamp {
        self.last_update_time
    }

    pub fn was_replaced(&self) -> bool {
        self.replaced
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut res = Vec::new();

        let node_id_bytes = self.node_id.as_bytes();
        let node_id_len = node_id_bytes.len() as u16;
        res.extend_from_slice(&node_id_len.to_be_bytes());
        res.extend_from_slice(node_id_bytes);

        let node_ip_bytes = self.node_ip.as_bytes();
        let node_ip_len = node_ip_bytes.len() as u16;
        res.extend_from_slice(&node_ip_len.to_be_bytes());
        res.extend_from_slice(node_ip_bytes);

        res.extend_from_slice(&self.node_port.to_be_bytes());

        res.extend_from_slice(&self.slots.0.to_be_bytes());
        res.extend_from_slice(&self.slots.1.to_be_bytes());

        res.extend_from_slice(&self.config_epoch.to_be_bytes());

        res.extend_from_slice(&self.flags.to_be_bytes());

        res.extend_from_slice(&self.last_pong_received.to_be_bytes());

        res.extend_from_slice(&serialize_option_node_id(&self.master_id));

        res.extend_from_slice(&self.last_update_time.to_be_bytes());

        res.push(self.replaced as u8);

        res
    }

    pub fn serialize_vector(entries: &Vec<Self>) -> Vec<u8> {
        let entries_len = entries.len() as u16;
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&entries_len.to_be_bytes());
        for entry in entries {
            buffer.extend(entry.serialize());
        }
        buffer
    }

    pub fn from_bytes<R: Read>(reader: &mut R) -> Result<Self, String> {
        let node_id_len = read_u16_from_buffer(reader)?;
        let node_id = read_string_from_buffer(reader, node_id_len as usize)?;

        let node_ip_len = read_u16_from_buffer(reader)?;
        let node_ip = read_string_from_buffer(reader, node_ip_len as usize)?;

        let node_port = read_u16_from_buffer(reader)?;

        let slot_start_range = read_u16_from_buffer(reader)?;
        let slot_end_range = read_u16_from_buffer(reader)?;
        let slots: SlotRange = (slot_start_range, slot_end_range);

        let config_epoch = read_u64_from_buffer(reader)?;

        let flag = read_u8_from_buffer(reader)?;
        let mut flags = NodeFlags::new();
        flags.set(flag);

        let last_pong_received = read_timestamp_from_buffer(reader)? as i64;

        let master_id = deserialize_option_node_id(reader)?;

        let last_update_time = read_timestamp_from_buffer(reader)? as i64;

        let replaced = read_u8_from_buffer(reader)? != 0;

        Ok(GossipEntry {
            node_id,
            node_ip,
            node_port,
            slots,
            config_epoch,
            flags,
            last_pong_received,
            master_id,
            last_update_time,
            replaced,
        })
    }
}

fn serialize_option_node_id(opt: &Option<NodeId>) -> Vec<u8> {
    let mut res = Vec::new();
    match opt {
        Some(node_id) => {
            res.push(1);
            let bytes = node_id.as_bytes();
            let len = bytes.len() as u16;
            res.extend_from_slice(&len.to_be_bytes());
            res.extend_from_slice(bytes);
        }
        None => {
            res.push(0);
        }
    }
    res
}

fn deserialize_option_node_id<R: Read>(reader: &mut R) -> Result<Option<NodeId>, String> {
    let mut flag = [0u8; 1];
    reader.read_exact(&mut flag).map_err(|e| e.to_string())?;
    if flag[0] == 1 {
        let len = read_u16_from_buffer(reader)? as usize;
        let node_id = read_string_from_buffer(reader, len)?;
        Ok(Some(node_id))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cluster::state::flags::NodeFlags;

    fn sample_gossip_entry() -> GossipEntry {
        GossipEntry::new(
            "node1".to_string(),
            "0.0.0.0".to_string(),
            6379,
            (0, 100),
            1,
            NodeFlags::new(),
            1234567890,
            None,
            -1,
            false,
        )
    }

    #[test]
    fn test_serialize_and_from_bytes() {
        let entry1 = sample_gossip_entry();
        let entry2 = GossipEntry::new(
            "node2".to_string(),
            "192.168.1.1".to_string(),
            6380,
            (101, 200),
            2,
            NodeFlags::new(),
            987654321,
            None,
            -1,
            false,
        );
        let msg = GossipMessage::new(
            42,
            84,
            NodeFlags::new(),
            vec![entry1.clone(), entry2.clone()],
        );

        let serialized = msg.serialize();
        let deserialized = GossipMessage::from_bytes(&serialized).expect("Deserialization failed");

        assert_eq!(deserialized.get_ping_id(), 42);
        assert_eq!(deserialized.get_pong_id(), 84);
        assert_eq!(deserialized.get_entries().len(), 2);

        let d_entries = deserialized.get_entries();
        assert_eq!(d_entries[0].get_id(), entry1.get_id());
        assert_eq!(d_entries[1].get_id(), entry2.get_id());
        assert_eq!(d_entries[0].get_ip(), entry1.get_ip());
        assert_eq!(d_entries[1].get_ip(), entry2.get_ip());
    }

    #[test]
    fn test_serialize_gossip_entry_and_from_bytes() {
        let entry = sample_gossip_entry();
        let serialized = entry.serialize();
        let mut cursor = Cursor::new(&serialized);
        let deserialized = GossipEntry::from_bytes(&mut cursor).expect("Deserialization failed");
        assert_eq!(deserialized.get_id(), entry.get_id());
        assert_eq!(deserialized.get_ip(), entry.get_ip());
        assert_eq!(deserialized.get_port(), entry.get_port());
        assert_eq!(deserialized.get_slots(), entry.get_slots());
        assert_eq!(deserialized.get_config_epoch(), entry.get_config_epoch());
        assert_eq!(deserialized.get_state(), entry.get_state());
        assert_eq!(
            deserialized.get_last_pong_time(),
            entry.get_last_pong_time()
        );
    }

    #[test]
    fn test_serialize_gossip_entry_vector_and_from_bytes() {
        let entry1 = sample_gossip_entry();
        let entry2 = GossipEntry::new(
            "node2".to_string(),
            "192.168.1.1".to_string(),
            6380,
            (101, 200),
            2,
            NodeFlags::new(),
            987654321,
            None,
            -1,
            false,
        );
        let entries = vec![entry1.clone(), entry2.clone()];
        let serialized_vec = GossipEntry::serialize_vector(&entries);

        let mut cursor = Cursor::new(&serialized_vec);
        let len = crate::cluster::utils::read_u16_from_buffer(&mut cursor).unwrap();
        assert_eq!(len, 2);

        let d1 = GossipEntry::from_bytes(&mut cursor).unwrap();
        let d2 = GossipEntry::from_bytes(&mut cursor).unwrap();

        assert_eq!(d1.get_id(), entry1.get_id());
        assert_eq!(d2.get_id(), entry2.get_id());
    }
}
