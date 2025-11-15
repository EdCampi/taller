use crate::cluster::state::flags::{CONNECTED, HANDSHAKE, NodeFlags};
use crate::cluster::types::SlotRange;
use crate::cluster::utils::{read_string_from_buffer, read_u16_from_buffer};
use crate::cluster::{
    sharding::rehash_message::RehashMessage,
    state::{
        flags::{MASTER, SLAVE},
        node_data::NodeData,
    },
    types::{KnownNode, NodeId, NodeMessage, REHASH_TYPE},
};
use std::sync::{RwLockReadGuard, RwLockWriteGuard};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock, mpsc::Sender},
};

static MAX_AMOUNT_MASTERS: usize = 3;

pub fn process_join_msg(
    message: NodeMessage,
    node_data: &Arc<RwLock<NodeData>>,
    output_sender: &Sender<(NodeId, SocketAddr, Option<Vec<u8>>)>,
    known_nodes: &Arc<RwLock<HashMap<NodeId, KnownNode>>>,
) -> Result<(), String> {
    let join_msg = JoinMessage::from_bytes(&message.get_payload())
        .map_err(|_| "Error when processing the join message".to_string())?;
    println!(
        "\x1b[34m[CLUSTER] JoinMessage recibido: {:?}\x1b[0m",
        join_msg
    );
    handle_join_message(join_msg, output_sender, known_nodes, node_data);
    Ok(())
}

#[derive(Debug, Clone)]
pub struct JoinMessage {
    pub node_id: NodeId,
    pub ip: String,
    pub port: u16,
}

impl JoinMessage {
    pub fn new(node_id: NodeId, ip: String, port: u16) -> Self {
        Self { node_id, ip, port }
    }

    pub fn get_id(&self) -> NodeId {
        self.node_id.clone()
    }
    pub fn get_ip(&self) -> String {
        self.ip.clone()
    }
    pub fn get_port(&self) -> u16 {
        self.port
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // Serialización básica
        let mut result = Vec::new();

        let node_id_bytes = self.node_id.as_bytes();
        let node_id_len = node_id_bytes.len() as u16;
        result.extend_from_slice(&node_id_len.to_be_bytes());
        result.extend_from_slice(node_id_bytes);

        let ip_bytes = self.ip.as_bytes();
        let ip_len = ip_bytes.len() as u16;
        result.extend_from_slice(&ip_len.to_be_bytes());
        result.extend_from_slice(ip_bytes);

        result.extend_from_slice(&self.port.to_be_bytes());
        result
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        let mut cursor = std::io::Cursor::new(data);

        let id_len = read_u16_from_buffer(&mut cursor)?;
        let node_id = read_string_from_buffer(&mut cursor, id_len as usize)?;

        let ip_len = read_u16_from_buffer(&mut cursor)?;
        let ip = read_string_from_buffer(&mut cursor, ip_len as usize)?;

        let port = read_u16_from_buffer(&mut cursor)?;

        Ok(Self { node_id, ip, port })
    }
}

