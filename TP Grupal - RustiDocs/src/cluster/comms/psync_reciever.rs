use crate::cluster::state::flags::{NodeFlags, SLAVE};
use crate::cluster::types::REQUEST_PSYNC_TYPE;
use crate::cluster::utils::system_time_to_i64;
use crate::{
    cluster::{
        comms::psync_message::PsyncMessage,
        state::node_data::NodeData,
        types::{NodeId, NodeMessage},
    },
    storage::DataStore,
};
use std::io::Cursor;
use std::sync::RwLockWriteGuard;
use std::time::SystemTime;
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock, mpsc::Sender},
};

/// Propone iniciar el psync, se lo manda al maestro y el maestro hace los cambios en la data store para devolver.
pub fn process_psync_message(
    message: NodeMessage,
    node_data: &Arc<RwLock<NodeData>>,
    data_store: &Arc<RwLock<DataStore>>,
    output: &Sender<(NodeId, SocketAddr, Option<Vec<u8>>)>,
) -> Result<(), String> {
    let mut myself = node_data.write().unwrap();
    if NodeFlags::state_contains(myself.get_state(), SLAVE) {
        return update_data_store(message, &mut myself, data_store);
    }

    let mut payload = message.get_payload();
    let mut cursor = Cursor::new(&mut payload);
    let psync_message = PsyncMessage::from_bytes(&mut cursor);
    let replica_node_id = psync_message.node_id.clone();
    let data_store_replica = psync_message.data_store;

    let mut updated_data_store = data_store_replica.clone();
    let master_data_store = data_store.read().unwrap();

    DataStore::sync_database(
        &master_data_store.string_db,
        &mut updated_data_store.string_db,
    );
    DataStore::sync_database(&master_data_store.list_db, &mut updated_data_store.list_db);
    DataStore::sync_database(&master_data_store.set_db, &mut updated_data_store.set_db);

    let node_addr = message.get_addr();

    let psync_res = PsyncMessage::new(
        replica_node_id.clone(),
        updated_data_store,
        Some(system_time_to_i64(SystemTime::now())),
    );
    let bytes = psync_res.serialize();

    let response = NodeMessage::new(
        myself.get_id(),
        myself.get_ip(),
        myself.get_port(),
        REQUEST_PSYNC_TYPE,
        bytes.len() as u16,
        bytes,
    );

    if let Err(e) = output.send((replica_node_id, node_addr, Some(response.serialize()))) {
        eprintln!("Failed to send PSYNC response: {}", e);
    }
    Ok(())
}

fn update_data_store(
    message: NodeMessage,
    myself: &mut RwLockWriteGuard<NodeData>,
    data_store: &Arc<RwLock<DataStore>>,
) -> Result<(), String> {
    let mut payload = message.get_payload();
    let mut cursor = Cursor::new(&mut payload);
    let psync_message = PsyncMessage::from_bytes(&mut cursor);

    let mut data_store = data_store.write().unwrap();

    data_store.update(psync_message.data_store);
    myself.set_last_update_time(system_time_to_i64(SystemTime::now()));
    Ok(())
}
