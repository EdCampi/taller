//! Módulo de criptografía básica usando solo la biblioteca estándar
//!
//! Implementa algoritmos de encriptación simétrica y funciones hash
//! usando operaciones matemáticas básicas.

use std::io::{Error as IoError, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::Mutex;

/// Error que puede ocurrir durante operaciones criptográficas
#[derive(Debug, Clone)]
pub enum CryptoError {
    /// Error de I/O
    Io(String),
    /// Error de encriptación
    Encryption(String),
    /// Error de desencriptación
    Decryption(String),
    /// Error de generación de clave
    KeyGeneration(String),
    /// Error de validación
    Validation(String),
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoError::Io(msg) => write!(f, "Error de I/O: {}", msg),
            CryptoError::Encryption(msg) => write!(f, "Error de encriptación: {}", msg),
            CryptoError::Decryption(msg) => write!(f, "Error de desencriptación: {}", msg),
            CryptoError::KeyGeneration(msg) => write!(f, "Error de generación de clave: {}", msg),
            CryptoError::Validation(msg) => write!(f, "Error de validación: {}", msg),
        }
    }
}

impl std::error::Error for CryptoError {}

impl From<IoError> for CryptoError {
    fn from(err: IoError) -> Self {
        CryptoError::Io(err.to_string())
    }
}

/// Generador de números pseudo-aleatorios simple
pub struct SimpleRng {
    seed: u64,
}

impl SimpleRng {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    pub fn new_from_time() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        Self::new(time)
    }

    pub fn next_u32(&mut self) -> u32 {
        // Implementación del algoritmo xorshift
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed & 0xFFFFFFFF) as u32
    }

    pub fn next_u8(&mut self) -> u8 {
        (self.next_u32() & 0xFF) as u8
    }

    pub fn generate_bytes(&mut self, length: usize) -> Vec<u8> {
        (0..length).map(|_| self.next_u8()).collect()
    }
}

/// Función hash simple (para demostración)
pub fn simple_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0x811c9dc5; // FNV-1a offset basis
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x01000193); // FNV-1a prime
    }
    hash
}

/// Algoritmo de encriptación simétrica simple (XOR con clave expandida)
#[derive(Debug)]
pub struct SimpleCipher {
    key: Vec<u8>,
    key_hash: u64,
}

impl SimpleCipher {
    pub fn new(key: Vec<u8>) -> Self {
        let key_hash = simple_hash(&key);
        Self { key, key_hash }
    }

    pub fn generate_key(length: usize) -> Self {
        let mut rng = SimpleRng::new_from_time();
        let key = rng.generate_bytes(length);
        Self::new(key)
    }

    /// Encripta datos usando XOR con clave expandida
    pub fn encrypt(&self, data: &[u8]) -> Vec<u8> {
        let mut encrypted = Vec::with_capacity(data.len());
        let mut rng = SimpleRng::new(self.key_hash);

        for (i, &byte) in data.iter().enumerate() {
            let key_byte = if i < self.key.len() {
                self.key[i]
            } else {
                rng.next_u8()
            };
            encrypted.push(byte ^ key_byte);
        }

        encrypted
    }

    /// Desencripta datos (simétrico)
    pub fn decrypt(&self, data: &[u8]) -> Vec<u8> {
        self.encrypt(data) // XOR es simétrico
    }

    /// Encripta con autenticación (incluye hash de integridad)
    pub fn encrypt_with_auth(&self, data: &[u8]) -> Vec<u8> {
        let encrypted = self.encrypt(data);
        let hash = simple_hash(data);
        let hash_bytes = hash.to_le_bytes();

        let mut result = Vec::with_capacity(encrypted.len() + 8);
        result.extend_from_slice(&hash_bytes);
        result.extend_from_slice(&encrypted);
        result
    }

    /// Desencripta con verificación de autenticación
    pub fn decrypt_with_auth(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if data.len() < 8 {
            return Err(CryptoError::Validation(
                "Datos encriptados demasiado cortos".to_string(),
            ));
        }

        let hash_bytes = &data[..8];
        let encrypted = &data[8..];

        let decrypted = self.decrypt(encrypted);
        let expected_hash = u64::from_le_bytes(hash_bytes.try_into().unwrap());
        let actual_hash = simple_hash(&decrypted);

        if expected_hash != actual_hash {
            return Err(CryptoError::Validation(
                "Hash de integridad no coincide".to_string(),
            ));
        }

        Ok(decrypted)
    }
}

/// Stream encriptado que implementa Read y Write
pub struct EncryptedStream {
    stream: TcpStream,
    cipher: Arc<Mutex<SimpleCipher>>,
    buffer: Vec<u8>,
    buffer_pos: usize,
}

impl EncryptedStream {
    pub fn new(stream: TcpStream, cipher: SimpleCipher) -> Self {
        Self {
            stream,
            cipher: Arc::new(Mutex::new(cipher)),
            buffer: Vec::new(),
            buffer_pos: 0,
        }
    }

    pub fn with_key(stream: TcpStream, key: Vec<u8>) -> Self {
        let cipher = SimpleCipher::new(key);
        Self::new(stream, cipher)
    }

    pub fn with_random_key(stream: TcpStream, key_length: usize) -> Self {
        let cipher = SimpleCipher::generate_key(key_length);
        Self::new(stream, cipher)
    }
}

