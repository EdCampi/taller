use crate::app::utils::connect_to_cluster;
use crate::command::utils::parse_flat_cluster_slots;
use crate::{
    cluster::{sharding::hash_slot::hash_slot, types::SlotRange},
    network::{RespMessage, resp_parser::parse_resp_line},
};
use std::{
    collections::HashMap,
    io::{BufReader, Error, Write},
    net::TcpStream,
};

type HashRange = (u16, u16);
type NodeData = Vec<String>;

#[derive(Debug)]
pub enum ClusterError {
    GetKeyIsEmpty,
    GetInvalidData,
    InvalidRedisResponse,
    TcpConnectionError,
    NotSubscribedToChannel,
    CannotGetClusterData,
}

/// Struct encargado de la conexion con un cluster de redis
#[derive(Debug)]
pub struct ClusterManager {
    // Place Holder de 1 solo nodo proximamente se deberian  tener mas
    pub active_node: TcpStream,
    pub node_address: String,
    cluster_data: HashMap<HashRange, Vec<NodeData>>,
    username: String,
    password: String,
}

/// Convierte bytes en una cadena hexadecimal segura
fn bytes_to_hex_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

/// Convierte una cadena hexadecimal de nuevo a bytes
fn hex_string_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    // Verifica que la longitud sea par
    if hex.len() % 2 != 0 {
        return Err("Longitud de cadena hex inválida".to_string());
    }

    let mut bytes = Vec::with_capacity(hex.len() / 2);
    let mut i = 0;

    while i < hex.len() {
        // Tomar dos caracteres por byte
        let byte = u8::from_str_radix(&hex[i..i + 2], 16)
            .map_err(|_| format!("Carácter hex inválido: {}", &hex[i..i + 2]))?;
        bytes.push(byte);
        i += 2;
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_hex_string_conversion() {
        let original = b"Hola mundo! 1234567890";
        let hex = bytes_to_hex_string(original);
        let back = hex_string_to_bytes(&hex).unwrap();
        assert_eq!(original.to_vec(), back);
    }
    #[test]
    fn test_hex_string_conversion_all_bytes() {
        let original: Vec<u8> = (0..=255).collect();
        let hex = bytes_to_hex_string(&original);
        let back = hex_string_to_bytes(&hex).unwrap();
        assert_eq!(original, back);
    }
}

impl ClusterManager {
    /// Se le pasa la ip a 1 nodo del cluster,
    /// Devuelve error si no se pudo conectar.
    pub fn new(address: String, username: String, password: String) -> Result<Self, Error> {
        println!(
            "[ClusterManager::new] Connecting to node at address: {}",
            address
        );
        let (node_stream, _) =
            connect_to_cluster(address.clone(), username.clone(), password.clone())?;

        let mut cluster = Self {
            active_node: node_stream,
            node_address: address.clone(),
            username,
            cluster_data: HashMap::new(),
            password,
        };

        println!("[ClusterManager::new] Filling cluster data...");
        cluster.fill_cluster().unwrap_or_else(|e| {
            println!("[ClusterManager::new] Error filling cluster: {:?}", e);
        });

        for element in &cluster.cluster_data {
            println!("[ClusterManager::new] Cluster data element: {:?}", element)
        }

        Ok(cluster)
    }
    pub fn get(&mut self, key: &str) -> Result<Vec<u8>, ClusterError> {
        println!("[ClusterManager::get] Called with key: {}", key);
        match self.ensure_correct_node(key) {
            Ok(_) => println!("[ClusterManager::get] ensure_correct_node OK"),
            Err(e) => {
                println!("[ClusterManager::get] ensure_correct_node ERROR: {:?}", e);
                return Err(e);
            }
        }

        let resp = create_get(key);
        println!("\x1b[34m[ClusterManager::get] Sending GET command\x1b[0m");

        // Intento de escritura con reconexión automática
        let mut tried_reconnect = false;
        'retry: loop {
            let write_result = self.active_node.write_all(&resp);
            let flush_result = self.active_node.flush();
            if write_result.is_err() || flush_result.is_err() {
                let write_err = write_result.as_ref().err();
                let flush_err = flush_result.as_ref().err();
                println!(
                    "[ClusterManager::get] Error writing/flushing to active_node: write={:?}, flush={:?}",
                    write_err, flush_err
                );
                if !tried_reconnect {
                    println!(
                        "[ClusterManager::get] Intentando reconectar tras error de escritura..."
                    );
                    match connect_to_cluster(
                        self.node_address.clone(),
                        self.username.clone(),
                        self.password.clone(),
                    ) {
                        Ok((new_stream, _)) => {
                            self.active_node = new_stream;
                            tried_reconnect = true;
                            continue 'retry;
                        }
                        Err(e) => {
                            println!("[ClusterManager::get] Falló la reconexión: {:?}", e);
                            return Err(ClusterError::TcpConnectionError);
                        }
                    }
                } else {
                    println!("[ClusterManager::get] Ya se intentó reconectar, abortando.");
                    return Err(ClusterError::TcpConnectionError);
                }
            }
            break;
        }

