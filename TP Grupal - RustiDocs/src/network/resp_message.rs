//! Módulo de mensajes RESP (Redis Serialization Protocol)
//!
//! Este módulo define los tipos de mensajes RESP que se utilizan para
//! la comunicación entre clientes y servidor Redis.
//!
//! # Características
//! - Soporte completo para todos los tipos de datos RESP
//! - Serialización a bytes para transmisión de red
//! - Conversión desde tipos de respuesta internos
//! - Manejo de valores nulos y errores
//!
//! # Tipos de mensajes RESP
//!
//! - **SimpleString**: Cadenas simples que comienzan con `+`
//! - **Error**: Mensajes de error que comienzan con `-`
//! - **Integer**: Números enteros que comienzan con `:`
//! - **BulkString**: Cadenas de longitud variable que comienzan con `$`
//! - **Array**: Arrays que comienzan con `*`
//! - **Boolean**: Valores booleanos
//! - **Doubles**: Números de punto flotante que comienzan con `!`
//! - **Null**: Valores nulos representados con `_`

use crate::command::types::ResponseType;
use std::fmt;

/// Error que puede ocurrir durante el manejo de mensajes RESP.
#[derive(Debug, Clone, PartialEq)]
pub enum RespMessageError {
    /// Error al convertir string a bytes
    StringConversionError(String),
    /// Error al formatear número
    NumberFormatError(String),
    /// Error al serializar array
    ArraySerializationError(String),
    /// Error al convertir ResponseType
    ResponseTypeConversionError(String),
}

impl fmt::Display for RespMessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RespMessageError::StringConversionError(msg) => {
                write!(f, "Error al convertir string a bytes: {}", msg)
            }
            RespMessageError::NumberFormatError(msg) => {
                write!(f, "Error al formatear número: {}", msg)
            }
            RespMessageError::ArraySerializationError(msg) => {
                write!(f, "Error al serializar array: {}", msg)
            }
            RespMessageError::ResponseTypeConversionError(msg) => {
                write!(f, "Error al convertir ResponseType: {}", msg)
            }
        }
    }
}

impl std::error::Error for RespMessageError {}

/// Enum que representa los diferentes tipos de mensajes RESP.
///
/// RESP (Redis Serialization Protocol) es un protocolo de serialización
/// utilizado por Redis para la comunicación cliente-servidor.
#[derive(Clone, Debug, PartialEq)]
pub enum RespMessage {
    /// Cadena simple que comienza con `+`
    SimpleString(String),
    /// Mensaje de error que comienza con `-`
    Error(String),
    /// Número entero que comienza con `:`
    Integer(i64),
    /// Cadena de longitud variable que comienza con `$`
    /// Puede ser `None` para una cadena nula
    BulkString(Option<Vec<u8>>),
    /// Array que comienza con `*`
    Array(Vec<RespMessage>),
    /// Error simple que comienza con `-`
    SimpleError(String),
    /// Valor booleano
    Boolean(bool),
    /// Error de longitud variable que comienza con `!`
    /// Puede ser `None` para un error nulo
    BulkError(Option<Vec<u8>>),
    /// Valor nulo representado con `_`
    /// Puede ser `None` para un valor nulo
    Null(Option<()>),
    /// Número de punto flotante que comienza con `!`
    Doubles(f64),
    /// Mensaje de desconexión
    Disconnect,
}

/* TIPOS A IMPLEMENTAR:
Array(Vec<RespValue>), ya estaria??

Doubles(f64),
BigNumber(String),
*/

