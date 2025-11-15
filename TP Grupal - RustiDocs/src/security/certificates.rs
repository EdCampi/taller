//! Módulo de certificados simplificado usando solo la biblioteca estándar
//!
//! Implementa generación y validación básica de certificados
//! para el protocolo TLS simplificado.

use crate::security::crypto::{SimpleRng, simple_hash};
use std::path::Path;
use std::time::UNIX_EPOCH;
use std::{fs, time::SystemTime};

/// Error que puede ocurrir durante operaciones con certificados
#[derive(Debug, Clone)]
pub enum CertificateError {
    /// Error de I/O
    Io(String),
    /// Error de generación
    Generation(String),
    /// Error de validación
    Validation(String),
    /// Error de formato
    Format(String),
}

impl std::fmt::Display for CertificateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CertificateError::Io(msg) => write!(f, "Error de I/O: {}", msg),
            CertificateError::Generation(msg) => write!(f, "Error de generación: {}", msg),
            CertificateError::Validation(msg) => write!(f, "Error de validación: {}", msg),
            CertificateError::Format(msg) => write!(f, "Error de formato: {}", msg),
        }
    }
}

impl std::error::Error for CertificateError {}

/// Estructura de certificado simplificado
#[derive(Debug, Clone)]
pub struct SimpleCertificate {
    pub version: u8,
    pub serial_number: u64,
    pub subject: String,
    pub issuer: String,
    pub not_before: u64,
    pub not_after: u64,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

impl SimpleCertificate {
    pub fn new(
        subject: String,
        issuer: String,
        public_key: Vec<u8>,
        validity_days: u32,
    ) -> Result<Self, CertificateError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let not_after = now + (validity_days as u64 * 24 * 60 * 60);

        let mut rng = SimpleRng::new_from_time();
        let serial_number = rng.next_u32() as u64;

        let cert = Self {
            version: 1,
            serial_number,
            subject,
            issuer,
            not_before: now,
            not_after,
            public_key,
            signature: Vec::new(),
        };

        Ok(cert)
    }

    /// Serializa el certificado a formato PEM simplificado
    pub fn to_pem(&self) -> String {
        let mut pem = String::new();
        pem.push_str("-----BEGIN SIMPLE CERTIFICATE-----\n");

        // Codificar datos en base64 simple
        let data = self.to_bytes();
        let encoded = simple_base64_encode(&data);

        // Dividir en líneas de 64 caracteres
        for chunk in encoded.as_bytes().chunks(64) {
            pem.push_str(&String::from_utf8_lossy(chunk));
            pem.push('\n');
        }

        pem.push_str("-----END SIMPLE CERTIFICATE-----\n");
        pem
    }

    /// Deserializa desde formato PEM
    pub fn from_pem(pem_data: &str) -> Result<Self, CertificateError> {
        let lines: Vec<&str> = pem_data.lines().collect();

        if lines.len() < 3 {
            return Err(CertificateError::Format("PEM inválido".to_string()));
        }

        if !lines[0].contains("BEGIN SIMPLE CERTIFICATE") {
            return Err(CertificateError::Format(
                "No es un certificado simple".to_string(),
            ));
        }

        if !lines[lines.len() - 1].contains("END SIMPLE CERTIFICATE") {
            return Err(CertificateError::Format("PEM incompleto".to_string()));
        }

        // Extraer datos codificados
        let encoded_data: String = lines[1..lines.len() - 1].join("");
        let data = simple_base64_decode(&encoded_data)
            .map_err(|e| CertificateError::Format(format!("Error decodificando base64: {}", e)))?;

        Self::from_bytes(&data)
    }

    /// Convierte el certificado a bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data = Vec::new();

        // Version
        data.push(self.version);

        // Serial number (8 bytes)
        data.extend_from_slice(&self.serial_number.to_be_bytes());

        // Subject length + subject
        let subject_bytes = self.subject.as_bytes();
        data.extend_from_slice(&(subject_bytes.len() as u32).to_be_bytes());
        data.extend_from_slice(subject_bytes);

        // Issuer length + issuer
        let issuer_bytes = self.issuer.as_bytes();
        data.extend_from_slice(&(issuer_bytes.len() as u32).to_be_bytes());
        data.extend_from_slice(issuer_bytes);

