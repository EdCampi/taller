//! Módulo TLS simplificado usando solo la biblioteca estándar
//!
//! Implementa un protocolo de handshake básico y encriptación
//! para proteger las comunicaciones.

use crate::security::crypto::{CryptoError, SimpleCipher, SimpleRng, simple_hash};
use std::io::{Error as IoError, Read, Write};
use std::net::TcpStream;

/// Error que puede ocurrir durante operaciones TLS
#[derive(Debug, Clone)]
pub enum TlsError {
    /// Error de I/O
    Io(String),
    /// Error de handshake
    Handshake(String),
    /// Error de encriptación
    Encryption(String),
    /// Error de validación
    Validation(String),
    /// Error de protocolo
    Protocol(String),
}

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TlsError::Io(msg) => write!(f, "Error de I/O: {}", msg),
            TlsError::Handshake(msg) => write!(f, "Error de handshake: {}", msg),
            TlsError::Encryption(msg) => write!(f, "Error de encriptación: {}", msg),
            TlsError::Validation(msg) => write!(f, "Error de validación: {}", msg),
            TlsError::Protocol(msg) => write!(f, "Error de protocolo: {}", msg),
        }
    }
}

impl std::error::Error for TlsError {}

impl From<IoError> for TlsError {
    fn from(err: IoError) -> Self {
        TlsError::Io(err.to_string())
    }
}

impl From<CryptoError> for TlsError {
    fn from(err: CryptoError) -> Self {
        TlsError::Encryption(err.to_string())
    }
}

/// Tipos de mensajes del protocolo TLS simplificado
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TlsMessageType {
    ClientHello = 1,
    ServerHello = 2,
    KeyExchange = 3,
    Finished = 4,
    ApplicationData = 5,
}

impl TlsMessageType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(TlsMessageType::ClientHello),
            2 => Some(TlsMessageType::ServerHello),
            3 => Some(TlsMessageType::KeyExchange),
            4 => Some(TlsMessageType::Finished),
            5 => Some(TlsMessageType::ApplicationData),
            _ => None,
        }
    }
}

/// Estructura de mensaje TLS
#[derive(Debug, Clone)]
pub struct TlsMessage {
    pub message_type: TlsMessageType,
    pub payload: Vec<u8>,
}

impl TlsMessage {
    pub fn new(message_type: TlsMessageType, payload: Vec<u8>) -> Self {
        Self {
            message_type,
            payload,
        }
    }

    /// Serializa el mensaje a bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        result.push(self.message_type as u8);

        let length = self.payload.len() as u32;
        result.extend_from_slice(&length.to_be_bytes());
        result.extend_from_slice(&self.payload);

        result
    }

    /// Deserializa bytes a mensaje
    pub fn from_bytes(data: &[u8]) -> Result<Self, TlsError> {
        if data.len() < 5 {
            return Err(TlsError::Protocol(
                "Mensaje TLS demasiado corto".to_string(),
            ));
        }

        let message_type = TlsMessageType::from_u8(data[0])
            .ok_or_else(|| TlsError::Protocol("Tipo de mensaje inválido".to_string()))?;

        let length = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;

        if data.len() < 5 + length {
            return Err(TlsError::Protocol("Mensaje TLS incompleto".to_string()));
        }

        let payload = data[5..5 + length].to_vec();

        Ok(Self {
            message_type,
            payload,
        })
    }
}

/// Configuración TLS para el cliente
pub struct TlsClientConfig {
    pub server_name: String,
    pub supported_ciphers: Vec<u32>,
}

impl TlsClientConfig {
    pub fn new(server_name: String) -> Self {
        Self {
            server_name,
            supported_ciphers: vec![0x0001], // Cipher suite simple
        }
    }
}

/// Configuración TLS para el servidor
pub struct TlsServerConfig {
    pub cert_data: Vec<u8>,
    pub key_data: Vec<u8>,
}

impl TlsServerConfig {
    pub fn new() -> Self {
        // Para simplificar, generamos datos de certificado y clave
        let mut rng = SimpleRng::new_from_time();
        let cert_data = rng.generate_bytes(256);
        let key_data = rng.generate_bytes(128);

        Self {
            cert_data,
            key_data,
        }
    }
}