impl RespMessage {
    /* (Comento porque saltaba warning no usada)
    fn generate_array_response<I>(items: I) -> RespMessage
    where
        I: IntoIterator<Item = String>,
    {
        let inner = items
            .into_iter()
            .map(|item| RespMessage::BulkString(Some(item.into_bytes())))
            .collect();
        RespMessage::Array(inner)
    }
    */
    /// Convierte un `ResponseType` interno a un `RespMessage`.
    ///
    /// # Arguments
    ///
    /// * `response` - El tipo de respuesta interno a convertir
    ///
    /// # Returns
    ///
    /// `RespMessage` - El mensaje RESP convertido
    pub fn from_response(response: ResponseType) -> Self {
        match response {
            ResponseType::Str(s) if s == "OK" || s == "PONG" => RespMessage::SimpleString(s),
            ResponseType::Str(s) => {
                let bytes = s.into_bytes();
                RespMessage::BulkString(Some(bytes))
            }
            ResponseType::Int(n) => RespMessage::Integer(n as i64),
            ResponseType::List(items) => {
                let inner: Vec<RespMessage> = items
                    .into_iter()
                    .map(|item| {
                        let bytes = item.into_bytes();
                        RespMessage::BulkString(Some(bytes))
                    })
                    .collect();
                RespMessage::Array(inner)
            }
            ResponseType::Set(set_items) => {
                let inner: Vec<RespMessage> = set_items
                    .into_iter()
                    .map(|item| {
                        let bytes = item.into_bytes();
                        RespMessage::BulkString(Some(bytes))
                    })
                    .collect();
                RespMessage::Array(inner)
            }
            ResponseType::Null(_) => RespMessage::Null(None),
        }
    }

    /// Crea un mensaje de error RESP.
    ///
    /// # Arguments
    ///
    /// * `msg` - El mensaje de error
    ///
    /// # Returns
    ///
    /// `RespMessage` - Un mensaje de error RESP
    pub fn error(msg: String) -> Self {
        RespMessage::Error(msg)
    }

