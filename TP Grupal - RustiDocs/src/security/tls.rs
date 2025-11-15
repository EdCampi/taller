//! Implementación de TLS/SSL para conexiones seguras
//! 
//! Este módulo proporciona wrappers para TcpStream que implementan
//! encriptación TLS/SSL para proteger las comunicaciones.

use std::io::{Read, Write, Error as IoError, BufRead, BufReader};
use std::net::TcpStream;
use rustls::{ClientConfig, ServerConfig, OwnedTrustAnchor, RootCertStore, Certificate, ClientConnection, ServerConnection};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

/// Error que puede ocurrir durante operaciones TLS
#[derive(Debug, thiserror::Error)]
pub enum TlsError {
    #[error("Error de I/O: {0}")]
    Io(#[from] IoError),
    #[error("Error de configuración TLS: {0}")]
    Config(String),
    #[error("Error al cargar certificados: {0}")]
    Certificate(String),
    #[error("Error de handshake TLS: {0}")]
    Handshake(String),
}

/// Configuración TLS para el cliente
pub struct TlsClientConfig {
    config: Arc<ClientConfig>,
}

impl TlsClientConfig {
    /// Crea una nueva configuración TLS para cliente
    pub fn new() -> Result<Self, TlsError> {
        let mut root_store = RootCertStore::empty();
        root_store.add_trust_anchors(
            webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
                OwnedTrustAnchor::from_subject_spki_name_constraints(
                    ta.subject,
                    ta.spki,
                    ta.name_constraints,
                )
            })
        );

        let config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        Ok(Self {
            config: Arc::new(config),
        })
    }

    /// Crea una configuración TLS con certificados personalizados
    pub fn with_custom_certs(
        ca_cert_path: &Path,
        client_cert_path: Option<&Path>,
        client_key_path: Option<&Path>,
    ) -> Result<Self, TlsError> {
        let mut root_store = RootCertStore::empty();
        
        // Cargar certificado CA
        let mut ca_file = File::open(ca_cert_path)
            .map_err(|e| TlsError::Certificate(format!("No se pudo abrir CA cert: {}", e)))?;
        let ca_certs = certs(&mut BufReader::new(&mut ca_file))
            .map_err(|e| TlsError::Certificate(format!("Error parseando CA cert: {}", e)))?;
        
        for cert in ca_certs {
            root_store.add(&Certificate(cert))
                .map_err(|e| TlsError::Certificate(format!("Error agregando CA cert: {}", e)))?;
        }

        let mut config_builder = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store);

        // Si se proporcionan certificados de cliente, configurar autenticación mutua
        if let (Some(cert_path), Some(key_path)) = (client_cert_path, client_key_path) {
            let mut cert_file = File::open(cert_path)
                .map_err(|e| TlsError::Certificate(format!("No se pudo abrir client cert: {}", e)))?;
            let cert_chain = certs(&mut BufReader::new(&mut cert_file))
                .map_err(|e| TlsError::Certificate(format!("Error parseando client cert: {}", e)))?;

            let mut key_file = File::open(key_path)
                .map_err(|e| TlsError::Certificate(format!("No se pudo abrir client key: {}", e)))?;
            let mut keys = pkcs8_private_keys(&mut BufReader::new(&mut key_file))
                .map_err(|e| TlsError::Certificate(format!("Error parseando client key: {}", e)))?;

            if keys.is_empty() {
                return Err(TlsError::Certificate("No se encontraron claves privadas".to_string()));
            }

            let key = rustls::PrivateKey(keys.remove(0));
            let cert_chain: Vec<Certificate> = cert_chain.into_iter().map(Certificate).collect();
            config_builder = config_builder.with_single_cert(cert_chain, key)
                .map_err(|e| TlsError::Certificate(format!("Error configurando certificado cliente: {}", e)))?;
        }

        Ok(Self {
            config: Arc::new(config_builder),
        })
    }

    /// Establece una conexión TLS con un servidor
    pub fn connect(&self, stream: TcpStream, server_name: &str) -> Result<TlsStream, TlsError> {
        let server_name = server_name.try_into()
            .map_err(|_| TlsError::Config("Nombre de servidor inválido".to_string()))?;
        
        let client_conn = ClientConnection::new(self.config.clone(), server_name)
            .map_err(|e| TlsError::Handshake(format!("Error creando conexión cliente: {}", e)))?;

        Ok(TlsStream::Client(TlsClientStream {
            stream,
            conn: client_conn,
        }))
    }
}

/// Configuración TLS para el servidor
pub struct TlsServerConfig {
    config: Arc<ServerConfig>,
}