/// Stream TLS del cliente
#[derive(Debug)]
pub struct TlsClientStream {
    stream: TcpStream,
    cipher: Option<SimpleCipher>,
    session_id: Vec<u8>,
}

/// Stream TLS del servidor
#[derive(Debug)]
pub struct TlsServerStream {
    stream: TcpStream,
    cipher: Option<SimpleCipher>,
    session_id: Vec<u8>,
}

/// Stream TLS que puede ser cliente o servidor
pub enum TlsStream {
    Client(TlsClientStream),
    Server(TlsServerStream),
}

impl TlsClientStream {
    pub fn new(stream: TcpStream, _config: TlsClientConfig) -> Result<Self, TlsError> {
        let mut client_stream = Self {
            stream,
            cipher: None,
            session_id: Vec::new(),
        };

        client_stream.perform_handshake()?;
        Ok(client_stream)
    }

    fn perform_handshake(&mut self) -> Result<(), TlsError> {
        // Paso 1: ClientHello
        let mut rng = SimpleRng::new_from_time();
        self.session_id = rng.generate_bytes(32);

        let client_hello = TlsMessage::new(TlsMessageType::ClientHello, self.session_id.clone());

        self.send_message(&client_hello)?;

        // Paso 2: Recibir ServerHello
        let server_hello = self.receive_message()?;
        if server_hello.message_type != TlsMessageType::ServerHello {
            return Err(TlsError::Handshake("Esperaba ServerHello".to_string()));
        }

        // Paso 3: Recibir KeyExchange
        let key_exchange = self.receive_message()?;
        if key_exchange.message_type != TlsMessageType::KeyExchange {
            return Err(TlsError::Handshake("Esperaba KeyExchange".to_string()));
        }

        // Generar clave compartida
        let shared_key = self.generate_shared_key(&key_exchange.payload)?;
        self.cipher = Some(SimpleCipher::new(shared_key));

        // Paso 4: Enviar Finished
        let finished = TlsMessage::new(
            TlsMessageType::Finished,
            simple_hash(&self.session_id).to_le_bytes().to_vec(),
        );

        self.send_message(&finished)?;

        // Paso 5: Recibir Finished del servidor
        let server_finished = self.receive_message()?;
        if server_finished.message_type != TlsMessageType::Finished {
            return Err(TlsError::Handshake(
                "Esperaba Finished del servidor".to_string(),
            ));
        }

        Ok(())
    }

    fn generate_shared_key(&self, server_key_data: &[u8]) -> Result<Vec<u8>, TlsError> {
        // Implementación simple de generación de clave compartida
        let mut key = Vec::new();
        key.extend_from_slice(&self.session_id);
        key.extend_from_slice(server_key_data);

        // Usar hash para generar clave final
        let key_hash = simple_hash(&key);
        Ok(key_hash.to_le_bytes().to_vec())
    }

    fn send_message(&mut self, message: &TlsMessage) -> Result<(), TlsError> {
        let data = message.to_bytes();
        self.stream.write_all(&data)?;
        Ok(())
    }

    fn receive_message(&mut self) -> Result<TlsMessage, TlsError> {
        let mut header = [0u8; 5];
        self.stream.read_exact(&mut header)?;

        let message_type = TlsMessageType::from_u8(header[0])
            .ok_or_else(|| TlsError::Protocol("Tipo de mensaje inválido".to_string()))?;

        let length = u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;

        let mut payload = vec![0u8; length];
        self.stream.read_exact(&mut payload)?;

        Ok(TlsMessage::new(message_type, payload))
    }
}

impl TlsServerStream {
    pub fn new(stream: TcpStream, config: TlsServerConfig) -> Result<Self, TlsError> {
        let mut server_stream = Self {
            stream,
            cipher: None,
            session_id: Vec::new(),
        };

        server_stream.perform_handshake(config)?;
        Ok(server_stream)
    }

