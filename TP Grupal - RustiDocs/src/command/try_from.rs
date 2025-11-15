//! Implementación de conversiones TryFrom para comandos
//!
//! Este módulo proporciona conversiones seguras entre tipos de datos,
//! específicamente la conversión de RespMessage a Instruction.
//!
//! # Errores
//! Todas las conversiones retornan un enum TryFromError para manejo robusto de errores.

use crate::command::Instruction;
use crate::network::resp_message::RespMessage;
pub use std::convert::TryFrom;

/// Errores específicos de conversión TryFrom
#[derive(Debug, Clone, PartialEq)]
pub enum TryFromError {
    /// Error cuando el primer elemento no es una cadena válida
    InvalidInstructionName(String),
    /// Error cuando se encuentra un BulkString nulo como nombre de instrucción
    NullInstructionName,
    /// Error cuando el primer elemento no es una cadena
    NonStringInstructionName,
    /// Error de codificación UTF-8 en argumentos
    InvalidUtf8InArgument(String),
    /// Error de codificación UTF-8 en nombre de instrucción
    InvalidUtf8InInstructionName,
    /// Error de codificación UTF-8 en BulkError
    InvalidUtf8InBulkError,
    /// Error cuando se encuentran arrays anidados no soportados
    NestedArraysNotSupported,
    /// Error cuando se espera un array no vacío
    ExpectedNonEmptyArray,
    /// Error genérico de conversión
    ConversionError(String),
}

impl std::fmt::Display for TryFromError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TryFromError::InvalidInstructionName(msg) => {
                write!(f, "Invalid instruction name: {}", msg)
            }
            TryFromError::NullInstructionName => {
                write!(f, "Null BulkString as instruction name")
            }
            TryFromError::NonStringInstructionName => {
                write!(f, "First element must be a string as instruction name")
            }
            TryFromError::InvalidUtf8InArgument(msg) => {
                write!(f, "Invalid UTF-8 in argument: {}", msg)
            }
            TryFromError::InvalidUtf8InInstructionName => {
                write!(f, "Invalid UTF-8 in instruction name")
            }
            TryFromError::InvalidUtf8InBulkError => {
                write!(f, "Invalid UTF-8 in BulkError")
            }
            TryFromError::NestedArraysNotSupported => {
                write!(f, "Nested arrays not supported in arguments")
            }
            TryFromError::ExpectedNonEmptyArray => {
                write!(f, "Expected a non-empty Array message for Instruction")
            }
            TryFromError::ConversionError(msg) => {
                write!(f, "Conversion error: {}", msg)
            }
        }
    }
}

impl std::error::Error for TryFromError {}

impl TryFrom<RespMessage> for Instruction {
    type Error = TryFromError;

