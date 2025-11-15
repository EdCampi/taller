use crate::cluster::comms::node_input::NODAL_COMMS_PORT;
use crate::cluster::types::SlotRange;
use rand::RngCore;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::net::SocketAddr;

#[derive(Clone, Debug)]
pub struct NodeConfigs {
    ip: String,
    port: String,
    initial_role: String,
    clients_limit: i64,
    snapshot_interval: i64,
    snapshot_k_changes: i64,
    snapshot_file: String,
    snapshot_path: String,
    log_file: String,
    log_level: String,
    node_id: String,
    initial_slots_range: SlotRange,
}

impl NodeConfigs {
    /// Parsea el archivo .conf y vuelca las propiedades
    /// iniciales del nodo en la instancia de NodeSettings.
    pub fn new(file_path: &str) -> Result<Self, std::io::Error> {
        let config_file = File::open(file_path)?;
        let reader = BufReader::new(config_file);

        // Default values
        let mut ip = String::new();
        let mut port = String::new();
        let mut role = "M".to_string();
        let mut clients_limit = 1000;
        let mut snapshot_interval = 900;
        let mut snapshot_k_changes = 15;
        let mut snapshot_file = "dump.rdb".to_string();
        let mut snapshot_path = "./".to_string();
        let mut log_file = "redis.log".to_string();
        let mut log_level = "notice".to_string();
        let mut node_id: Option<String> = None;
        let mut slots_range: SlotRange = (0, 0);

        let mut lines: Vec<String> = vec![];
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim().to_string();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                lines.push(trimmed.clone());
                continue;
            }

            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() < 2 {
                lines.push(trimmed.clone());
                continue;
            }

            match parts[0] {
                "bind" => ip = parts[1].to_string(),
                "port" => port = parts[1].to_string(),
                "role" => role = parts[1].to_string(),
                "maxclients" => clients_limit = parts[1].parse().unwrap_or(clients_limit),
                "save" => {
                    if parts.len() >= 3 {
                        snapshot_interval = parts[1].parse().unwrap_or(snapshot_interval);
                        snapshot_k_changes = parts[2].parse().unwrap_or(snapshot_k_changes);
                    }
                }
                "dbfilename" => snapshot_file = parts[1].to_string(),
                "dir" => snapshot_path = parts[1].to_string(),
                "logfile" => log_file = parts[1].to_string(),
                "loglevel" => log_level = parts[1].to_string(),
                "node-id" => node_id = Some(parts[1].to_string()),
                "hash-slots" => {
                    let ranges: Vec<&str> = parts[1..].to_vec();
                    for range in ranges {
                        if let Some((start, end)) = range.split_once('-') {
                            let start: u16 = start.trim().parse().unwrap_or(0);
                            let end: u16 = end.trim().parse().unwrap_or(0);
                            slots_range = (start, end);
                        } else if let Ok(slot) = range.trim().parse() {
                            slots_range = (slot, slot);
                        }
                    }
                }
                _ => {}
            }

            lines.push(trimmed);
        }

        if node_id.is_none() {
            let new_id = random_32bit_id();
            lines.push(format!("node-id {}", new_id));
            node_id = Some(new_id);

            let mut file = OpenOptions::new()
                .write(true)
                .append(false)
                .truncate(true)
                .open(file_path)?;
            for line in lines {
                writeln!(file, "{}", line)?;
            }
        }

        if ip.is_empty() || port.is_empty() {
            panic!("Faltan 'bind' o 'port' en la configuración.");
        }

        Ok(Self {
            ip,
            port,
            initial_role: role,
            clients_limit,
            snapshot_interval,
            snapshot_k_changes,
            snapshot_file,
            snapshot_path,
            log_file,
            log_level,
            node_id: node_id.unwrap(),
            initial_slots_range: slots_range,
        })
    }

    pub fn get_addr(&self) -> SocketAddr {
        (self.ip.clone() + ":" + &self.port.clone())
            .parse()
            .unwrap()
    }

    pub fn get_id(&self) -> String {
        self.node_id.clone()
    }

    pub fn get_role(&self) -> String {
        self.initial_role.clone()
    }

    pub fn get_clients_limit(&self) -> i64 {
        self.clients_limit
    }

    pub fn get_snapshot_data(&self) -> SnapshotData {
        let path = self.snapshot_path.clone() + &self.snapshot_file.clone();
        SnapshotData::new(
            path,
            self.snapshot_interval as u64,
            self.snapshot_k_changes as u64,
        )
    }

    pub fn get_snapshot_dst(&self) -> String {
        self.snapshot_path.clone() + &self.snapshot_file.clone()
    }

    pub fn get_snapshot_interval(&self) -> u64 {
        self.snapshot_interval as u64
    }

    pub fn get_snapshot_k_changes(&self) -> u64 {
        self.snapshot_k_changes as u64
    }

    pub fn get_log_dst(&self) -> String {
        self.log_file.clone()
    }

    pub fn get_log_level(&self) -> String {
        self.log_level.clone()
    }

    pub fn get_node_port(&self) -> u16 {
        let aux = self.port.parse::<usize>().unwrap_or(0);
        aux as u16 + NODAL_COMMS_PORT
    }

    pub fn get_node_ip(&self) -> String {
        self.ip.clone()
    }

    pub fn get_hash_slots(&self) -> SlotRange {
        self.initial_slots_range.clone()
    }

    pub fn set_hash_slots(&mut self, slots: SlotRange) {
        self.initial_slots_range = slots;
    }

    pub fn owns_slot(&self, slot: u16) -> bool {
        if slot > self.initial_slots_range.0 && slot < self.initial_slots_range.1 {
            return true;
        };
        false
    }
}

#[derive(Clone)]
pub struct SnapshotData {
    pub path: String,
    pub interval: u64,
    pub k_changes: u64,
}

impl SnapshotData {
    fn new(path: String, interval: u64, k_changes: u64) -> Self {
        Self {
            path,
            interval,
            k_changes,
        }
    }
}

pub fn random_32bit_id() -> String {
    let id: u32 = RngCore::next_u32(&mut rand::thread_rng());
    id.to_string()
}