/// Maneja la incorporación de un nuevo nodo al cluster.
///
/// Este procedimiento:
/// - Rechaza el nodo si ya está registrado.
/// - Si hay menos de 3 nodos en total, lo asigna como `MASTER` y redistribuye slots.
/// - Si ya hay suficientes masters, lo asigna como `SLAVE` de un master existente.
/// - Finalmente, registra el nodo y dispara la conexión saliente.
///
/// Parámetros:
/// - `join_msg`: mensaje recibido del nodo que desea unirse.
/// - `output_sender`: canal para notificar que se debe abrir una conexión con el nuevo nodo.
/// - `known_nodes`: hash con todos los nodos conocidos.
/// - `actual_node`: información del nodo actual (por si necesita redistribuir slots).
///
pub fn handle_join_message(
    join_msg: JoinMessage,
    output_sender: &Sender<(NodeId, SocketAddr, Option<Vec<u8>>)>,
    known_nodes: &Arc<RwLock<HashMap<NodeId, KnownNode>>>,
    node_data_lock: &Arc<RwLock<NodeData>>,
) {
    let mut nodes = known_nodes.write().unwrap();
    let new_node_id = join_msg.get_id();
    if nodes.contains_key(&new_node_id) {
        println!(
            "[CLUSTER] Nodo {} ya estaba registrado, se ignora",
            new_node_id
        );
        return;
    }

    // Insertar nuevo nodo
    let mut new_node = KnownNode::new(join_msg.get_id(), join_msg.get_ip(), join_msg.get_port());
    new_node.get_flags_mut().set(CONNECTED);
    new_node.get_flags_mut().set(HANDSHAKE);
    let masters: Vec<_> = nodes
        .values()
        .filter(|n| n.is_master() && !n.is_fail())
        .cloned()
        .collect();

    let mut failed_masters: Vec<_> = nodes
        .values()
        .filter(|n| n.is_master() && n.is_fail() && !n.is_replaced())
        .cloned()
        .collect();

    println!(
        "[CLUSTER] Nodo {} se une. Total actuales: {} nodos, {} masters",
        new_node_id,
        nodes.len(),
        masters.len()
    );

    let addr_str = format!("{}:{}", join_msg.get_ip(), join_msg.get_port());
    let addr = addr_str.parse().unwrap();
    let node_data = node_data_lock.read().unwrap();

    // Reviso de todos esos masters los que siguen conectados...
    // Me agrego si soy un master
    let total_valid_masters = masters.len()
        + if NodeFlags::state_contains(node_data.get_state(), MASTER) {
            1
        } else {
            0
        };
    drop(node_data);

    // Decisión: master o slave
    if total_valid_masters < MAX_AMOUNT_MASTERS {
        // Si hay menos de 3 nodos en total, lo asignamos como master
        new_node.get_flags_mut().set(MASTER);

        if failed_masters.len() > 0 {
            // Si hay un master que no pudo ser reemplazado, reutilizo sus slots en el nuevo master (la data se pierde igual)
            let failed_master = failed_masters.get_mut(0).unwrap();
            println!(
                "[CLUSTER] Reemplazando master {} por {}",
                failed_master.get_id(),
                new_node_id
            );
            replace_master(
                node_data_lock,
                &new_node_id,
                &addr,
                failed_master,
                output_sender,
            );
            if let Some(failed) = nodes.get_mut(&failed_master.get_id()) {
                failed.set_as_replaced();
                failed.set_hash_slots((0, 0));
                failed.set_last_pong_time(None);
                failed.add_cepoch();
            }
            return;
        }

        println!("[CLUSTER] Node {} assigned as MASTER", new_node_id);
        // Redistribuir slots entre los masters
        if NodeFlags::state_contains(node_data_lock.read().unwrap().get_state(), MASTER) {
            let rehash_msg = rehash(&mut nodes, &node_data_lock, join_msg.clone());
            let _ = output_sender.send((new_node_id.clone(), addr, Some(rehash_msg.serialize())));
        } else {
            // Si soy una réplica no rebano mis slots, redirijo la consulta a algún master
            redirect_join_to_master(join_msg.clone(), &masters, &output_sender);
            return;
        }
    } else {
        // Si ya hay suficientes masters, asignar como slave
        new_node.get_flags_mut().set(SLAVE);
        join_slave(&node_data_lock, &mut new_node, &mut *nodes, &output_sender);
    }
    nodes.insert(new_node_id.clone(), new_node);
    println!("[CLUSTER] New node added {}", join_msg.node_id);
}

/// Usada en caso ya había un PFAIL que no tuvo reemplazos, para no perder los
/// slots para siempre, se los asigno al nuevo master (la data igualmente se pierde
/// definitivamente, como es en el caso de perder un master sin réplica alguna).
fn replace_master(
    node_data_lock: &Arc<RwLock<NodeData>>,
    new_node_id: &NodeId,
    new_node_addr: &SocketAddr,
    failed_master: &KnownNode,
    output_sender: &Sender<(NodeId, SocketAddr, Option<Vec<u8>>)>,
) {
    let rehash_msg = RehashMessage::new(
        new_node_id.clone(),
        MASTER,
        failed_master.get_slots().0,
        failed_master.get_slots().1,
        "".to_string(),
    );
    let bytes = rehash_msg.serialize();
    let node_data = node_data_lock.read().unwrap();
    let msg = NodeMessage::new(
        node_data.get_id(),
        node_data.get_ip(),
        node_data.get_port(),
        REHASH_TYPE,
        bytes.len() as u16,
        bytes,
    );
    let _ = output_sender.send((new_node_id.clone(), *new_node_addr, Some(msg.serialize())));
}