        // Obtener la respuesta en formato hex
        let hex_result = self.get_response();

        match hex_result {
            Ok(hex_bytes) => {
                // El string hexadecimal debe ser ASCII puro, no UTF-8
                let hex_string = match std::str::from_utf8(&hex_bytes) {
                    Ok(s) => s,
                    Err(_) => {
                        println!("[ClusterManager::get] Error: respuesta no es ASCII válido");
                        return Err(ClusterError::GetInvalidData);
                    }
                };
                // Validar que solo contiene caracteres hexadecimales
                if !hex_string.chars().all(|c| c.is_ascii_hexdigit()) {
                    println!(
                        "[ClusterManager::get] Error: respuesta contiene caracteres no hexadecimales"
                    );
                    return Err(ClusterError::GetInvalidData);
                }
                // Convertir de hex a los bytes originales
                match hex_string_to_bytes(hex_string) {
                    Ok(original_bytes) => {
                        println!(
                            "\x1b[34m[ClusterManager::get] Converted back to original bytes: {} bytes\x1b[0m",
                            original_bytes.len()
                        );
                        Ok(original_bytes)
                    }
                    Err(e) => {
                        println!("[ClusterManager::get] Error converting hex to bytes: {}", e);
                        Err(ClusterError::GetInvalidData)
                    }
                }
            }
            Err(e) => {
                println!("[ClusterManager::get] get_response ERROR: {:?}", e);
                Err(e)
            }
        }
    }

    pub fn publish(&mut self, channel: &str, value: &[u8]) -> Result<(), ClusterError> {
        // Convertir bytes a string hexadecimal
        let resp = create_publish(channel, value);

        // Intento de escritura con reconexión automática
        let mut tried_reconnect = false;
        'retry: loop {
            let write_result = self.active_node.write_all(&resp);
            let flush_result = self.active_node.flush();
            if write_result.is_err() || flush_result.is_err() {
                let write_err = write_result.as_ref().err();
                let flush_err = flush_result.as_ref().err();
                println!(
                    "[ClusterManager::publish] Error writing/flushing to active_node: write={:?}, flush={:?}",
                    write_err, flush_err
                );
                if !tried_reconnect {
                    println!(
                        "[ClusterManager::publish] Intentando reconectar tras error de escritura..."
                    );
                    match connect_to_cluster(
                        self.node_address.clone(),
                        self.username.clone(),
                        self.password.clone(),
                    ) {
                        Ok((new_stream, _)) => {
                            self.active_node = new_stream;
                            tried_reconnect = true;
                            continue 'retry;
                        }
                        Err(e) => {
                            println!("[ClusterManager::publish] Falló la reconexión: {:?}", e);
                            return Err(ClusterError::TcpConnectionError);
                        }
                    }
                } else {
                    println!("[ClusterManager::publish] Ya se intentó reconectar, abortando.");
                    return Err(ClusterError::TcpConnectionError);
                }
            }
            break;
        }

        self.discard_response();

        Ok(())
    }

    fn ensure_correct_node(&mut self, key: &str) -> Result<(), ClusterError> {
        // Verificar si estamos en modo Docker (deshabilitar cluster switching)
        if std::env::var("DOCKER_MODE").unwrap_or_default() == "true" {
            println!(
                "[ClusterManager::ensure_correct_node] Docker mode enabled - skipping cluster switching for key: {}",
                key
            );
            return Ok(());
        }

        println!(
            "[ClusterManager::ensure_correct_node] Calculating hash slot for key: {}",
            key
        );
        let slot = hash_slot(key).unwrap();
        println!(
            "[ClusterManager::ensure_correct_node] Hash slot for key '{}': {}",
            key, slot
        );
        if let Some(master_node) = self.find_master_for_slot(slot) {
            let master_node_cloned = master_node.clone();
            let ip = &master_node_cloned[0];
            let port = &master_node_cloned[1];
            let address = format!("{}:{}", ip, port);
            println!(
                "[ClusterManager::ensure_correct_node] Master node for slot {}: {} (current: {})",
                slot, address, self.node_address
            );
            if self.node_address != address {
                println!(
                    "[ClusterManager::ensure_correct_node] Switching active node to: {}",
                    address
                );
                self.switch_active_node(&master_node_cloned)?;
            } else {
                println!("[ClusterManager::ensure_correct_node] Already on correct node.");
            }
            Ok(())
        } else {
            println!(
                "[ClusterManager::ensure_correct_node] Cannot find master for slot {}",
                slot
            );
            Err(ClusterError::CannotGetClusterData)
        }
    }

    fn get_response(&mut self) -> Result<Vec<u8>, ClusterError> {
        println!("[ClusterManager::get_response] Waiting for response...");
        let mut reader = BufReader::new(&self.active_node);
        if let Ok(message) = parse_resp_line(&mut reader) {
            println!(
                "[ClusterManager::get_response] Received message: {:?}",
                message
            );
            match message {
                RespMessage::Null(_) => {
                    println!("[ClusterManager::get_response] Null response (key is empty)");
                    Err(ClusterError::GetKeyIsEmpty)
                }
                RespMessage::BulkString(Some(data)) => {
                    println!(
                        "[ClusterManager::get_response] BulkString data: {:?}",
                        String::from_utf8_lossy(&data)
                    );
                    Ok(data)
                }
                _ => {
                    println!("[ClusterManager::get_response] Invalid data type in response");
                    Err(ClusterError::GetInvalidData)
                }
            }
        } else {
            println!("[ClusterManager::get_response] Invalid Redis response");
            Err(ClusterError::InvalidRedisResponse)
        }
    }

    pub fn set(&mut self, key: &str, value: &[u8]) -> Result<(), ClusterError> {
        println!("[ClusterManager::set] Called with key: {}", key);
        println!(
            "[ClusterManager::set] Original value size: {} bytes",
            value.len()
        );

        // Convertir bytes a string hexadecimal
        let hex_value = bytes_to_hex_string(value);
        println!(
            "[ClusterManager::set] Converted to hex string: {} chars",
            hex_value.len()
        );

        match self.ensure_correct_node(key) {
            Ok(_) => println!("[ClusterManager::set] ensure_correct_node OK"),
            Err(e) => {
                println!("[ClusterManager::set] ensure_correct_node ERROR: {:?}", e);
                return Err(e);
            }
        }

        // Ahora enviamos la string hexadecimal como bytes
        let hex_bytes = hex_value.as_bytes();
        let resp = create_set(key, hex_bytes);

        println!(
            "\x1b[33m[ClusterManager::set] Sending SET command for hex string (length: {})\x1b[0m",
            hex_value.len()
        );

        // Intento de escritura con reconexión automática
        let mut tried_reconnect = false;
        'retry: loop {
            let write_result = self.active_node.write_all(&resp);
            let flush_result = self.active_node.flush();
            if write_result.is_err() || flush_result.is_err() {
                let write_err = write_result.as_ref().err();
                let flush_err = flush_result.as_ref().err();
                println!(
                    "[ClusterManager::set] Error writing/flushing to active_node: write={:?}, flush={:?}",
                    write_err, flush_err
                );
                if !tried_reconnect {
                    println!(
                        "[ClusterManager::set] Intentando reconectar tras error de escritura..."
                    );
                    match connect_to_cluster(
                        self.node_address.clone(),
                        self.username.clone(),
                        self.password.clone(),
                    ) {
                        Ok((new_stream, _)) => {
                            self.active_node = new_stream;
                            tried_reconnect = true;
                            continue 'retry;
                        }
                        Err(e) => {
                            println!("[ClusterManager::set] Falló la reconexión: {:?}", e);
                            return Err(ClusterError::TcpConnectionError);
                        }
                    }
                } else {
                    println!("[ClusterManager::set] Ya se intentó reconectar, abortando.");
                    return Err(ClusterError::TcpConnectionError);
                }
            }
            break;
        }

        let result = self.set_response();
        match &result {
            Ok(_) => println!("\x1b[33m[ClusterManager::set] Value set successfully\x1b[0m"),
            Err(e) => println!("[ClusterManager::set] set_response ERROR: {:?}", e),
        }
        result
    }

    pub fn del(&mut self, key: &str) -> Result<(), ClusterError> {
        println!("[ClusterManager::del] Called with key: {}", key);

        match self.ensure_correct_node(key) {
            Ok(_) => println!("[ClusterManager::del] ensure_correct_node OK"),
            Err(e) => {
                println!("[ClusterManager::del] ensure_correct_node ERROR: {:?}", e);
                return Err(e);
            }
        }

        let resp = create_del(key);

        println!(
            "\x1b[33m[ClusterManager::del] Sending DEL command\x1b[0m"
        );

        // Intento de escritura con reconexión automática
        let mut tried_reconnect = false;
        'retry: loop {
            let write_result = self.active_node.write_all(&resp);
            let flush_result = self.active_node.flush();
            if write_result.is_err() || flush_result.is_err() {
                let write_err = write_result.as_ref().err();
                let flush_err = flush_result.as_ref().err();
                println!(
                    "[ClusterManager::del] Error writing/flushing to active_node: write={:?}, flush={:?}",
                    write_err, flush_err
                );
                if !tried_reconnect {
                    println!(
                        "[ClusterManager::del] Intentando reconectar tras error de escritura..."
                    );
                    match connect_to_cluster(
                        self.node_address.clone(),
                        self.username.clone(),
                        self.password.clone(),
                    ) {
                        Ok((new_stream, _)) => {
                            self.active_node = new_stream;
                            tried_reconnect = true;
                            continue 'retry;
                        }
                        Err(e) => {
                            println!("[ClusterManager::del] Falló la reconexión: {:?}", e);
                            return Err(ClusterError::TcpConnectionError);
                        }
                    }
                } else {
                    println!("[ClusterManager::del] Ya se intentó reconectar, abortando.");
                    return Err(ClusterError::TcpConnectionError);
                }
            }
            break;
        }

        let result = self.del_response();
        match &result {
            Ok(_) => println!("\x1b[33m[ClusterManager::del] Value deleted successfully\x1b[0m"),
            Err(e) => println!("[ClusterManager::del] del_response ERROR: {:?}", e),
        }
        result
    }

    fn del_response(&mut self) -> Result<(), ClusterError> {
        println!("[ClusterManager::del_response] Waiting for response...");
        let mut reader = BufReader::new(&self.active_node);
        if let Ok(message) = parse_resp_line(&mut reader) {
            println!(
                "[ClusterManager::del_response] Received message: {:?}",
                message
            );
            match message {
                RespMessage::Integer(_) => {
                    println!("[ClusterManager::del_response] Response: OK");
                    Ok(())
                }
                _ => {
                    println!("[ClusterManager::del_response] Invalid response type");
                    Err(ClusterError::InvalidRedisResponse)
                }
            }
        } else {
            println!("[ClusterManager::del_response] Invalid Redis response");
            Err(ClusterError::InvalidRedisResponse)
        }
    }

    pub fn subscribe(&mut self, channel: &str) -> Result<TcpStream, ClusterError> {
        let address = self.node_address.clone();
        println!("[ClusterManager::subscribe] Conectando para suscripción a: {}", address);
        let (mut stream, _) =
            connect_to_cluster(address, self.username.clone(), self.password.clone()).unwrap();

        println!("[ClusterManager::subscribe] Suscribiéndose al canal: {}", channel);
        let resp_message = create_subscribe(channel);
        stream.write_all(&resp_message).unwrap();

        // Descartar la respuesta inicial del SUBSCRIBE
        let mut reader = BufReader::new(&stream);
        let _ = parse_resp_line(&mut reader);

        println!("[ClusterManager::subscribe] Suscripción completada para canal: {}", channel);
        Ok(stream)
    }

    fn fill_cluster(&mut self) -> Result<(), ClusterError> {
        self.active_node.write_all(&create_cluster_slot()).unwrap();

        let cluster_data: HashMap<HashRange, Vec<NodeData>> = self.cluster_slots_response()?;

        self.cluster_data = cluster_data;

        Ok(())
    }

    fn discard_response(&mut self) {
        let mut reader = BufReader::new(&self.active_node);
        let _ = parse_resp_line(&mut reader);
    }

    fn set_response(&mut self) -> Result<(), ClusterError> {
        println!("[ClusterManager::set_response] Waiting for response...");
        let mut reader = BufReader::new(&self.active_node);
        if let Ok(message) = parse_resp_line(&mut reader) {
            println!(
                "[ClusterManager::set_response] Received message: {:?}",
                message
            );
            match message {
                RespMessage::SimpleString(_) => {
                    println!("[ClusterManager::set_response] Response: OK");
                    Ok(())
                }
                _ => {
                    println!("[ClusterManager::set_response] Invalid response type");
                    Err(ClusterError::InvalidRedisResponse)
                }
            }
        } else {
            println!("[ClusterManager::set_response] Invalid Redis response");
            Err(ClusterError::InvalidRedisResponse)
        }
    }

    fn find_master_for_slot(&self, slot: u16) -> Option<&NodeData> {
        for ((start, end), nodes) in &self.cluster_data {
            if slot >= *start && slot <= *end {
                // The first node in the list is always the master node
                if let Some(master_node) = nodes.get(0) {
                    return Some(master_node);
                }
            }
        }
        None
    }
    fn switch_active_node(&mut self, node: &NodeData) -> Result<(), ClusterError> {
        let ip = &node[0];
        let port = &node[1];
        let address = format!("{}:{}", ip, port);
        println!(
            "[ClusterManager::switch_active_node] Switching to node at address: {}",
            address
        );
        match connect_to_cluster(
            address.clone(),
            self.username.clone(),
            self.password.clone(),
        ) {
            Ok((stream, _)) => {
                self.active_node = stream;
                self.node_address = address;
                self.fill_cluster()?; // Refresca la info del cluster si querés
                Ok(())
            }
            Err(e) => {
                println!(
                    "[ClusterManager::switch_active_node] Error connecting to {}: {:?}",
                    address, e
                );
                Err(ClusterError::TcpConnectionError)
            }
        }
    }

    fn cluster_slots_response(
        &mut self,
    ) -> Result<HashMap<SlotRange, Vec<Vec<String>>>, ClusterError> {
        let mut reader = BufReader::new(&self.active_node);
        if let Ok(message) = parse_resp_line(&mut reader) {
            match array_to_vec(message) {
                Some(vec) => Ok(parse_flat_cluster_slots(&vec)),
                _ => Err(ClusterError::CannotGetClusterData),
            }
        } else {
            Err(ClusterError::CannotGetClusterData)
        }
    }
}

