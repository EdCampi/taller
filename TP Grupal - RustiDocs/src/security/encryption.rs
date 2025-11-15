//! Módulo de encriptación para datos sensibles
//! 
//! Este módulo proporciona funciones de utilidad para TLS
//! y generación de claves de encriptación.

/// Error que puede ocurrir durante operaciones de encriptación
#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error("Error de I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("Error de encriptación: {0}")]
    Encryption(String),
    #[error("Error de desencriptación: {0}")]
    Decryption(String),
}

/// Función de utilidad para generar una clave de encriptación
pub fn generate_encryption_key(length: usize) -> Vec<u8> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..length).map(|_| rng.gen()).collect()
}

/// Función de utilidad para encriptar datos en memoria
pub fn encrypt_in_memory(data: &[u8], key: &[u8]) -> Result<Vec<u8>, EncryptionError> {
    let mut encrypted = Vec::with_capacity(data.len());
    for (i, &byte) in data.iter().enumerate() {
        let key_byte = key[i % key.len()];
        encrypted.push(byte ^ key_byte);
    }
    Ok(encrypted)
}

/// Función de utilidad para desencriptar datos en memoria
pub fn decrypt_in_memory(data: &[u8], key: &[u8]) -> Result<Vec<u8>, EncryptionError> {
    encrypt_in_memory(data, key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() {
        let key = generate_encryption_key(16);
        let original_data = b"Hello, World!";
        
        let encrypted = encrypt_in_memory(original_data, &key).unwrap();
        let decrypted = decrypt_in_memory(&encrypted, &key).unwrap();
        
        assert_eq!(original_data, decrypted.as_slice());
    }
} 