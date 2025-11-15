use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock, mpsc::Sender},
    thread,
};

use crate::{
    cluster::{
        comms::psync_message::PsyncMessage,
        state::{
            flags::{MASTER, NodeFlags},
            node_data::NodeData,
        },
        types::{KnownNode, NodeId, NodeMessage, REQUEST_PSYNC_TYPE},
    },
    storage::DataStore,
};

static PSYNC_INTERVAL: u64 = 2;

//Mensaje de confirmacion, devuelvo la data store actualizada
pub fn psync_sender(
    node_data: Arc<RwLock<NodeData>>,
    data_store: Arc<RwLock<DataStore>>,
    output: Sender<(NodeId, SocketAddr, Option<Vec<u8>>)>,
    nodos_conocidos: Arc<RwLock<HashMap<NodeId, KnownNode>>>,
) {
    loop {
        thread::sleep(std::time::Duration::from_secs(PSYNC_INTERVAL));
        // Check if the node is a master node
        psync_send(&node_data, &data_store, &output, &nodos_conocidos);
    }
}

pub fn psync_send(
    node_data: &Arc<RwLock<NodeData>>,
    data_store: &Arc<RwLock<DataStore>>,
    output: &Sender<(NodeId, SocketAddr, Option<Vec<u8>>)>,
    nodos_conocidos: &Arc<RwLock<HashMap<String, KnownNode>>>,
) {
    let myself = node_data.read().unwrap();

    if NodeFlags::state_contains(myself.get_state(), MASTER) {
        return; // Soy master, no hago nada
    };

    let id_de_mi_master = myself.get_master_id().unwrap_or_default();
    let nodos_conocidos = nodos_conocidos.read().unwrap();

    //de los conocidos, busco el nodo que es mi master

    if let Some(master_node) = nodos_conocidos.get(&id_de_mi_master) {
        let psync_message =
            PsyncMessage::new(myself.get_id(), data_store.read().unwrap().clone(), None);

        let bytes = psync_message.serialize();

        println!("[PS-CLUSTER] PSYNC message hacia {}", master_node.get_id());
        //Armo NodeMessage
        let message = NodeMessage::new(
            myself.get_id(),
            myself.get_ip(),
            myself.get_port(),
            REQUEST_PSYNC_TYPE,
            bytes.len() as u16,
            bytes,
        );
        // Envio el mensaje al master
        println!(
            "[PS-CLUSTER] Envío un PSYNC message desde réplica hacia {}",
            master_node.get_id()
        );
        let master_addr = master_node.get_addr();
        let node_master_addr = SocketAddr::new(master_addr.ip(), master_addr.port());
        output
            .send((
                master_node.get_id(),
                node_master_addr,
                Some(message.serialize()),
            ))
            .expect("[PS-CLUSTER] Failed to send PSYNC message");
    }
}