fn array_to_vec(array: RespMessage) -> Option<Vec<String>> {
    match array {
        RespMessage::Array(vec) => {
            let mut vec_string: Vec<String> = Vec::new();
            for message in vec {
                if let RespMessage::BulkString(Some(content)) = message {
                    vec_string.push(String::from_utf8_lossy(&content).to_string());
                }
            }
            Some(vec_string)
        }
        _ => None,
    }
}

fn create_subscribe(channel: &str) -> Vec<u8> {
    let mut resp: Vec<u8> = Vec::new();

    resp.extend_from_slice(b"*2\r\n");
    resp.extend_from_slice(b"$9\r\nSUBSCRIBE\r\n");
    resp.extend_from_slice(format!("${}\r\n", channel.len()).as_bytes());
    resp.extend_from_slice(channel.as_bytes());
    resp.extend_from_slice(b"\r\n");

    resp
}

fn create_publish(channel: &str, argument: &[u8]) -> Vec<u8> {
    let mut resp = Vec::new();

    resp.extend_from_slice(b"*3\r\n");
    resp.extend_from_slice(b"$7\r\nPUBLISH\r\n");
    resp.extend_from_slice(format!("${}\r\n", channel.len()).as_bytes());
    resp.extend_from_slice(channel.as_bytes());
    resp.extend_from_slice(b"\r\n");
    resp.extend_from_slice(format!("${}\r\n", argument.len()).as_bytes());
    resp.extend_from_slice(&argument);
    resp.extend_from_slice(b"\r\n");

    resp
}

