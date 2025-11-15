//! Funciones que proveen funcionalidad a los comandos
//! (command_executor.rs, Command)
//!
//! Este módulo implementa la lógica de ejecución de los comandos de la base de datos,
//! incluyendo operaciones sobre strings, listas, sets, y comandos de pubsub y persistencia.
//!
//! # Errores
//! Todas las funciones retornan un enum CommandError para manejo robusto de errores.

// IMPORTS
use super::types::ResponseType;
use crate::cluster::cluster_node::ClusterNode;
use crate::cluster::state::node_data::NodeData;
use crate::cluster::types::{KnownNode, NodeId, SlotRange};
use crate::command::types::Command;
use crate::config::node_configs::NodeConfigs;
use crate::logs::aof_logger::AofLogger;
use crate::network::RespMessage;
use crate::storage::DataStore;
use crate::storage::snapshot_manager::create_dump;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};
use std::thread;

/// Errores específicos de comandos
#[derive(Debug)]
pub enum CommandError {
    WrongType,
    WrongNumArgs,
    NotFound,
    IoError(String),
    Internal(String),
    Custom(String),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::WrongType => write!(
                f,
                "WRONGTYPE Operation against a key holding the wrong kind of value"
            ),
            CommandError::WrongNumArgs => write!(f, "ERR wrong number of arguments for command"),
            CommandError::NotFound => write!(f, "ERR not found"),
            CommandError::IoError(e) => write!(f, "IO error: {}", e),
            CommandError::Internal(e) => write!(f, "Internal error: {}", e),
            CommandError::Custom(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for CommandError {}

// MENSAJES DE ERROR
// const ERR_WRONG_TYPE: &str = "WRONGTYPE Operation against a key holding the wrong kind of value";
const ERR_WRONG_NUM_ARGS: &str = "ERR wrong number of arguments for '_' command";

// CÓDIGOS DE ERROR
const STR_CODE: i64 = 0;
const LIST_CODE: i64 = 1;
const SET_CODE: i64 = 2;

// CÓDIGO

/// Revisa que la clave no sea del tipo deseado.
///
/// # Arguments
///
/// * `store` Store de hashmaps
/// * `key` Clave a buscar
/// * `code` Código de tipo de dato buscar
///
/// # Returns
///
/// Verdadero si el valor no es del tipo buscado. Caso contrario, Falso.
fn wrong_type_error(store: &DataStore, key: &String, code: i64) -> bool {
    match code {
        STR_CODE => store.list_db.contains_key(key) || store.set_db.contains_key(key),
        LIST_CODE => store.string_db.contains_key(key) || store.set_db.contains_key(key),
        SET_CODE => store.string_db.contains_key(key) || store.list_db.contains_key(key),
        _ => false,
    }
}

pub fn set(
    store: &mut DataStore,
    key: String,
    value: String,
) -> Result<ResponseType, CommandError> {
    store.list_db.remove(&key);
    store.set_db.remove(&key);
    store.string_db.insert(key, value);
    Ok(ResponseType::Str("OK".to_string()))
}

pub fn get(store: &DataStore, key: &String) -> Result<ResponseType, CommandError> {
    if wrong_type_error(store, key, STR_CODE) {
        return Err(CommandError::WrongType);
    }
    if let Some(value) = store.string_db.get(key) {
        return Ok(ResponseType::Str(value.clone()));
    }
    Ok(ResponseType::Null(None))
}

pub fn append(
    store: &mut DataStore,
    key: String,
    values: Vec<String>,
) -> Result<ResponseType, CommandError> {
    if wrong_type_error(store, &key, LIST_CODE) {
        return Err(CommandError::WrongType);
    }
    if let Some(list) = store.list_db.get_mut(&key) {
        list.extend(values);
        return Ok(ResponseType::Int(list.len() as i64));
    }
    let original_len = values.len();
    let mut new_list = Vec::new();
    new_list.extend(values);
    store.list_db.insert(key, new_list);
    Ok(ResponseType::Int(original_len as i64))
}

pub fn sadd(
    store: &mut DataStore,
    key: String,
    values: Vec<String>,
) -> Result<ResponseType, CommandError> {
    if wrong_type_error(store, &key, SET_CODE) {
        return Err(CommandError::WrongType);
    }
    let set = store.set_db.entry(key).or_insert_with(HashSet::new);
    let mut added = 0;
    for v in values {
        if set.insert(v) {
            added += 1;
        }
    }
    Ok(ResponseType::Int(added))
}

pub fn get_slice(
    store: &DataStore,
    key: &String,
    start: i64,
    end: i64,
) -> Result<ResponseType, CommandError> {
    if wrong_type_error(store, key, LIST_CODE) {
        return Err(CommandError::WrongType);
    }
    if let Some(list) = store.list_db.get(key) {
        let len = list.len() as i64;
        let s = if start < 0 {
            (len + start).max(0)
        } else {
            start
        };
        let e = if end < 0 || end >= len { len - 1 } else { end };

        if s > e {
            return Ok(ResponseType::List(vec![]));
        }
        let s_usize = s as usize;
        let e_usize = e as usize;
        return Ok(ResponseType::List(list[s_usize..=e_usize].to_vec()));
    }
    Ok(ResponseType::List(vec![]))
}

pub fn move_vec_to_set(set: &mut HashSet<String>, vec: &Vec<String>) {
    for val in vec {
        set.insert(val.clone());
    }
}

pub fn get_set_items(store: &DataStore, key: &String) -> Result<ResponseType, CommandError> {
    if wrong_type_error(store, key, SET_CODE) {
        return Err(CommandError::WrongType);
    }
    if let Some(set) = store.set_db.get(key) {
        return Ok(ResponseType::Set(set.clone()));
    }
    Ok(ResponseType::Set(HashSet::new()))
}

pub fn get_set_data(
    store: &DataStore,
    key: &String,
    val: &String,
) -> Result<ResponseType, CommandError> {
    if wrong_type_error(store, key, SET_CODE) {
        return Err(CommandError::WrongType);
    }
    if let Some(set) = store.set_db.get(key) {
        if set.contains(val) {
            return Ok(ResponseType::Int(1));
        }
    }
    Ok(ResponseType::Int(0))
}

pub fn move_data_to_other_set(
    store: &mut DataStore,
    src_key: &String,
    dst_key: &String,
    value: &String,
) -> Result<ResponseType, CommandError> {
    if wrong_type_error(store, src_key, SET_CODE) || wrong_type_error(store, dst_key, SET_CODE) {
        return Err(CommandError::WrongType);
    }
    if let Some(src_set) = store.set_db.get_mut(src_key) {
        if src_set.contains(value) {
            src_set.remove(value);
            let dest_set = store
                .set_db
                .entry(dst_key.clone())
                .or_insert_with(HashSet::new);
            dest_set.insert(value.clone());
            return Ok(ResponseType::Int(1));
        }
    }
    Ok(ResponseType::Int(0))
}

pub fn left_push(
    store: &mut DataStore,
    key: &String,
    vec: &Vec<String>,
) -> Result<ResponseType, CommandError> {
    if wrong_type_error(store, key, LIST_CODE) {
        return Err(CommandError::WrongType);
    }

    if let Some(list) = store.list_db.get_mut(key) {
        for item in vec.iter().rev() {
            list.insert(0, item.clone());
        }
        return Ok(ResponseType::Int(list.len() as i64));
    }

    // Si la key no existe, la creo y después hago push
    let mut new_list = Vec::new();
    for item in vec.iter().rev() {
        new_list.insert(0, item.clone());
    }
    store.list_db.insert(key.clone(), new_list);
    Ok(ResponseType::Int(vec.len() as i64))
}

pub fn string_slice(
    store: &DataStore,
    key: &String,
    start: &i64,
    end: &i64,
) -> Result<ResponseType, CommandError> {
    if wrong_type_error(store, key, STR_CODE) {
        return Err(CommandError::WrongType);
    }
    if let Some(value) = store.string_db.get(key) {
        let len = value.len() as i64;
        let floor = if *start < 0 { len + *start } else { *start };
        let roof = if *end < 0 { len + *end } else { *end };

        if floor < 0 || floor >= len || floor > roof {
            return Ok(ResponseType::Str("".to_string()));
        }

        let floor = floor.max(0) as usize;

        let aux = roof.min(len) as usize;
        let roof = if aux == value.len() { aux } else { aux + 1 };

        return Ok(ResponseType::Str(value[floor..roof].to_string()));
    }
    Ok(ResponseType::Str("".to_string()))
}

pub fn get_len(
    store: &DataStore,
    key: &String,
    op: &Command,
) -> Result<ResponseType, CommandError> {
    if let Command::Llen(_) = op {
        if let Some(list) = store.list_db.get(key) {
            return Ok(ResponseType::Int(list.len() as i64));
        }
    }
    if let Command::Scard(_) = op {
        if let Some(set) = store.set_db.get(key) {
            return Ok(ResponseType::Int(set.len() as i64));
        }
    }
    if let Command::Strlen(_) = op {
        if let Some(s) = store.string_db.get(key) {
            return Ok(ResponseType::Int(s.len() as i64));
        }
    }

    if store.list_db.contains_key(key)
        || store.set_db.contains_key(key)
        || store.string_db.contains_key(key)
    {
        return Err(CommandError::WrongType);
    }
    Ok(ResponseType::Int(0))
}

pub fn str_concat(
    store: &mut DataStore,
    key: &String,
    val: &String,
) -> Result<ResponseType, CommandError> {
    if wrong_type_error(store, key, STR_CODE) {
        return Err(CommandError::WrongType);
    }

    if let Some(str) = store.string_db.get_mut(key) {
        str.push_str(val);
        return Ok(ResponseType::Int(str.len() as i64));
    }

    let new_str = val.to_string();
    let res = new_str.len();
    store.string_db.insert(key.clone(), new_str);
    Ok(ResponseType::Int(res as i64))
}

pub fn retrieve_delete(store: &mut DataStore, key: &String) -> Result<ResponseType, CommandError> {
    if wrong_type_error(store, key, STR_CODE) {
        return Err(CommandError::WrongType);
    }

    if let Some(value) = store.string_db.remove(key) {
        return Ok(ResponseType::Str(value));
    }
    Ok(ResponseType::Null(None))
}

pub fn bulk_delete(
    store: &mut DataStore,
    keys: &Vec<String>,
) -> Result<ResponseType, CommandError> {
    if keys.len() == 0 {
        let _err_msg = ERR_WRONG_NUM_ARGS.replace("_", "del");
        return Err(CommandError::WrongNumArgs);
    };
    let mut deleted_keys = 0;
    for key in keys {
        if let Some(_) = store.string_db.remove(key) {
            deleted_keys += 1;
        }
        if let Some(_) = store.list_db.remove(key) {
            deleted_keys += 1;
        }
        if let Some(_) = store.set_db.remove(key) {
            deleted_keys += 1;
        }
    }
    Ok(ResponseType::Int(deleted_keys))
}

pub fn list_pop(
    store: &mut DataStore,
    key: &String,
    amount: &i64,
    op: &Command,
) -> Result<ResponseType, CommandError> {
    if wrong_type_error(store, key, LIST_CODE) {
        return Err(CommandError::WrongType);
    }

    let mut counter = 0;
    let mut res = vec![];
    if let Some(list) = store.list_db.get_mut(key) {
        let original_len = list.len();
        while counter < *amount && (counter as usize) < original_len {
            let index_to_rmv = match op {
                Command::Lpop(_, _) => 0,
                Command::Rpop(_, _) => list.len() - 1,
                _ => return Err(CommandError::WrongType),
            };

            let aux = list.remove(index_to_rmv);
            res.push(aux);
            counter += 1
        }
        return Ok(ResponseType::List(res));
    }
    Ok(ResponseType::Null(None))
}

pub fn set_pop(
    store: &mut DataStore,
    key: &String,
    amount: &i64,
) -> Result<ResponseType, CommandError> {
    if wrong_type_error(store, key, SET_CODE) {
        return Err(CommandError::WrongType);
    }
    let mut res = vec![];
    if let Some(set) = store.set_db.get_mut(key) {
        let mut counter: usize = 0;
        let mut aux_vec: Vec<String> = set.iter().cloned().collect();
        let set_size = set.len();
        while counter < (*amount) as usize && counter < set_size {
            let aux_val = aux_vec.remove(0);
            res.push(aux_val.clone());
            set.remove(&aux_val);
            counter += 1;
        }
        return Ok(ResponseType::List(res));
    }
    Ok(ResponseType::Null(None))
}

pub fn backup_ds(
    store: &DataStore,
    settings: NodeConfigs,
    logger: Arc<AofLogger>,
    bg_task: bool,
) -> Result<ResponseType, CommandError> {
    if !bg_task {
        return match create_dump(store, &settings.get_snapshot_dst()) {
            Ok(_) => {
                logger.log_notice("DB saved on disk".to_string());
                Ok(ResponseType::Str("OK".to_string()))
            }
            Err(_) => Err(CommandError::IoError(
                "ERROR when saving the database".to_string(),
            )),
        };
    }
    let store_aux = store.clone();
    let logger_aux = logger.clone();
    logger.log_notice("DB background thread started".to_string());
    let _ = thread::Builder::new()
        .name("Background save".to_string())
        .spawn(
            move || match create_dump(&store_aux, &settings.get_snapshot_dst()) {
                Ok(_) => {
                    logger_aux.log_notice("DB saved on disk".to_string());
                }
                Err(_) => {
                    logger_aux.log_event("ERROR when saving the database".to_string());
                }
            },
        );
    Ok(ResponseType::Str("Background saving started".to_string()))
}

pub fn subscribe(
    client_id: String,
    channel_id: String,
    pubsub_sender: &Sender<(String, Command, Sender<String>, Sender<RespMessage>)>,
    client_sender: &Sender<RespMessage>,
) -> Result<ResponseType, CommandError> {
    let (response_sender, response_receiver) = mpsc::channel::<String>();
    let command = Command::Subscribe(channel_id);

    pubsub_sender
        .send((client_id, command, response_sender, client_sender.clone()))
        .map_err(|e| {
            CommandError::Custom(format!("Failed to send subscribe instruction: {}", e))
        })?;

    let response = response_receiver.recv().map_err(|e| {
        CommandError::Custom(format!("Failed to receive subscribe response: {}", e))
    })?;

    if response.is_empty() {
        Ok(ResponseType::Str("Successfully subscribed.".to_string()))
    } else {
        Err(CommandError::Custom(response))
    }
}

pub fn unsubscribe(
    client_id: String,
    channel_id: String,
    pubsub_sender: &Sender<(String, Command, Sender<String>, Sender<RespMessage>)>,
) -> Result<ResponseType, CommandError> {
    let (response_sender, response_receiver) = mpsc::channel::<String>();
    let command = Command::Unsubscribe(channel_id);

    let (_dummy_sender, _dummy_receiver) = std::sync::mpsc::channel();
    pubsub_sender
        .send((client_id, command, response_sender, _dummy_sender))
        .map_err(|e| {
            CommandError::Custom(format!("Failed to send unsubscribe instruction: {}", e))
        })?;

    let response = response_receiver.recv().map_err(|e| {
        CommandError::Custom(format!("Failed to receive unsubscribe response: {}", e))
    })?;

    if response.is_empty() {
        Ok(ResponseType::Str("Successfully unsubscribed".to_string()))
    } else {
        Err(CommandError::Custom(response))
    }
}

pub fn publish(
    client_id: String,
    channel_id: String,
    pubsub_sender: &Sender<(String, Command, Sender<String>, Sender<RespMessage>)>,
    message: &RespMessage,
) -> Result<ResponseType, CommandError> {
    let (response_sender, response_receiver) = mpsc::channel::<String>();
    let command = Command::Publish(channel_id, message.clone());

    let (_dummy_sender, _dummy_receiver) = std::sync::mpsc::channel();
    pubsub_sender
        .send((client_id, command, response_sender, _dummy_sender))
        .map_err(|e| CommandError::Custom(format!("Failed to send publish instruction: {}", e)))?;

    let response = response_receiver
        .recv()
        .map_err(|e| CommandError::Custom(format!("Failed to receive publish response: {}", e)))?;

    if response.is_empty() {
        Ok(ResponseType::Str("Successfully published".to_string()))
    } else {
        match response.parse::<i64>() {
            Ok(subscriber_count) => Ok(ResponseType::Int(subscriber_count)),
            Err(_) => Err(CommandError::Custom(response)),
        }
    }
}

pub fn send_first_ping(ip: &String, settings: NodeConfigs) -> Result<ResponseType, CommandError> {
    let _ = ClusterNode::connect_to_cluster(settings, Some(ip.to_string()), None);
    Ok(ResponseType::Str("Ok".to_string()))
}

/// Devuelve los slots y los nodos que los contienen.
pub fn return_cluster_slots_data(
    node_data_lock: &Arc<RwLock<NodeData>>,
    known_nodes_lock: &Arc<RwLock<HashMap<NodeId, KnownNode>>>,
) -> Result<ResponseType, CommandError> {
    let mut map: HashMap<SlotRange, Vec<Vec<String>>> = HashMap::new();
    let node_data = node_data_lock.read().unwrap();
    let slots = node_data.get_slots();
    let myself = vec![
        node_data.get_addr().ip().to_string(),
        node_data.get_addr().port().to_string(),
        node_data.get_id().to_string(),
        "MASTER".to_string(),
    ];
    map.insert(slots, vec![myself]);

    let known_nodes = known_nodes_lock.read().unwrap();
    for (id, node) in known_nodes.iter() {
        let slots = node.get_slots();
        let role = if node.is_master() {
            "MASTER".to_string()
        } else {
            "SLAVE".to_string()
        };
        if !map.contains_key(&slots) {
            let addr = node.get_addr();
            let data = vec![
                addr.ip().to_string(),
                (addr.port() - 10000).to_string(),
                id.to_string(),
                role,
            ];
            map.insert(slots, vec![data]);
        } else {
            let data = map.get_mut(&slots).unwrap();
            let addr = node.get_addr();
            data.push(vec![
                addr.ip().to_string(),
                (addr.port() - 10000).to_string(),
                id.to_string(),
                role,
            ]);
        }
    }
    // Lo aplana a la forma [0, 5460, 0.0.0.0, 30001, id, SLAVE, localhost, 30004, id, MASTER, 215.2.1.1, 30002, id, SLAVE, 5461, 16834, ...]
    let mut res = vec![];
    for (key, nodes) in map.iter() {
        res.push(key.0.to_string());
        res.push(key.1.to_string());
        for node in nodes.iter() {
            for value in node.iter() {
                res.push(value.to_string());
            }
        }
    }
    Ok(ResponseType::List(res))
}