impl TlsServerConfig {
    /// Crea una nueva configuración TLS para servidor
    pub fn new(cert_path: &Path, key_path: &Path) -> Result<Self, TlsError> {
        let mut cert_file = File::open(cert_path)
            .map_err(|e| TlsError::Certificate(format!("No se pudo abrir cert: {}", e)))?;
        let cert_chain = certs(&mut BufReader::new(&mut cert_file))
            .map_err(|e| TlsError::Certificate(format!("Error parseando cert: {}", e)))?;

        let mut key_file = File::open(key_path)
            .map_err(|e| TlsError::Certificate(format!("No se pudo abrir key: {}", e)))?;
        let mut keys = pkcs8_private_keys(&mut BufReader::new(&mut key_file))
            .map_err(|e| TlsError::Certificate(format!("Error parseando key: {}", e)))?;

        if keys.is_empty() {
            return Err(TlsError::Certificate("No se encontraron claves privadas".to_string()));
        }

        let key = rustls::PrivateKey(keys.remove(0));
        let cert_chain: Vec<Certificate> = cert_chain.into_iter().map(Certificate).collect();
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, key)
            .map_err(|e| TlsError::Certificate(format!("Error configurando certificado: {}", e)))?;

        Ok(Self {
            config: Arc::new(config),
        })
    }

    /// Acepta una conexión TLS entrante
    pub fn accept(&self, stream: TcpStream) -> Result<TlsStream, TlsError> {
        let server_conn = ServerConnection::new(self.config.clone())
            .map_err(|e| TlsError::Handshake(format!("Error creando conexión servidor: {}", e)))?;

        Ok(TlsStream::Server(TlsServerStream {
            stream,
            conn: server_conn,
        }))
    }
}

/// Stream TLS del cliente
pub struct TlsClientStream {
    stream: TcpStream,
    conn: ClientConnection,
}

/// Stream TLS del servidor
pub struct TlsServerStream {
    stream: TcpStream,
    conn: ServerConnection,
}

/// Stream TLS que puede ser cliente o servidor
pub enum TlsStream {
    Client(TlsClientStream),
    Server(TlsServerStream),
}

impl Read for TlsStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, IoError> {
        match self {
            TlsStream::Client(stream) => stream.read(buf),
            TlsStream::Server(stream) => stream.read(buf),
        }
    }
}

impl Write for TlsStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize, IoError> {
        match self {
            TlsStream::Client(stream) => stream.write(buf),
            TlsStream::Server(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> Result<(), IoError> {
        match self {
            TlsStream::Client(stream) => stream.flush(),
            TlsStream::Server(stream) => stream.flush(),
        }
    }
}

impl Read for TlsClientStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, IoError> {
        self.conn.read_tls(&mut self.stream)?;
        self.conn.process_new_packets()
            .map_err(|e| IoError::new(std::io::ErrorKind::InvalidData, e))?;
        
        let mut plaintext = Vec::new();
        self.conn.reader().read_to_end(&mut plaintext)?;
        
        if plaintext.len() > buf.len() {
            return Err(IoError::new(std::io::ErrorKind::InvalidInput, "Buffer too small"));
        }
        
        buf[..plaintext.len()].copy_from_slice(&plaintext);
        Ok(plaintext.len())
    }
}

impl Write for TlsClientStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize, IoError> {
        self.conn.writer().write_all(buf)?;
        self.conn.write_tls(&mut self.stream)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), IoError> {
        self.conn.writer().flush()?;
        self.conn.write_tls(&mut self.stream)?;
        self.stream.flush()
    }
}

impl Read for TlsServerStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, IoError> {
        self.conn.read_tls(&mut self.stream)?;
        self.conn.process_new_packets()
            .map_err(|e| IoError::new(std::io::ErrorKind::InvalidData, e))?;
        
        let mut plaintext = Vec::new();
        self.conn.reader().read_to_end(&mut plaintext)?;
        
        if plaintext.len() > buf.len() {
            return Err(IoError::new(std::io::ErrorKind::InvalidInput, "Buffer too small"));
        }
        
        buf[..plaintext.len()].copy_from_slice(&plaintext);
        Ok(plaintext.len())
    }
}

impl Write for TlsServerStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize, IoError> {
        self.conn.writer().write_all(buf)?;
        self.conn.write_tls(&mut self.stream)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), IoError> {
        self.conn.writer().flush()?;
        self.conn.write_tls(&mut self.stream)?;
        self.stream.flush()
    }
}

impl TlsStream {
    /// Obtiene la dirección del peer
    pub fn peer_addr(&self) -> Result<std::net::SocketAddr, IoError> {
        match self {
            TlsStream::Client(stream) => stream.stream.peer_addr(),
            TlsStream::Server(stream) => stream.stream.peer_addr(),
        }
    }

    /// Clona el stream TLS
    pub fn try_clone(&self) -> Result<Self, IoError> {
        match self {
            TlsStream::Client(stream) => {
                let tcp_stream = stream.stream.try_clone()?;
                // Nota: Esta es una implementación simplificada
                // En una implementación real, necesitarías manejar el estado TLS
                Err(IoError::new(std::io::ErrorKind::Unsupported, "Clonación TLS no soportada"))
            },
            TlsStream::Server(stream) => {
                let tcp_stream = stream.stream.try_clone()?;
                Err(IoError::new(std::io::ErrorKind::Unsupported, "Clonación TLS no soportada"))
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_client_config_creation() {
        let config = TlsClientConfig::new();
        assert!(config.is_ok());
    }
} 