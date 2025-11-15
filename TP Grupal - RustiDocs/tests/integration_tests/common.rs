//! Utilidades comunes para los tests de integración

use rustidocs::{
    network::resp_message::RespMessage,
    network::resp_parser::{RespParserError, parse_resp_line},
    storage::DataStore,
};
use std::io::{BufReader, Cursor};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

/// Cliente de prueba para simular conexiones a Redis
#[allow(dead_code)]
pub struct TestClient {
    pub id: String,
    pub connected: bool,
}

#[allow(dead_code)]
impl TestClient {
    pub fn new(id: String) -> Self {
        Self {
            id,
            connected: true,
        }
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

/// Servidor de prueba para simular un servidor Redis
#[allow(dead_code)]
pub struct TestServer {
    pub port: u16,
    pub clients: Vec<TestClient>,
    pub store: Arc<DataStore>,
}

#[allow(dead_code)]
impl TestServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            clients: Vec::new(),
            store: Arc::new(DataStore::new()),
        }
    }

    pub fn add_client(&mut self, client: TestClient) {
        self.clients.push(client);
    }

    pub fn remove_client(&mut self, client_id: &str) {
        self.clients.retain(|c| c.id != client_id);
    }

    pub fn client_count(&self) -> usize {
        self.clients.len()
    }
}

/// Utilidades para tests de concurrencia
pub mod concurrency {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    /// Ejecuta múltiples clientes concurrentemente
    #[allow(dead_code)]
    pub fn run_concurrent_clients<F>(num_clients: usize, test_fn: F) -> Result<(), String>
    where
        F: Fn(usize) -> Result<(), String> + Send + Sync + Clone + 'static,
    {
        let mut handles = vec![];

        for i in 0..num_clients {
            let test_fn = test_fn.clone();
            let handle = thread::spawn(move || test_fn(i));
            handles.push(handle);
        }

        for handle in handles {
            match handle.join() {
                Ok(Ok(())) => (),
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err("Thread panicked".to_string()),
            }
        }

        Ok(())
    }

    /// Ejecuta operaciones concurrentes en un store compartido
    #[allow(dead_code)]
    pub fn run_concurrent_operations<F>(num_operations: usize, test_fn: F) -> Result<(), String>
    where
        F: Fn(usize, Arc<DataStore>) -> Result<(), String> + Send + Sync + Clone + 'static,
    {
        let store = Arc::new(DataStore::new());
        let mut handles = vec![];

        for i in 0..num_operations {
            let test_fn = test_fn.clone();
            let store = store.clone();
            let handle = thread::spawn(move || test_fn(i, store));
            handles.push(handle);
        }

        for handle in handles {
            match handle.join() {
                Ok(Ok(())) => (),
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err("Thread panicked".to_string()),
            }
        }

        Ok(())
    }
}

/// Utilidades para tests de rendimiento
pub mod performance {
    use super::*;
    use std::time::Instant;

    /// Mide el tiempo de ejecución de una función
    #[allow(dead_code)]
    pub fn measure_time<F, T>(func: F) -> (T, Duration)
    where
        F: FnOnce() -> T,
    {
        let start = Instant::now();
        let result = func();
        let duration = start.elapsed();
        (result, duration)
    }

    /// Ejecuta una función múltiples veces y mide el rendimiento promedio
    #[allow(dead_code)]
    pub fn benchmark<F>(iterations: usize, func: F) -> Duration
    where
        F: Fn() + Copy,
    {
        let start = Instant::now();

        for _ in 0..iterations {
            func();
        }

        start.elapsed()
    }

    /// Verifica que una operación se ejecuta dentro de un límite de tiempo
    #[allow(dead_code)]
    pub fn assert_performance<F>(max_duration: Duration, func: F)
    where
        F: FnOnce(),
    {
        let (_, duration) = measure_time(func);
        assert!(
            duration <= max_duration,
            "Operation took {:?}, expected <= {:?}",
            duration,
            max_duration
        );
    }
}

