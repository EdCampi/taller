use crate::cluster::types::TimeStamp;
use crate::cluster::utils::{
    read_string_from_buffer, read_timestamp_from_buffer, read_u16_from_buffer,
};
use crate::storage::DataStore;
use std::io::Read;

pub struct PsyncMessage {
    pub node_id: String,
    pub last_update_time: TimeStamp,
    pub data_store: DataStore,
}

impl PsyncMessage {
    pub fn new(
        node_id: String,
        data_store: DataStore,
        last_update_time: Option<TimeStamp>,
    ) -> Self {
        PsyncMessage {
            node_id,
            last_update_time: if let Some(time) = last_update_time {
                time
            } else {
                -1
            },
            data_store,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let id_bytes = self.node_id.as_bytes();
        bytes.extend_from_slice(&(id_bytes.len() as u16).to_be_bytes());
        bytes.extend_from_slice(id_bytes);
        bytes.extend_from_slice(&self.last_update_time.to_be_bytes());
        bytes.extend_from_slice(&self.data_store.serialize());
        bytes
    }

    pub fn from_bytes<R: Read>(buffer: &mut R) -> Self {
        let node_id_len = read_u16_from_buffer(buffer).unwrap();
        let node_id = read_string_from_buffer(buffer, node_id_len as usize).unwrap();
        let last_update_time = read_timestamp_from_buffer(buffer).unwrap();
        let data_store = DataStore::from_bytes(buffer).unwrap();

        PsyncMessage {
            node_id,
            last_update_time,
            data_store,
        }
    }
}
