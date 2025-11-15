//! Dumps de la base de datos y funciones relacionadas.

// IMPORTS
use crate::config::node_configs::NodeConfigs;
use crate::logs::aof_logger::AofLogger;
use crate::storage::DataStore;
use crate::storage::serializer::serialize_ds;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
// CÓDIGO

/// SnapshotManager escribe/lee dumps periódicos.
/// La idea es que, por cada intervalo de tiempo, se guarde el estado actual del DataStore.
pub struct SnapshotManager {
    interval: Duration,
    datastore: Arc<RwLock<DataStore>>,
    logger: Arc<AofLogger>,
    dst: String,
}

impl SnapshotManager {
    pub fn new(
        datastore: Arc<RwLock<DataStore>>,
        settings: NodeConfigs,
        logger: Arc<AofLogger>,
    ) -> Self {
        SnapshotManager {
            interval: Duration::from_secs(settings.get_snapshot_interval()),
            datastore,
            logger,
            dst: settings.get_snapshot_dst(),
        }
    }

    /// Función que inicia el proceso de snapshot en un nuevo hilo.
    pub fn start(&mut self) {
        let interval = self.interval;
        let aux = self.datastore.clone();
        let logger = self.logger.clone();
        let dst = self.dst.clone();
        let _ = thread::Builder::new()
            .name("Snapshot manager".to_string())
            .spawn(move || {
                loop {
                    thread::sleep(interval);
                    let guard = aux
                        .read()
                        .map_err(|e| {
                            logger.log_error(format!("ERROR when trying to read for dumping {}", e))
                        })
                        .unwrap();
                    create_dump(&guard, &dst).unwrap(); // TODO: nodo_1 paniqueo
                    logger.log_notice("DB saved on disk".to_string())
                }
            });
    }
}

/// Función para crear un dump del DataStore en el directorio especificado.
/// El archivo tendrá la estructura del `DataStore` serializada en bytes, con el siguiente orden:
///
/// 1. `string_db`:
///     - Longitud de `string_db`, seguido de iteración sobre el HashMap guardando longitudes y claves/valores.
/// 2. `list_db`:
///     - Longitud de `list_db`, luego claves con sus longitudes y valores como vectores de strings
///     cada uno con su longitud y contenido.
/// 3. `list_db`:
///     - Proceso análogo al anterior.
///
/// NOTA: Antes de un dato o conjunto, **siempre está su longitud**.
pub(crate) fn create_dump(ds: &DataStore, path: &String) -> Result<(), std::io::Error> {
    let mut file = std::fs::File::create(path)?;
    serialize_ds(&ds, &mut file)?;
    Ok(())
}