    fn perform_handshake(&mut self, config: TlsServerConfig) -> Result<(), TlsError> {
        // Paso 1: Recibir ClientHello
        let client_hello = self.receive_message()?;
        if client_hello.message_type != TlsMessageType::ClientHello {
            return Err(TlsError::Handshake("Esperaba ClientHello".to_string()));
        }

        self.session_id = client_hello.payload;

        // Paso 2: Enviar ServerHello
        let server_hello = TlsMessage::new(TlsMessageType::ServerHello, self.session_id.clone());

        self.send_message(&server_hello)?;

        // Paso 3: Enviar KeyExchange
        let key_exchange = TlsMessage::new(TlsMessageType::KeyExchange, config.key_data.clone());

        self.send_message(&key_exchange)?;

        // Generar clave compartida
        let shared_key = self.generate_shared_key(&config.key_data)?;
        self.cipher = Some(SimpleCipher::new(shared_key));

        // Paso 4: Recibir Finished del cliente
        let client_finished = self.receive_message()?;
        if client_finished.message_type != TlsMessageType::Finished {
            return Err(TlsError::Handshake(
                "Esperaba Finished del cliente".to_string(),
            ));
        }

        // Verificar Finished
        let expected_hash = simple_hash(&self.session_id);
        let received_hash = u64::from_le_bytes(
            client_finished
                .payload
                .as_slice()
                .try_into()
                .map_err(|_| TlsError::Validation("Hash inválido".to_string()))?,
        );

        if expected_hash != received_hash {
            return Err(TlsError::Validation(
                "Hash de Finished no coincide".to_string(),
            ));
        }

        // Paso 5: Enviar Finished
        let finished = TlsMessage::new(
            TlsMessageType::Finished,
            expected_hash.to_le_bytes().to_vec(),
        );

        self.send_message(&finished)?;

        Ok(())
    }

    fn generate_shared_key(&self, server_key_data: &[u8]) -> Result<Vec<u8>, TlsError> {
        // Misma implementación que el cliente
        let mut key = Vec::new();
        key.extend_from_slice(&self.session_id);
        key.extend_from_slice(server_key_data);

        let key_hash = simple_hash(&key);
        Ok(key_hash.to_le_bytes().to_vec())
    }

    fn send_message(&mut self, message: &TlsMessage) -> Result<(), TlsError> {
        let data = message.to_bytes();
        self.stream.write_all(&data)?;
        Ok(())
    }

    fn receive_message(&mut self) -> Result<TlsMessage, TlsError> {
        let mut header = [0u8; 5];
        self.stream.read_exact(&mut header)?;

        let message_type = TlsMessageType::from_u8(header[0])
            .ok_or_else(|| TlsError::Protocol("Tipo de mensaje inválido".to_string()))?;

        let length = u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;

        let mut payload = vec![0u8; length];
        self.stream.read_exact(&mut payload)?;

        Ok(TlsMessage::new(message_type, payload))
    }
}

impl Read for TlsClientStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, IoError> {
        if let Some(cipher) = &self.cipher {
            // Leer mensaje encriptado
            let message = {
                let mut header = [0u8; 5];
                self.stream.read_exact(&mut header)?;

                let message_type = TlsMessageType::from_u8(header[0]).ok_or_else(|| {
                    IoError::new(std::io::ErrorKind::InvalidData, "Tipo de mensaje inválido")
                })?;

                let length =
                    u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;

                let mut payload = vec![0u8; length];
                self.stream.read_exact(&mut payload)?;

                TlsMessage::new(message_type, payload)
            };

            if message.message_type != TlsMessageType::ApplicationData {
                return Err(IoError::new(
                    std::io::ErrorKind::InvalidData,
                    "Tipo de mensaje inválido",
                ));
            }

            // Desencriptar datos
            let decrypted = cipher.decrypt(&message.payload);

            let copy_len = std::cmp::min(decrypted.len(), buf.len());
            buf[..copy_len].copy_from_slice(&decrypted[..copy_len]);

            Ok(copy_len)
        } else {
            Err(IoError::new(
                std::io::ErrorKind::NotConnected,
                "Handshake no completado",
            ))
        }
    }
}