    /// Convierte el mensaje RESP a bytes para transmisión de red.
    ///
    /// # Returns
    ///
    /// `Vec<u8>` - Los bytes serializados del mensaje
    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            RespMessage::SimpleString(s) => {
                let formatted = format!("+{}\r\n", s);
                formatted.into_bytes()
            }
            RespMessage::Error(e) => {
                let formatted = format!("-{}\r\n", e);
                formatted.into_bytes()
            }
            RespMessage::Integer(n) => {
                let formatted = format!(":{}\r\n", n);
                formatted.into_bytes()
            }
            RespMessage::BulkString(Some(bs)) => {
                let mut result = format!("${}\r\n", bs.len()).into_bytes();
                result.extend(bs.clone());
                result.extend(b"\r\n");
                result
            }
            RespMessage::BulkString(None) => b"-1\r\n".to_vec(),
            RespMessage::Array(arr) => {
                let mut out = format!("*{}\r\n", arr.len()).into_bytes();
                for item in arr {
                    out.extend(item.as_bytes());
                }
                out
            }
            RespMessage::SimpleError(e) => {
                let formatted = format!("-{}\r\n", e);
                formatted.into_bytes()
            }
            RespMessage::Boolean(b) => {
                let formatted = format!("{}{}\r\n", if *b { '+' } else { '-' }, "");
                formatted.into_bytes()
            }
            RespMessage::BulkError(None) => b"-1\r\n".to_vec(),
            RespMessage::BulkError(Some(bs)) => {
                let mut result = format!("!{}\r\n", bs.len()).into_bytes();
                result.extend(bs.clone());
                result.extend(b"\r\n");
                result
            }
            RespMessage::Null(_) => b"_\r\n".to_vec(),
            RespMessage::Doubles(d) => {
                let formatted = format!("!{}\r\n", d);
                formatted.into_bytes()
            }
            RespMessage::Disconnect => b"DISCONNECT\r\n".to_vec(),
        }
    }

    /// Obtiene el tipo de mensaje como string para debugging.
    ///
    /// # Returns
    ///
    /// String que representa el tipo de mensaje
    pub fn get_type_name(&self) -> &'static str {
        match self {
            RespMessage::SimpleString(_) => "SimpleString",
            RespMessage::Error(_) => "Error",
            RespMessage::Integer(_) => "Integer",
            RespMessage::BulkString(_) => "BulkString",
            RespMessage::Array(_) => "Array",
            RespMessage::SimpleError(_) => "SimpleError",
            RespMessage::Boolean(_) => "Boolean",
            RespMessage::BulkError(_) => "BulkError",
            RespMessage::Null(_) => "Null",
            RespMessage::Doubles(_) => "Doubles",
            RespMessage::Disconnect => "Disconnect",
        }
    }

    /// Verifica si el mensaje es un error.
    ///
    /// # Returns
    ///
    /// `true` si el mensaje es un error, `false` en caso contrario
    pub fn is_error(&self) -> bool {
        matches!(
            self,
            RespMessage::Error(_) | RespMessage::SimpleError(_) | RespMessage::BulkError(_)
        )
    }

    /// Verifica si el mensaje es nulo.
    ///
    /// # Returns
    ///
    /// `true` si el mensaje es nulo, `false` en caso contrario
    pub fn is_null(&self) -> bool {
        matches!(
            self,
            RespMessage::Null(_) | RespMessage::BulkString(None) | RespMessage::BulkError(None)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::types::ResponseType;
    use std::collections::HashSet;

    #[test]
    fn test_resp_message_error_display() {
        let error = RespMessageError::StringConversionError("test error".to_string());
        assert_eq!(
            error.to_string(),
            "Error al convertir string a bytes: test error"
        );
    }

    #[test]
    fn test_resp_message_error_debug() {
        let error = RespMessageError::NumberFormatError("test error".to_string());
        assert_eq!(format!("{:?}", error), "NumberFormatError(\"test error\")");
    }

    #[test]
    fn test_simple_string_serialization() {
        let msg = RespMessage::SimpleString("OK".to_string());
        let bytes = msg.as_bytes();
        assert_eq!(bytes, b"+OK\r\n");
    }

    #[test]
    fn test_error_serialization() {
        let msg = RespMessage::Error("Error message".to_string());
        let bytes = msg.as_bytes();
        assert_eq!(bytes, b"-Error message\r\n");
    }

    #[test]
    fn test_integer_serialization() {
        let msg = RespMessage::Integer(42);
        let bytes = msg.as_bytes();
        assert_eq!(bytes, b":42\r\n");
    }

    #[test]
    fn test_bulk_string_serialization() {
        let msg = RespMessage::BulkString(Some(b"Hello".to_vec()));
        let bytes = msg.as_bytes();
        assert_eq!(bytes, b"$5\r\nHello\r\n");
    }

    #[test]
    fn test_bulk_string_null_serialization() {
        let msg = RespMessage::BulkString(None);
        let bytes = msg.as_bytes();
        assert_eq!(bytes, b"-1\r\n");
    }

    #[test]
    fn test_array_serialization() {
        let msg = RespMessage::Array(vec![
            RespMessage::BulkString(Some(b"Hello".to_vec())),
            RespMessage::BulkString(Some(b"World".to_vec())),
        ]);
        let bytes = msg.as_bytes();
        assert_eq!(bytes, b"*2\r\n$5\r\nHello\r\n$5\r\nWorld\r\n");
    }

    #[test]
    fn test_boolean_serialization() {
        let msg_true = RespMessage::Boolean(true);
        let bytes_true = msg_true.as_bytes();
        assert_eq!(bytes_true, b"+\r\n");

        let msg_false = RespMessage::Boolean(false);
        let bytes_false = msg_false.as_bytes();
        assert_eq!(bytes_false, b"-\r\n");
    }

    #[test]
    fn test_doubles_serialization() {
        let msg = RespMessage::Doubles(3.14);
        let bytes = msg.as_bytes();
        assert_eq!(bytes, b"!3.14\r\n");
    }

    #[test]
    fn test_null_serialization() {
        let msg = RespMessage::Null(None);
        let bytes = msg.as_bytes();
        assert_eq!(bytes, b"_\r\n");
    }

    #[test]
    fn test_disconnect_serialization() {
        let msg = RespMessage::Disconnect;
        let bytes = msg.as_bytes();
        assert_eq!(bytes, b"DISCONNECT\r\n");
    }

    #[test]
    fn test_from_response_str() {
        let response = ResponseType::Str("OK".to_string());
        let msg = RespMessage::from_response(response);
        assert_eq!(msg, RespMessage::SimpleString("OK".to_string()));
    }

    #[test]
    fn test_from_response_bulk_string() {
        let response = ResponseType::Str("Hello".to_string());
        let msg = RespMessage::from_response(response);
        assert_eq!(msg, RespMessage::BulkString(Some(b"Hello".to_vec())));
    }

    #[test]
    fn test_from_response_int() {
        let response = ResponseType::Int(42);
        let msg = RespMessage::from_response(response);
        assert_eq!(msg, RespMessage::Integer(42));
    }

    #[test]
    fn test_from_response_list() {
        let response = ResponseType::List(vec!["Hello".to_string(), "World".to_string()]);
        let msg = RespMessage::from_response(response);
        let expected = RespMessage::Array(vec![
            RespMessage::BulkString(Some(b"Hello".to_vec())),
            RespMessage::BulkString(Some(b"World".to_vec())),
        ]);
        assert_eq!(msg, expected);
    }

    #[test]
    fn test_from_response_set() {
        let mut set = HashSet::new();
        set.insert("Hello".to_string());
        set.insert("World".to_string());
        let response = ResponseType::Set(set);
        let msg = RespMessage::from_response(response);

        // Verificar que es un Array con 2 elementos
        match msg {
            RespMessage::Array(items) => {
                assert_eq!(items.len(), 2);

                // Convertir los elementos a strings para comparación
                let mut strings: Vec<String> = items
                    .into_iter()
                    .filter_map(|item| {
                        if let RespMessage::BulkString(Some(bytes)) = item {
                            String::from_utf8(bytes).ok()
                        } else {
                            None
                        }
                    })
                    .collect();

                // Ordenar para comparación independiente del orden
                strings.sort();

                // Verificar que contiene los elementos esperados
                assert_eq!(strings, vec!["Hello".to_string(), "World".to_string()]);
            }
            _ => panic!("Expected Array, got {:?}", msg),
        }
    }

    #[test]
    fn test_from_response_null() {
        let response = ResponseType::Null(None);
        let msg = RespMessage::from_response(response);
        assert_eq!(msg, RespMessage::Null(None));
    }

    #[test]
    fn test_error_creation() {
        let msg = RespMessage::error("Test error".to_string());
        assert_eq!(msg, RespMessage::Error("Test error".to_string()));
    }

    #[test]
    fn test_get_type_name() {
        assert_eq!(
            RespMessage::SimpleString("".to_string()).get_type_name(),
            "SimpleString"
        );
        assert_eq!(RespMessage::Error("".to_string()).get_type_name(), "Error");
        assert_eq!(RespMessage::Integer(0).get_type_name(), "Integer");
        assert_eq!(RespMessage::BulkString(None).get_type_name(), "BulkString");
        assert_eq!(RespMessage::Array(vec![]).get_type_name(), "Array");
        assert_eq!(RespMessage::Boolean(true).get_type_name(), "Boolean");
        assert_eq!(RespMessage::Doubles(0.0).get_type_name(), "Doubles");
        assert_eq!(RespMessage::Disconnect.get_type_name(), "Disconnect");
    }

    #[test]
    fn test_is_error() {
        assert!(RespMessage::Error("".to_string()).is_error());
        assert!(RespMessage::SimpleError("".to_string()).is_error());
        assert!(RespMessage::BulkError(None).is_error());
        assert!(!RespMessage::SimpleString("".to_string()).is_error());
        assert!(!RespMessage::Integer(0).is_error());
    }

    #[test]
    fn test_is_null() {
        assert!(RespMessage::Null(None).is_null());
        assert!(RespMessage::BulkString(None).is_null());
        assert!(RespMessage::BulkError(None).is_null());
        assert!(!RespMessage::SimpleString("".to_string()).is_null());
        assert!(!RespMessage::BulkString(Some(vec![])).is_null());
    }

    #[test]
    fn test_clone_and_equality() {
        let msg1 = RespMessage::SimpleString("test".to_string());
        let msg2 = msg1.clone();
        assert_eq!(msg1, msg2);
    }

    #[test]
    fn test_debug_format() {
        let msg = RespMessage::SimpleString("test".to_string());
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("SimpleString"));
        assert!(debug_str.contains("test"));
    }
}