        // Validity dates
        data.extend_from_slice(&self.not_before.to_be_bytes());
        data.extend_from_slice(&self.not_after.to_be_bytes());

        // Public key length + public key
        data.extend_from_slice(&(self.public_key.len() as u32).to_be_bytes());
        data.extend_from_slice(&self.public_key);

        // Signature length + signature
        data.extend_from_slice(&(self.signature.len() as u32).to_be_bytes());
        data.extend_from_slice(&self.signature);

        data
    }

    /// Crea un certificado desde bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, CertificateError> {
        if data.len() < 25 {
            return Err(CertificateError::Format(
                "Datos de certificado demasiado cortos".to_string(),
            ));
        }

        let mut pos = 0;

        // Version
        let version = data[pos];
        pos += 1;

        // Serial number
        let serial_number = u64::from_be_bytes([
            data[pos],
            data[pos + 1],
            data[pos + 2],
            data[pos + 3],
            data[pos + 4],
            data[pos + 5],
            data[pos + 6],
            data[pos + 7],
        ]);
        pos += 8;

        // Subject
        let subject_len =
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;
        let subject = String::from_utf8_lossy(&data[pos..pos + subject_len]).to_string();
        pos += subject_len;

        // Issuer
        let issuer_len =
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;
        let issuer = String::from_utf8_lossy(&data[pos..pos + issuer_len]).to_string();
        pos += issuer_len;

        // Validity dates
        let not_before = u64::from_be_bytes([
            data[pos],
            data[pos + 1],
            data[pos + 2],
            data[pos + 3],
            data[pos + 4],
            data[pos + 5],
            data[pos + 6],
            data[pos + 7],
        ]);
        pos += 8;

        let not_after = u64::from_be_bytes([
            data[pos],
            data[pos + 1],
            data[pos + 2],
            data[pos + 3],
            data[pos + 4],
            data[pos + 5],
            data[pos + 6],
            data[pos + 7],
        ]);
        pos += 8;

        // Public key
        let pubkey_len =
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;
        let public_key = data[pos..pos + pubkey_len].to_vec();
        pos += pubkey_len;

        // Signature
        let sig_len = if pos < data.len() {
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize
        } else {
            0
        };
        pos += 4;
        let signature = if pos < data.len() && pos + sig_len <= data.len() {
            data[pos..pos + sig_len].to_vec()
        } else {
            Vec::new()
        };

        Ok(Self {
            version,
            serial_number,
            subject,
            issuer,
            not_before,
            not_after,
            public_key,
            signature,
        })
    }

    /// Verifica si el certificado es válido
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now >= self.not_before && now <= self.not_after
    }

    /// Verifica si el certificado está próximo a expirar
    pub fn is_expiring_soon(&self, days: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let warning_time = self.not_after - (days * 24 * 60 * 60);
        now >= warning_time
    }

    /// Firma el certificado con una clave privada
    pub fn sign(&mut self, private_key: &[u8]) -> Result<(), CertificateError> {
        // Crear datos para firmar (todo excepto la firma)
        let mut data_to_sign = self.to_bytes();

        // Remover la firma existente
        let sig_len_pos = data_to_sign.len() - 4;
        data_to_sign.truncate(sig_len_pos);

        // Generar firma simple
        let mut signature_data = Vec::new();
        signature_data.extend_from_slice(&data_to_sign);
        signature_data.extend_from_slice(private_key);

        self.signature = simple_hash(&signature_data).to_le_bytes().to_vec();

        Ok(())
    }

    /// Verifica la firma del certificado
    pub fn verify_signature(&self, public_key: &[u8]) -> bool {
        if self.signature.is_empty() {
            return false;
        }

        // Recrear datos firmados
        let mut data_to_sign = self.to_bytes();
        let sig_len_pos = data_to_sign.len() - 4;
        data_to_sign.truncate(sig_len_pos);

        // Generar firma esperada
        let mut signature_data = Vec::new();
        signature_data.extend_from_slice(&data_to_sign);
        signature_data.extend_from_slice(public_key);

        let expected_signature = simple_hash(&signature_data).to_le_bytes().to_vec();

        self.signature == expected_signature
    }
}