impl Read for EncryptedStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, IoError> {
        // Si no hay datos en el buffer, leer del stream
        if self.buffer_pos >= self.buffer.len() {
            let mut temp_buf = vec![0u8; 1024];
            let bytes_read = self.stream.read(&mut temp_buf)?;

            if bytes_read == 0 {
                return Ok(0);
            }

            // Desencriptar datos
            let _cipher = self
                .cipher
                .lock()
                .map_err(|_| IoError::new(std::io::ErrorKind::InvalidData, "Error de lock"))?;

            self.buffer = _cipher.decrypt(&temp_buf[..bytes_read]);
            self.buffer_pos = 0;
        }

        // Copiar datos del buffer
        let available = self.buffer.len() - self.buffer_pos;
        let to_copy = std::cmp::min(available, buf.len());

        if to_copy > 0 {
            buf[..to_copy]
                .copy_from_slice(&self.buffer[self.buffer_pos..self.buffer_pos + to_copy]);
            self.buffer_pos += to_copy;
        }

        Ok(to_copy)
    }
}

impl Write for EncryptedStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize, IoError> {
        let cipher = self
            .cipher
            .lock()
            .map_err(|_| IoError::new(std::io::ErrorKind::InvalidData, "Error de lock"))?;

        let encrypted = cipher.encrypt(buf);
        self.stream.write_all(&encrypted)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), IoError> {
        self.stream.flush()
    }
}

impl EncryptedStream {
    /// Obtiene la dirección del peer
    pub fn peer_addr(&self) -> Result<std::net::SocketAddr, IoError> {
        self.stream.peer_addr()
    }

    /// Clona el stream encriptado
    pub fn try_clone(&self) -> Result<Self, IoError> {
        let stream = self.stream.try_clone()?;
        let _cipher = self
            .cipher
            .lock()
            .map_err(|_| IoError::new(std::io::ErrorKind::InvalidData, "Error de lock"))?;

        Ok(Self {
            stream,
            cipher: self.cipher.clone(),
            buffer: Vec::new(),
            buffer_pos: 0,
        })
    }

    /// Cambia la clave de encriptación
    pub fn change_key(&mut self, new_key: Vec<u8>) {
        let mut cipher = self.cipher.lock().unwrap();
        *cipher = SimpleCipher::new(new_key);
    }
}

/// Funciones de utilidad para encriptación en memoria
pub fn encrypt_in_memory(data: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = SimpleCipher::new(key.to_vec());
    Ok(cipher.encrypt(data))
}

pub fn decrypt_in_memory(data: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = SimpleCipher::new(key.to_vec());
    Ok(cipher.decrypt(data))
}

pub fn encrypt_with_auth_in_memory(data: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = SimpleCipher::new(key.to_vec());
    Ok(cipher.encrypt_with_auth(data))
}

pub fn decrypt_with_auth_in_memory(data: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = SimpleCipher::new(key.to_vec());
    cipher.decrypt_with_auth(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn test_simple_rng() {
        let mut rng = SimpleRng::new(12345);
        let num1 = rng.next_u32();
        let num2 = rng.next_u32();
        assert_ne!(num1, num2);
    }

    #[test]
    fn test_simple_hash() {
        let data1 = b"Hello, World!";
        let data2 = b"Hello, World!";
        let data3 = b"Hello, World";

        assert_eq!(simple_hash(data1), simple_hash(data2));
        assert_ne!(simple_hash(data1), simple_hash(data3));
    }

    #[test]
    fn test_simple_cipher() {
        let key = b"secret_key_32_bytes_long_key".to_vec();
        let cipher = SimpleCipher::new(key);
        let original_data = b"Hello, World! This is a test message.";

        let encrypted = cipher.encrypt(original_data);
        let decrypted = cipher.decrypt(&encrypted);

        assert_eq!(original_data, decrypted.as_slice());
    }

    #[test]
    fn test_cipher_with_auth() {
        let key = b"secret_key_32_bytes_long_key".to_vec();
        let cipher = SimpleCipher::new(key);
        let original_data = b"Hello, World! This is a test message.";

        let encrypted = cipher.encrypt_with_auth(original_data);
        let decrypted = cipher.decrypt_with_auth(&encrypted).unwrap();

        assert_eq!(original_data, decrypted.as_slice());
    }

    #[test]
    fn test_encrypted_stream() {
        let listener = TcpListener::bind("0.0.0.0:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let key = b"secret_key_32_bytes_long_key".to_vec();
        let test_data = b"Test data for encryption";

        // Cliente
        let client_stream = TcpStream::connect(addr).unwrap();
        let mut client_encrypted = EncryptedStream::with_key(client_stream, key.clone());

        // Servidor
        let (server_stream, _) = listener.accept().unwrap();
        let mut server_encrypted = EncryptedStream::with_key(server_stream, key);

        // Enviar datos encriptados
        client_encrypted.write_all(test_data).unwrap();

        // Recibir y desencriptar datos
        let mut buffer = [0u8; 1024];
        let bytes_read = server_encrypted.read(&mut buffer).unwrap();

        assert_eq!(&buffer[..bytes_read], test_data);
    }
}
