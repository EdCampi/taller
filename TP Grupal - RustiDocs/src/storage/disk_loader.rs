//! Carga de datos desde disco.

// IMPORTS
use crate::config::node_configs::NodeConfigs;
use crate::logs::aof_logger::AofLogger;
use crate::storage::DataStore;
use crate::storage::deserializer::deserialize_db;
use std::io;
use std::sync::{Arc, RwLock};
// FUNCIONES

/// DiskLoader, estructura encargada de recuperar estado inicial de la base
/// de datos al arrancar a partir de un archivo en disco.
pub struct DiskLoader {
    // Path del archivo del cual cargar.
    source: String,
    logger: Arc<AofLogger>,
}

impl DiskLoader {
    pub fn new(settings: NodeConfigs, logger: Arc<AofLogger>) -> Self {
        DiskLoader {
            source: settings.get_snapshot_dst(),
            logger,
        }
    }

    /// Método para cargar el estado inicial de la base de datos
    /// a partir de un archivo en disco.
    ///
    /// # Returns
    /// * `Arc<RwLock<DataStore>>` Base de datos lista para su uso.
    pub fn load(&self) -> Result<Arc<RwLock<DataStore>>, io::Error> {
        self.logger
            .log_event(format!("Starting DB retrieve from {}", self.source));
        let _ = if let Ok(metadata) = std::fs::metadata(&self.source) {
            if metadata.len() == 0 {
                self.logger
                    .log_event(format!("No data was retrieved from {}", self.source));
                return Ok(Arc::new(RwLock::new(DataStore::new())));
            }
            let ds = Arc::new(RwLock::new(deserialize_db(self.source.to_string())?));
            let ds_length = ds.read().unwrap().len();
            self.logger.log_event(format!(
                "DB retrieve from {} finished with {} items",
                self.source, ds_length
            ));
            return Ok(ds);
        };
        self.logger
            .log_event("No DB backup was found, starting with blank ds".to_string());
        Ok(Arc::new(RwLock::new(DataStore::new())))
    }
}