/// Está para el caso "Tengo que asignar como master, pero me llegó siendo réplica, el master
/// debe ser el que divida sus slots".
fn redirect_join_to_master(
    join_message: JoinMessage,
    masters: &Vec<KnownNode>,
    output_sender: &Sender<(NodeId, SocketAddr, Option<Vec<u8>>)>,
) {
    let payload = join_message.to_bytes();
    let msg = NodeMessage::new(
        join_message.get_id(), // Le paso como remitente el nodo que quiere unirse
        join_message.get_ip(),
        join_message.get_port(),
        REHASH_TYPE,
        payload.len() as u16,
        payload,
    );
    let master_dst = masters.get(0).unwrap();
    println!(
        "[CLUSTER] Redirigiendo join a master {}",
        master_dst.get_id()
    );
    let _ = output_sender.send((
        master_dst.get_id().clone(),
        master_dst.get_addr(),
        Some(msg.serialize()),
    ));
}

/*
 * Redistribuye los hash slots entre el nodo actual y un nuevo nodo que se une al cluster.
 *
 * Este procedimiento:
 * - Toma la mitad de los slots del nodo actual.
 * - Asigna esos slots al nuevo nodo.
 * - Actualiza localmente los rangos de slots de ambos nodos.
 * - Envía un mensaje `RehashMessage` al nuevo nodo para notificarle los slots asignados.
 *
 * Parámetros:
 * - `new_node`: referencia mutable al nuevo nodo que se incorpora al cluster.
 * - `nodes`: mapa mutable de nodos conocidos, usado para actualizar el nuevo nodo.
 * - `actual_node`: referencia al estado del nodo actual, usado para redistribuir slots.
 * - `join_msg`: mensaje original de unión del nuevo nodo, con sus datos de red.
 * - `output_sender`: canal que podría usarse para disparar comunicaciones (no se usa en esta función, pero se deja por consistencia).
*/
fn rehash(
    nodes: &mut RwLockWriteGuard<HashMap<NodeId, KnownNode>>,
    actual_node: &Arc<RwLock<NodeData>>,
    join_msg: JoinMessage,
) -> NodeMessage {
    let mut myself = actual_node.write().unwrap();
    let mut my_slots = myself.get_slots();
    let aux = my_slots.1;
    let half = myself.get_slots_len() / 2;
    my_slots.1 = my_slots.0 + half;
    let new_node_slots = (my_slots.1 + 1, aux);

    myself.set_slots(my_slots);
    myself.add_cepoch();

    // Asignar al nuevo nodo
    if let Some(new_node) = nodes.get_mut(&join_msg.node_id) {
        new_node.set_hash_slots(new_node_slots);
    }

    println!(
        "[CLUSTER] Slots {} reassigned to {}",
        new_node_slots.1 - new_node_slots.0,
        join_msg.get_id(),
    );

    let start = new_node_slots.0;
    let end = new_node_slots.1;

    let rehash_msg = RehashMessage::new(join_msg.get_id(), MASTER, start, end, "".to_string());
    let rehash_bytes = rehash_msg.serialize();

    let aux = NodeMessage::new(
        myself.get_id(),
        myself.get_ip(),
        myself.get_port(),
        REHASH_TYPE,
        rehash_bytes.len() as u16,
        rehash_bytes,
    );
    aux
}

/*
 * Asigna un nodo nuevo como réplica (SLAVE) de uno de los masters existentes.
 *
 * Este procedimiento:
 * - Selecciona un master de forma balanceada (round-robin).
 * - Marca al nuevo nodo como SLAVE.
 * - Establece en el nuevo nodo su master asignado.
 * - Agrega el nuevo nodo a la lista de réplicas del master correspondiente.
 *
 * Parámetros:
 * - `new_id`: ID del nodo que se está incorporando.
 * - `new_node`: referencia mutable al nuevo nodo a configurar como SLAVE.
 * - `masters`: lista de referencias a los nodos master actualmente activos.
 * - `nodes`: mapa de nodos conocidos, usado para actualizar el nodo master.
 */