/// Utilidades para tests de persistencia
pub mod persistence {
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    /// Verifica si un archivo existe
    #[allow(dead_code)]
    pub fn file_exists(path: &str) -> bool {
        Path::new(path).exists()
    }

    /// Obtiene el tamaño de un archivo
    #[allow(dead_code)]
    pub fn file_size(path: &str) -> Result<u64, std::io::Error> {
        let metadata = fs::metadata(path)?;
        Ok(metadata.len())
    }

    /// Lee el contenido de un archivo
    #[allow(dead_code)]
    pub fn read_file(path: &str) -> Result<String, std::io::Error> {
        fs::read_to_string(path)
    }

    /// Escribe contenido a un archivo
    #[allow(dead_code)]
    pub fn write_file(path: &str, content: &str) -> Result<(), std::io::Error> {
        fs::write(path, content)
    }

    /// Crea un directorio temporal para tests
    #[allow(dead_code)]
    pub fn create_temp_dir() -> Result<TempDir, std::io::Error> {
        TempDir::new().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }
}

/// Utilidades para tests de red
pub mod network {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

    /// Crea un listener TCP en un puerto específico
    #[allow(dead_code)]
    pub fn create_listener(port: u16) -> Result<TcpListener, std::io::Error> {
        TcpListener::bind(format!("0.0.0.0:{}", port))
    }

    /// Conecta a un servidor TCP
    #[allow(dead_code)]
    pub fn connect_to_server(port: u16) -> Result<TcpStream, std::io::Error> {
        TcpStream::connect(format!("0.0.0.0:{}", port))
    }

    /// Envía datos a través de una conexión TCP
    #[allow(dead_code)]
    pub fn send_data(stream: &mut TcpStream, data: &[u8]) -> Result<(), std::io::Error> {
        stream.write_all(data)
    }

    /// Recibe datos de una conexión TCP
    #[allow(dead_code)]
    pub fn receive_data(
        stream: &mut TcpStream,
        buffer: &mut [u8],
    ) -> Result<usize, std::io::Error> {
        stream.read(buffer)
    }

    /// Verifica si un puerto está disponible
    #[allow(dead_code)]
    pub fn is_port_available(port: u16) -> bool {
        TcpListener::bind(format!("0.0.0.0:{}", port)).is_ok()
    }
}

/// Configuración de test común
#[allow(dead_code)]
pub struct TestCommonConfig {
    pub temp_dir: TempDir,
    pub server_port: u16,
    pub timeout: Duration,
}

#[allow(dead_code)]
impl TestCommonConfig {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = persistence::create_temp_dir()?;

        Ok(Self {
            temp_dir,
            server_port: 6379,
            timeout: Duration::from_secs(5),
        })
    }

    /// Crea un archivo de configuración temporal
    pub fn create_config_file(&self) -> Result<String, Box<dyn std::error::Error>> {
        let config_path = self.temp_dir.path().join("test.conf");
        let config_content = "bind 0.0.0.0\nport 6379\nrole M\nmaxclients 1000\nsave 900 15\ndbfilename dump.rdb\ndir ./\nlogfile test.log\nloglevel notice\nnode-id test123\nhash-slots 0-16383";

        persistence::write_file(config_path.to_string_lossy().as_ref(), config_content)?;
        Ok(config_path.to_string_lossy().to_string())
    }
}

/// Función helper para parsear RESP desde bytes
pub fn parse_resp_from_bytes(bytes: &[u8]) -> Result<RespMessage, RespParserError> {
    let mut reader = BufReader::new(Cursor::new(bytes));
    parse_resp_line(&mut reader)
}

/// Función helper para serializar RespMessage a bytes
pub fn serialize_resp_to_bytes(message: &RespMessage) -> Vec<u8> {
    message.as_bytes()
}