fn create_set(key: &str, argument: &[u8]) -> Vec<u8> {
    let mut resp: Vec<u8> = Vec::new();

    resp.extend_from_slice(b"*3\r\n");
    resp.extend_from_slice(b"$3\r\nSET\r\n");
    resp.extend_from_slice(format!("${}\r\n", key.len()).as_bytes());
    resp.extend_from_slice(key.as_bytes());
    resp.extend_from_slice(b"\r\n");
    resp.extend_from_slice(format!("${}\r\n", argument.len()).as_bytes());
    resp.extend_from_slice(&argument);
    resp.extend_from_slice(b"\r\n");

    resp
}

fn create_del(key: &str) -> Vec<u8> {
    let mut resp: Vec<u8> = Vec::new();

    resp.extend_from_slice(b"*2\r\n");
    resp.extend_from_slice(b"$3\r\nDEL\r\n");
    resp.extend_from_slice(format!("${}\r\n", key.len()).as_bytes());
    resp.extend_from_slice(key.as_bytes());
    resp.extend_from_slice(b"\r\n");

    resp
}

fn create_get(key: &str) -> Vec<u8> {
    let mut resp: Vec<u8> = Vec::new();

    resp.extend_from_slice(b"*2\r\n");
    resp.extend_from_slice(b"$3\r\nGET\r\n");
    resp.extend_from_slice(format!("${}\r\n", key.len()).as_bytes());
    resp.extend_from_slice(key.as_bytes());
    resp.extend_from_slice(b"\r\n");

    resp
}

fn create_cluster_slot() -> Vec<u8> {
    let mut resp: Vec<u8> = Vec::new();

    resp.extend_from_slice(b"*2\r\n");
    resp.extend_from_slice(b"$7\r\nCLUSTER\r\n");
    resp.extend_from_slice(b"$5\r\nSLOTS\r\n");

    resp
}