impl Write for TlsClientStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize, IoError> {
        if let Some(cipher) = &self.cipher {
            // Encriptar datos
            let encrypted = cipher.encrypt(buf);

            // Crear mensaje de aplicación
            let message = TlsMessage::new(TlsMessageType::ApplicationData, encrypted);

            self.send_message(&message)
                .map_err(|e| IoError::new(std::io::ErrorKind::InvalidData, e))?;

            Ok(buf.len())
        } else {
            Err(IoError::new(
                std::io::ErrorKind::NotConnected,
                "Handshake no completado",
            ))
        }
    }

    fn flush(&mut self) -> Result<(), IoError> {
        self.stream.flush()
    }
}

impl Read for TlsServerStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, IoError> {
        if let Some(cipher) = &self.cipher {
            // Leer mensaje encriptado
            let message = {
                let mut header = [0u8; 5];
                self.stream.read_exact(&mut header)?;

                let message_type = TlsMessageType::from_u8(header[0]).ok_or_else(|| {
                    IoError::new(std::io::ErrorKind::InvalidData, "Tipo de mensaje inválido")
                })?;

                let length =
                    u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;

                let mut payload = vec![0u8; length];
                self.stream.read_exact(&mut payload)?;

                TlsMessage::new(message_type, payload)
            };

            if message.message_type != TlsMessageType::ApplicationData {
                return Err(IoError::new(
                    std::io::ErrorKind::InvalidData,
                    "Tipo de mensaje inválido",
                ));
            }

            // Desencriptar datos
            let decrypted = cipher.decrypt(&message.payload);

            let copy_len = std::cmp::min(decrypted.len(), buf.len());
            buf[..copy_len].copy_from_slice(&decrypted[..copy_len]);

            Ok(copy_len)
        } else {
            Err(IoError::new(
                std::io::ErrorKind::NotConnected,
                "Handshake no completado",
            ))
        }
    }
}

impl Write for TlsServerStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize, IoError> {
        if let Some(cipher) = &self.cipher {
            // Encriptar datos
            let encrypted = cipher.encrypt(buf);

            // Crear mensaje de aplicación
            let message = TlsMessage::new(TlsMessageType::ApplicationData, encrypted);

            self.send_message(&message)
                .map_err(|e| IoError::new(std::io::ErrorKind::InvalidData, e))?;

            Ok(buf.len())
        } else {
            Err(IoError::new(
                std::io::ErrorKind::NotConnected,
                "Handshake no completado",
            ))
        }
    }

    fn flush(&mut self) -> Result<(), IoError> {
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
            TlsStream::Client(_stream) => Err(IoError::new(
                std::io::ErrorKind::Unsupported,
                "Clonación TLS no soportada",
            )),
            TlsStream::Server(_stream) => Err(IoError::new(
                std::io::ErrorKind::Unsupported,
                "Clonación TLS no soportada",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn test_tls_message_serialization() {
        let message = TlsMessage::new(TlsMessageType::ClientHello, b"test_payload".to_vec());

        let bytes = message.to_bytes();
        let deserialized = TlsMessage::from_bytes(&bytes).unwrap();

        assert_eq!(message.message_type, deserialized.message_type);
        assert_eq!(message.payload, deserialized.payload);
    }

    #[test]
    fn test_tls_handshake() {
        let listener = TcpListener::bind("0.0.0.0:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server_config = TlsServerConfig::new();

        // Servidor en hilo separado
        let server_handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut tls_stream = TlsServerStream::new(stream, server_config).unwrap();

            let mut buffer = [0u8; 1024];
            let bytes_read = tls_stream.read(&mut buffer).unwrap();
            let response = b"Hello from TLS server!";
            tls_stream.write_all(response).unwrap();

            (bytes_read, response.len())
        });

        // Cliente
        let client_stream = TcpStream::connect(addr).unwrap();
        let client_config = TlsClientConfig::new("localhost".to_string());
        let mut tls_client = TlsClientStream::new(client_stream, client_config).unwrap();

        let test_data = b"Hello from TLS client!";
        tls_client.write_all(test_data).unwrap();

        let mut buffer = [0u8; 1024];
        let bytes_read = tls_client.read(&mut buffer).unwrap();

        let (server_bytes_read, server_response_len) = server_handle.join().unwrap();

        assert_eq!(server_bytes_read, test_data.len());
        assert_eq!(bytes_read, server_response_len);
    }
}