    /// Convierte un RespMessage en una Instruction
    ///
    /// # Arguments
    ///
    /// * `msg` - El mensaje RespMessage a convertir
    ///
    /// # Returns
    ///
    /// Un Result que contiene la Instruction si la conversión es exitosa,
    /// o un TryFromError si falla.
    ///
    /// # Examples
    ///
    /// ```
    /// use rustidocs::command::{Instruction, try_from::TryFromError};
    /// use rustidocs::network::resp_message::RespMessage;
    /// use std::convert::TryFrom;
    ///
    /// let msg = RespMessage::Array(vec![
    ///     RespMessage::SimpleString("GET".to_string()),
    ///     RespMessage::SimpleString("key".to_string()),
    /// ]);
    /// let instruction = Instruction::try_from(msg).unwrap();
    /// assert_eq!(instruction.instruction_type, "GET");
    /// ```
    fn try_from(msg: RespMessage) -> Result<Self, Self::Error> {
        match msg {
            RespMessage::Array(mut elements) if !elements.is_empty() => {
                let first = elements.remove(0);
                let instruction_type = match first {
                    RespMessage::SimpleString(s) => s,
                    RespMessage::BulkString(Some(bytes)) => String::from_utf8(bytes)
                        .map_err(|_| TryFromError::InvalidUtf8InInstructionName)?,
                    RespMessage::BulkString(None) => {
                        return Err(TryFromError::NullInstructionName);
                    }
                    _ => {
                        return Err(TryFromError::NonStringInstructionName);
                    }
                };

                let mut arguments = Vec::new();
                for (index, elem) in elements.into_iter().enumerate() {
                    let arg = match elem {
                        RespMessage::SimpleString(s) => s,
                        RespMessage::BulkString(Some(bytes)) => {
                            String::from_utf8(bytes).map_err(|_| {
                                TryFromError::InvalidUtf8InArgument(format!(
                                    "at position {}",
                                    index
                                ))
                            })?
                        }
                        RespMessage::BulkString(None) => "null".to_string(),
                        RespMessage::Integer(i) => i.to_string(),
                        RespMessage::Boolean(b) => b.to_string(),
                        RespMessage::Doubles(d) => d.to_string(),
                        RespMessage::Null(_) => "null".to_string(),
                        RespMessage::Error(e) | RespMessage::SimpleError(e) => {
                            format!("ERR: {e}")
                        }
                        RespMessage::BulkError(Some(e)) => {
                            let str_err = String::from_utf8(e)
                                .map_err(|_| TryFromError::InvalidUtf8InBulkError)?;
                            format!("ERR: {str_err}")
                        }
                        RespMessage::BulkError(None) => "ERR: null".to_string(),
                        RespMessage::Array(_) => {
                            return Err(TryFromError::NestedArraysNotSupported);
                        }
                        RespMessage::Disconnect => "DISCONNECT".to_string(),
                    };
                    arguments.push(arg);
                }
                Ok(Instruction::new(instruction_type, arguments))
            }
            RespMessage::Disconnect => Ok(Instruction::new("DISCONNECT".to_string(), Vec::new())),
            _ => Err(TryFromError::ExpectedNonEmptyArray),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Instruction;

    #[test]
    fn test_try_from_error_display() {
        let error = TryFromError::InvalidInstructionName("test".to_string());
        assert_eq!(error.to_string(), "Invalid instruction name: test");
    }

    #[test]
    fn test_try_from_error_debug() {
        let error = TryFromError::NullInstructionName;
        assert_eq!(format!("{:?}", error), "NullInstructionName");
    }

    #[test]
    fn test_try_from_simple_string_instruction() {
        let msg = RespMessage::Array(vec![
            RespMessage::SimpleString("GET".to_string()),
            RespMessage::SimpleString("key".to_string()),
        ]);
        let instruction = Instruction::try_from(msg).unwrap();
        assert_eq!(instruction.instruction_type, "GET");
        assert_eq!(instruction.arguments, &["key"]);
    }

    #[test]
    fn test_try_from_bulk_string_instruction() {
        let msg = RespMessage::Array(vec![
            RespMessage::BulkString(Some(b"SET".to_vec())),
            RespMessage::BulkString(Some(b"key".to_vec())),
            RespMessage::BulkString(Some(b"value".to_vec())),
        ]);
        let instruction = Instruction::try_from(msg).unwrap();
        assert_eq!(instruction.instruction_type, "SET");
        assert_eq!(instruction.arguments, &["key", "value"]);
    }

    #[test]
    fn test_try_from_mixed_types() {
        let msg = RespMessage::Array(vec![
            RespMessage::SimpleString("LPUSH".to_string()),
            RespMessage::SimpleString("list".to_string()),
            RespMessage::Integer(42),
            RespMessage::Boolean(true),
        ]);
        let instruction = Instruction::try_from(msg).unwrap();
        assert_eq!(instruction.instruction_type, "LPUSH");
        assert_eq!(instruction.arguments, &["list", "42", "true"]);
    }

    #[test]
    fn test_try_from_disconnect_message() {
        let msg = RespMessage::Disconnect;
        let instruction = Instruction::try_from(msg).unwrap();
        assert_eq!(instruction.instruction_type, "DISCONNECT");
        assert!(instruction.arguments.is_empty());
    }

    #[test]
    fn test_try_from_empty_array() {
        let msg = RespMessage::Array(vec![]);
        let result = Instruction::try_from(msg);
        assert!(matches!(result, Err(TryFromError::ExpectedNonEmptyArray)));
    }

    #[test]
    fn test_try_from_null_bulk_string_instruction() {
        let msg = RespMessage::Array(vec![
            RespMessage::BulkString(None),
            RespMessage::SimpleString("key".to_string()),
        ]);
        let result = Instruction::try_from(msg);
        assert!(matches!(result, Err(TryFromError::NullInstructionName)));
    }

    #[test]
    fn test_try_from_non_string_instruction_name() {
        let msg = RespMessage::Array(vec![
            RespMessage::Integer(42),
            RespMessage::SimpleString("key".to_string()),
        ]);
        let result = Instruction::try_from(msg);
        assert!(matches!(
            result,
            Err(TryFromError::NonStringInstructionName)
        ));
    }

    #[test]
    fn test_try_from_invalid_utf8_instruction_name() {
        let msg = RespMessage::Array(vec![
            RespMessage::BulkString(Some(vec![0xFF, 0xFE])), // Invalid UTF-8
            RespMessage::SimpleString("key".to_string()),
        ]);
        let result = Instruction::try_from(msg);
        assert!(matches!(
            result,
            Err(TryFromError::InvalidUtf8InInstructionName)
        ));
    }

    #[test]
    fn test_try_from_invalid_utf8_argument() {
        let msg = RespMessage::Array(vec![
            RespMessage::SimpleString("SET".to_string()),
            RespMessage::BulkString(Some(vec![0xFF, 0xFE])), // Invalid UTF-8
        ]);
        let result = Instruction::try_from(msg);
        assert!(matches!(
            result,
            Err(TryFromError::InvalidUtf8InArgument(_))
        ));
    }

    #[test]
    fn test_try_from_nested_arrays() {
        let msg = RespMessage::Array(vec![
            RespMessage::SimpleString("COMMAND".to_string()),
            RespMessage::Array(vec![RespMessage::SimpleString("nested".to_string())]),
        ]);
        let result = Instruction::try_from(msg);
        assert!(matches!(
            result,
            Err(TryFromError::NestedArraysNotSupported)
        ));
    }

    #[test]
    fn test_try_from_non_array_message() {
        let msg = RespMessage::SimpleString("GET".to_string());
        let result = Instruction::try_from(msg);
        assert!(matches!(result, Err(TryFromError::ExpectedNonEmptyArray)));
    }

    #[test]
    fn test_try_from_error_messages() {
        let msg = RespMessage::Array(vec![
            RespMessage::SimpleString("COMMAND".to_string()),
            RespMessage::Error("test error".to_string()),
            RespMessage::BulkError(Some(b"bulk error".to_vec())),
        ]);
        let instruction = Instruction::try_from(msg).unwrap();
        assert_eq!(instruction.instruction_type, "COMMAND");
        assert_eq!(
            instruction.arguments,
            &["ERR: test error", "ERR: bulk error"]
        );
    }

    #[test]
    fn test_try_from_null_values() {
        let msg = RespMessage::Array(vec![
            RespMessage::SimpleString("COMMAND".to_string()),
            RespMessage::BulkString(None),
            RespMessage::Null(None),
        ]);
        let instruction = Instruction::try_from(msg).unwrap();
        assert_eq!(instruction.instruction_type, "COMMAND");
        assert_eq!(instruction.arguments, &["null", "null"]);
    }
}
