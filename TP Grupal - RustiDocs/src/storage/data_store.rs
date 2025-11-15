use crate::cluster::utils::{read_string_from_buffer, read_u32_from_buffer, read_u64_from_buffer};
use std::collections::{HashMap, HashSet};
use std::io::Read;

#[derive(Debug, Clone)]
pub struct DataStore {
    pub string_db: HashMap<String, String>,
    pub list_db: HashMap<String, Vec<String>>,
    pub set_db: HashMap<String, HashSet<String>>,
}

impl DataStore {
    pub fn new() -> Self {
        DataStore {
            string_db: HashMap::new(),
            list_db: HashMap::new(),
            set_db: HashMap::new(),
        }
    }

    // Métodos para manipular la base de datos
    pub fn set(&mut self, key: String, value: String) {
        self.string_db.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.string_db.get(key)
    }

    pub fn len(&self) -> usize {
        self.string_db.len() + self.list_db.len() + self.set_db.len()
    }

    pub fn update(&mut self, data_store: DataStore) {
        self.string_db = data_store.string_db;
        self.list_db = data_store.list_db;
        self.set_db = data_store.set_db;
    }

    pub(crate) fn sync_database<T: Clone>(
        master_db: &HashMap<String, T>,
        updated_db: &mut HashMap<String, T>,
    ) {
        for (key, value) in master_db {
            updated_db.insert(key.clone(), value.clone());
        }

        let keys_to_remove: Vec<_> = updated_db
            .keys()
            .filter(|key| !master_db.contains_key(*key))
            .cloned()
            .collect();

        for key in keys_to_remove {
            updated_db.remove(&key);
        }
    }

    pub fn from_bytes<R: Read>(buffer: &mut R) -> Result<Self, String> {
        let mut string_db = HashMap::new();

        let string_db_len = read_u64_from_buffer(buffer)?;
        for _ in 0..string_db_len {
            let read_key_len = read_u32_from_buffer(buffer)?;
            let key = read_string_from_buffer(buffer, read_key_len as usize)?;

            let read_value_len = read_u64_from_buffer(buffer)?;
            let value = read_string_from_buffer(buffer, read_value_len as usize)?;

            string_db.insert(key, value);
        }

        let mut list_db = HashMap::new();
        let list_db_len = read_u64_from_buffer(buffer)?;
        for _ in 0..list_db_len {
            let read_key_len = read_u32_from_buffer(buffer)?;
            let key = read_string_from_buffer(buffer, read_key_len as usize)?;

            let mut list = Vec::new();
            let list_len = read_u64_from_buffer(buffer)?;
            for _ in 0..list_len {
                let read_list_item_len = read_u64_from_buffer(buffer)?;
                let list_item = read_string_from_buffer(buffer, read_list_item_len as usize)?;
                list.push(list_item);
            }
            list_db.insert(key, list);
        }

        let mut set_db = HashMap::new();
        let set_db_len = read_u64_from_buffer(buffer)?;
        for _ in 0..set_db_len {
            let read_key_len = read_u32_from_buffer(buffer)?;
            let key = read_string_from_buffer(buffer, read_key_len as usize)?;

            let mut set = HashSet::new();
            let set_len = read_u64_from_buffer(buffer)?;
            for _ in 0..set_len {
                let read_set_item_len = read_u32_from_buffer(buffer)?;
                let set_item = read_string_from_buffer(buffer, read_set_item_len as usize)?;
                set.insert(set_item);
            }
            set_db.insert(key, set);
        }

        Ok(DataStore {
            string_db,
            list_db,
            set_db,
        })
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&(self.string_db.len() as u64).to_be_bytes());
        for (key, value) in &self.string_db {
            let key_bytes = key.as_bytes();
            bytes.extend_from_slice(&(key_bytes.len() as u32).to_be_bytes());
            bytes.extend_from_slice(key_bytes);

            let value_bytes = value.as_bytes();
            bytes.extend_from_slice(&(value_bytes.len() as u64).to_be_bytes());
            bytes.extend_from_slice(value_bytes);
        }

        bytes.extend_from_slice(&(self.list_db.len() as u64).to_be_bytes());
        for (key, list) in &self.list_db {
            let key_bytes = key.as_bytes();
            bytes.extend_from_slice(&(key_bytes.len() as u32).to_be_bytes());
            bytes.extend_from_slice(key_bytes);

            bytes.extend_from_slice(&(list.len() as u64).to_be_bytes());
            for item in list {
                let list_item_bytes = item.as_bytes();
                bytes.extend_from_slice(&(list_item_bytes.len() as u64).to_be_bytes());
                bytes.extend_from_slice(list_item_bytes);
            }
        }

        bytes.extend_from_slice(&(self.set_db.len() as u64).to_be_bytes());
        for (key, set) in &self.set_db {
            let key_bytes = key.as_bytes();
            bytes.extend_from_slice(&(key_bytes.len() as u32).to_be_bytes());
            bytes.extend_from_slice(key_bytes);

            bytes.extend_from_slice(&(set.len() as u64).to_be_bytes());
            for item in set {
                let set_item = item.as_bytes();
                bytes.extend_from_slice(&(set_item.len() as u32).to_be_bytes());
                bytes.extend_from_slice(set_item);
            }
        }

        bytes
    }
}
