//! Módulo de identificación y comunicación de clientes
//!
//! Este módulo proporciona la funcionalidad para identificar y comunicarse con
//! clientes conectados al servidor Redis. Cada cliente tiene un identificador
//! único y un canal de comunicación para recibir mensajes de respuesta.
//!
//! # Características principales
//!
//! - **Identificación única**: Cada cliente tiene un ID único para tracking
//! - **Comunicación asíncrona**: Utiliza canales para envío de mensajes RESP
//! - **Validación de IDs**: Verifica que los IDs cumplan con el formato requerido
//! - **Manejo robusto de errores**: Enum específico para errores de comunicación
//! - **Thread Safety**: Implementa traits necesarios para uso en contextos concurrentes
//! - **Debugging**: Proporciona información detallada para debugging

use crate::network::resp_message::RespMessage;
use std::fmt;
use std::sync::mpsc::{SendError, Sender};

/// Error que puede ocurrir al enviar mensajes a un cliente.
#[derive(Debug, Clone, PartialEq)]
pub enum ClientIdError {
    /// Error al enviar mensaje al cliente
    SendError(String),
    /// ID de cliente inválido
    InvalidId(String),
}

impl fmt::Display for ClientIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientIdError::SendError(msg) => write!(f, "Error al enviar mensaje: {}", msg),
            ClientIdError::InvalidId(id) => write!(f, "ID de cliente inválido: {}", id),
        }
    }
}

impl std::error::Error for ClientIdError {}

impl From<SendError<RespMessage>> for ClientIdError {
    fn from(err: SendError<RespMessage>) -> Self {
        ClientIdError::SendError(err.to_string())
    }
}

/// Identificador único para un cliente conectado al servidor.
///
/// Cada cliente tiene un ID único y un canal de comunicación para recibir mensajes.
/// Esta estructura permite enviar mensajes de respuesta directamente al cliente.
#[derive(Debug, Clone)]
pub struct ClientId {
    /// ID único del cliente
    id: String,
    /// Canal de comunicación para enviar mensajes al cliente
    sender: Sender<RespMessage>,
}

impl ClientId {
    /// Crea una nueva instancia de ClientId.
    ///
    /// # Arguments
    ///
    /// * `id` - ID único del cliente
    /// * `sender` - Canal de comunicación para enviar mensajes
    ///
    /// # Returns
    ///
    /// Nueva instancia de ClientId
    pub fn new(id: String, sender: Sender<RespMessage>) -> Self {
        Self { id, sender }
    }

    /// Envía un mensaje al cliente.
    ///
    /// # Arguments
    ///
    /// * `message` - Mensaje a enviar
    ///
    /// # Returns
    ///
    /// `Result<(), ClientIdError>` - Resultado de la operación de envío
    pub fn send(&self, message: RespMessage) -> Result<(), ClientIdError> {
        self.sender
            .send(message)
            .map_err(|e| ClientIdError::SendError(e.to_string()))
    }

    /// Obtiene el ID del cliente.
    ///
    /// # Returns
    ///
    /// Referencia al ID del cliente
    pub fn get_id(&self) -> &str {
        &self.id
    }

    /// Valida si el ID del cliente es válido.
    ///
    /// Un ID válido debe ser no vacío y contener solo caracteres alfanuméricos y guiones bajos.
    ///
    /// # Returns
    ///
    /// `true` si el ID es válido, `false` en caso contrario
    pub fn is_valid_id(&self) -> bool {
        !self.id.is_empty() && self.id.chars().all(|c| c.is_alphanumeric() || c == '_')
    }

    /// Crea un ClientId con validación del ID.
    ///
    /// # Arguments
    ///
    /// * `id` - ID del cliente a validar
    /// * `sender` - Canal de comunicación
    ///
    /// # Returns
    ///
    /// `Result<ClientId, ClientIdError>` - ClientId si el ID es válido, error en caso contrario
    pub fn new_checked(id: String, sender: Sender<RespMessage>) -> Result<Self, ClientIdError> {
        if id.is_empty() {
            return Err(ClientIdError::InvalidId(
                "ID no puede estar vacío".to_string(),
            ));
        }

        if !id.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(ClientIdError::InvalidId(
                "ID debe contener solo caracteres alfanuméricos y guiones bajos".to_string(),
            ));
        }

        Ok(Self::new(id, sender))
    }
}