/// Genera un certificado autofirmado para desarrollo
pub fn generate_dev_certificate(
    subject: &str,
    validity_days: u32,
) -> Result<SimpleCertificate, CertificateError> {
    let mut rng = SimpleRng::new_from_time();
    let public_key = rng.generate_bytes(128);
    let private_key = rng.generate_bytes(64);

    let mut cert = SimpleCertificate::new(
        subject.to_string(),
        subject.to_string(), // Autofirmado
        public_key,
        validity_days,
    )?;

    cert.sign(&private_key)?;
    Ok(cert)
}

/// Guarda un certificado en formato PEM
pub fn save_certificate_pem(cert: &SimpleCertificate, path: &Path) -> Result<(), CertificateError> {
    let pem_data = cert.to_pem();
    fs::write(path, pem_data)
        .map_err(|e| CertificateError::Io(format!("Error escribiendo certificado: {}", e)))?;
    Ok(())
}

/// Carga un certificado desde formato PEM
pub fn load_certificate_pem(path: &Path) -> Result<SimpleCertificate, CertificateError> {
    let pem_data = fs::read_to_string(path)
        .map_err(|e| CertificateError::Io(format!("Error leyendo certificado: {}", e)))?;

    SimpleCertificate::from_pem(&pem_data)
}

/// Codificación base64 simple
fn simple_base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let mut value = 0u32;
        for (i, &byte) in chunk.iter().enumerate() {
            value |= (byte as u32) << ((2 - i) * 8);
        }

        for i in 0..4 {
            if i * 6 < chunk.len() * 8 {
                let index = ((value >> (18 - i * 6)) & 0x3F) as usize;
                result.push(CHARS[index] as char);
            } else {
                result.push('=');
            }
        }
    }

    result
}

/// Decodificación base64 simple
fn simple_base64_decode(encoded: &str) -> Result<Vec<u8>, String> {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = Vec::new();

    let mut value = 0u32;
    let mut bits = 0u32;

    for &byte in encoded.as_bytes() {
        if byte == b'=' {
            break;
        }

        let index = CHARS
            .iter()
            .position(|&c| c == byte)
            .ok_or_else(|| format!("Carácter inválido en base64: {}", byte as char))?;

        value = (value << 6) | (index as u32);
        bits += 6;

        if bits >= 8 {
            result.push(((value >> (bits - 8)) & 0xFF) as u8);
            bits -= 8;
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_certificate_creation() {
        let cert = generate_dev_certificate("test.localhost", 365).unwrap();
        assert_eq!(cert.subject, "test.localhost");
        assert_eq!(cert.issuer, "test.localhost");
        assert!(cert.is_valid());
    }

    #[test]
    fn test_certificate_serialization() {
        let cert = generate_dev_certificate("test.localhost", 365).unwrap();
        let bytes = cert.to_bytes();
        let deserialized = SimpleCertificate::from_bytes(&bytes).unwrap();

        assert_eq!(cert.subject, deserialized.subject);
        assert_eq!(cert.issuer, deserialized.issuer);
        assert_eq!(cert.serial_number, deserialized.serial_number);
    }

    #[test]
    fn test_certificate_pem() {
        let cert = generate_dev_certificate("test.localhost", 365).unwrap();
        let pem = cert.to_pem();
        let deserialized = SimpleCertificate::from_pem(&pem).unwrap();

        assert_eq!(cert.subject, deserialized.subject);
        assert_eq!(cert.issuer, deserialized.issuer);
    }

    #[test]
    fn test_certificate_file_io() {
        let temp_dir = TempDir::new().unwrap();
        let cert_path = temp_dir.path().join("test.crt");

        let cert = generate_dev_certificate("test.localhost", 365).unwrap();
        save_certificate_pem(&cert, &cert_path).unwrap();

        let loaded_cert = load_certificate_pem(&cert_path).unwrap();
        assert_eq!(cert.subject, loaded_cert.subject);
    }

    #[test]
    fn test_base64_encoding() {
        let original = b"Hello, World!";
        let encoded = simple_base64_encode(original);
        let decoded = simple_base64_decode(&encoded).unwrap();

        assert_eq!(original, decoded.as_slice());
    }
}
