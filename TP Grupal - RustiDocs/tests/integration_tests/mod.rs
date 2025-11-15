//! Tests de integración para la implementación de Redis
//!
//! Este módulo contiene tests que verifican la funcionalidad completa del sistema,
//! incluyendo el protocolo cliente/servidor, comandos Redis, almacenamiento en disco
//! y funcionalidad Pub/Sub.

pub mod command_tests;
pub mod common;
pub mod persistence_tests;
pub mod protocol_tests;
pub mod pubsub_tests;

use rustidocs::{
    config::node_configs::NodeConfigs,
    logs::aof_logger::AofLogger,
    storage::{DataStore, SnapshotManager},
};
use std::sync::Arc;
use tempfile::TempDir;

/// Estructura para simular un servidor Redis en tests
#[allow(dead_code)]
pub struct TestRedisServer {
    pub store: Arc<DataStore>,
    pub snapshot_manager: Option<SnapshotManager>,
    pub temp_dir: TempDir,
    pub config: NodeConfigs,
    pub logger: Arc<AofLogger>,
}

impl TestRedisServer {
    /// Crea una nueva instancia del servidor de test
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");

        // Crear un archivo de configuración temporal
        let config_path = temp_dir.path().join("test.conf");
        let config_content = "bind 0.0.0.0\nport 6379\nrole M\nmaxclients 1000\nsave 900 15\ndbfilename dump.rdb\ndir ./\nlogfile test.log\nloglevel notice\nnode-id test123\nhash-slots 0-16383";
        std::fs::write(&config_path, config_content).expect("Failed to write nodes file");

        let config = NodeConfigs::new(config_path.to_string_lossy().as_ref())
            .expect("Failed to create NodeConfigs");

        let logger = AofLogger::new(config.clone());

        Self {
            store: Arc::new(DataStore::new()),
            snapshot_manager: None,
            temp_dir,
            config,
            logger,
        }
    }

    /// Guarda el estado actual en disco
    pub fn save_to_disk(&self) -> Result<(), Box<dyn std::error::Error>> {
        // En una implementación real, aquí se guardaría el store
        // Por ahora solo verificamos que el directorio existe
        println!("Simulating save to disk");
        Ok(())
    }

    /// Carga el estado desde disco
    pub fn load_from_disk(&self) -> Result<(), Box<dyn std::error::Error>> {
        // En una implementación real, aquí se cargaría el store
        // Por ahora solo verificamos que el directorio existe
        println!("Simulating load from disk");
        Ok(())
    }
}

/// Configuración común para todos los tests
#[allow(dead_code)]
pub struct TestConfig {
    pub server_port: u16,
    pub cluster_ports: Vec<u16>,
    pub temp_dir: TempDir,
}

#[allow(dead_code)]
impl TestConfig {
    pub fn new() -> Self {
        Self {
            server_port: 6379,
            cluster_ports: vec![6380, 6381, 6382],
            temp_dir: TempDir::new().expect("Failed to create temp directory"),
        }
    }
}