impl PartialEq for ClientId {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ClientId {}

impl fmt::Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ClientId({})", self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn test_client_id_new() {
        let (sender, _) = mpsc::channel();
        let client = ClientId::new("test_client".to_string(), sender);
        assert_eq!(client.get_id(), "test_client");
    }

    #[test]
    fn test_client_id_send_success() {
        let (sender, receiver) = mpsc::channel();
        let client = ClientId::new("test_client".to_string(), sender);

        let message = RespMessage::SimpleString("OK".to_string());
        let result = client.send(message.clone());
        assert!(result.is_ok());

        let received = receiver.recv().unwrap();
        assert_eq!(received, message);
    }

    #[test]
    fn test_client_id_send_failure() {
        let (sender, receiver) = mpsc::channel();
        let client = ClientId::new("test_client".to_string(), sender);

        // Consumir el receptor para que el envío falle
        drop(receiver);

        let message = RespMessage::SimpleString("OK".to_string());
        let result = client.send(message);
        assert!(result.is_err());

        match result.unwrap_err() {
            ClientIdError::SendError(_) => (),
            _ => panic!("Se esperaba error SendError"),
        }
    }

    #[test]
    fn test_client_id_get_id() {
        let (sender, _) = mpsc::channel();
        let client = ClientId::new("test_client_123".to_string(), sender);
        assert_eq!(client.get_id(), "test_client_123");
    }

    #[test]
    fn test_client_id_partial_eq() {
        let (sender1, _) = mpsc::channel();
        let (sender2, _) = mpsc::channel();
        let (sender3, _) = mpsc::channel();

        let client1 = ClientId::new("test_client".to_string(), sender1);
        let client2 = ClientId::new("test_client".to_string(), sender2);
        let client3 = ClientId::new("other_client".to_string(), sender3);

        assert_eq!(client1, client2);
        assert_ne!(client1, client3);
    }

    #[test]
    fn test_client_id_is_valid_id() {
        let (sender, _) = mpsc::channel();

        let valid_client = ClientId::new("client_123".to_string(), sender.clone());
        assert!(valid_client.is_valid_id());

        let invalid_client = ClientId::new("".to_string(), sender.clone());
        assert!(!invalid_client.is_valid_id());

        let invalid_client2 = ClientId::new("client-123".to_string(), sender.clone());
        assert!(!invalid_client2.is_valid_id());

        let valid_client2 = ClientId::new("client123".to_string(), sender);
        assert!(valid_client2.is_valid_id());
    }

    #[test]
    fn test_client_id_new_checked_success() {
        let (sender, _) = mpsc::channel();
        let result = ClientId::new_checked("valid_client_123".to_string(), sender);
        assert!(result.is_ok());

        let client = result.unwrap();
        assert_eq!(client.get_id(), "valid_client_123");
    }

    #[test]
    fn test_client_id_new_checked_empty_id() {
        let (sender, _) = mpsc::channel();
        let result = ClientId::new_checked("".to_string(), sender);
        assert!(result.is_err());

        match result.unwrap_err() {
            ClientIdError::InvalidId(msg) => assert!(msg.contains("vacío")),
            _ => panic!("Se esperaba error InvalidId"),
        }
    }

    #[test]
    fn test_client_id_new_checked_invalid_characters() {
        let (sender, _) = mpsc::channel();
        let result = ClientId::new_checked("client-123".to_string(), sender);
        assert!(result.is_err());

        match result.unwrap_err() {
            ClientIdError::InvalidId(msg) => assert!(msg.contains("alfanumérico")),
            _ => panic!("Se esperaba error InvalidId"),
        }
    }

    #[test]
    fn test_client_id_display() {
        let (sender, _) = mpsc::channel();
        let client = ClientId::new("test_client".to_string(), sender);
        assert_eq!(client.to_string(), "ClientId(test_client)");
    }

    #[test]
    fn test_client_id_error_display() {
        let send_error = ClientIdError::SendError("Connection lost".to_string());
        assert_eq!(
            send_error.to_string(),
            "Error al enviar mensaje: Connection lost"
        );

        let invalid_id_error = ClientIdError::InvalidId("Empty ID".to_string());
        assert_eq!(
            invalid_id_error.to_string(),
            "ID de cliente inválido: Empty ID"
        );
    }

    #[test]
    fn test_client_id_error_from_send_error() {
        let (sender, receiver) = mpsc::channel();
        drop(receiver); // Esto hará que el envío falle

        let message = RespMessage::SimpleString("test".to_string());
        let send_result = sender.send(message);

        if let Err(send_error) = send_result {
            let client_error: ClientIdError = send_error.into();
            match client_error {
                ClientIdError::SendError(_) => (),
                _ => panic!("Se esperaba error SendError"),
            }
        } else {
            panic!("Se esperaba que el envío fallara");
        }
    }

    #[test]
    fn test_client_id_clone() {
        let (sender, _) = mpsc::channel();
        let client1 = ClientId::new("test_client".to_string(), sender);
        let client2 = client1.clone();

        assert_eq!(client1.get_id(), client2.get_id());
        assert_eq!(client1, client2);
    }

    #[test]
    fn test_client_id_debug() {
        let (sender, _) = mpsc::channel();
        let client = ClientId::new("test_client".to_string(), sender);
        let debug_str = format!("{:?}", client);
        assert!(debug_str.contains("test_client"));
    }
}