fn join_slave(
    node_data_lock: &Arc<RwLock<NodeData>>,
    new_node: &mut KnownNode,
    nodes: &mut HashMap<NodeId, KnownNode>,
    output_sender: &Sender<(NodeId, SocketAddr, Option<Vec<u8>>)>,
) {
    let master_id = get_master_with_least_slaves(nodes, node_data_lock).unwrap();
    println!(
        "[CLUSTER] Nodo {} asignado como SLAVE de {}",
        new_node.get_id(),
        master_id
    );

    new_node.get_flags_mut().set(SLAVE);
    new_node.set_master(Some(master_id.clone()));

    let node_data = node_data_lock.read().unwrap();
    if let Some(master_node) = nodes.get_mut(&master_id) {
        let before = master_node.get_replicas().len();
        master_node.add_replica(new_node.get_id());
        let after = master_node.get_replicas().len();
        println!(
            "[CLUSTER] Master {} ahora tiene {} réplicas (antes tenía {})",
            master_id, after, before
        );

        // Acá saco los slots de los nodos conocidos
        let slots = master_node.get_slots();
        send_rehash_message(&node_data, new_node, slots, master_id, output_sender);
    } else if node_data.get_id() == master_id {
        println!(
            "[CLUSTER] Me agrego a mi mismo el nodo {} como mí réplica",
            master_id
        );
        // Acá yo le doy mis slots a la nueva réplica (yo no estoy entre la lista de los conocidos)
        let slots = node_data.get_slots();
        send_rehash_message(&node_data, new_node, slots, master_id, output_sender);
    }
}

/// Crea y envía el mensaje de rehash al nuevo nodo a recibir dentro del cluster.
fn send_rehash_message(
    node_data: &RwLockReadGuard<NodeData>,
    new_node: &KnownNode,
    slots: SlotRange,
    master_id: NodeId,
    output_sender: &Sender<(NodeId, SocketAddr, Option<Vec<u8>>)>,
) {
    let rehash_msg = RehashMessage::new(
        new_node.get_id(),
        SLAVE,
        slots.0,
        slots.1,
        master_id.clone(),
    );
    let rehash_bytes = rehash_msg.serialize();
    let rehash_bytes_len = rehash_bytes.len() as u16;

    let new_node_addr = SocketAddr::new(new_node.get_addr().ip(), new_node.get_addr().port());

    let msg = NodeMessage::new(
        node_data.get_id(),
        node_data.get_ip(),
        node_data.get_port(),
        REHASH_TYPE,
        rehash_bytes_len,
        rehash_bytes,
    );

    let _ = output_sender.send((new_node.get_id(), new_node_addr, Some(msg.serialize())));
}

/// Devuelve el nodo cuya cantidad de réplicas es la menor para mantener
/// el cluster equilibrado.
fn get_master_with_least_slaves(
    nodes: &HashMap<NodeId, KnownNode>,
    node_data_lock: &Arc<RwLock<NodeData>>,
) -> Option<NodeId> {
    let mut master_counts: HashMap<NodeId, usize> = HashMap::new();
    let node_data = node_data_lock.read().unwrap();
    if NodeFlags::state_contains(node_data.get_state(), MASTER) {
        master_counts.insert(node_data.get_id(), 0);
    } else {
        master_counts.insert(node_data.get_master_id().unwrap().clone(), 1);
    }

    drop(node_data);
    for (node_id, data) in nodes.iter() {
        if data.is_master() && !data.is_fail() {
            master_counts.insert(node_id.clone(), 0);
        }
    }

    for (_, node) in nodes.iter() {
        if let Some(master_id) = node.get_master_id() {
            if let Some(count) = master_counts.get_mut(master_id) {
                *count += 1;
            }
        }
    }

    master_counts
        .into_iter()
        .min_by_key(|&(_, count)| count)
        .map(|(master_id, _)| master_id)
}
