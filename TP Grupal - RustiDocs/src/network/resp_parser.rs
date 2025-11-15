//! Módulo de parser RESP (Redis Serialization Protocol)
//!
//! Este módulo permite parsear mensajes RESP desde un stream de entrada,
//! devolviendo un enum `RespMessage` o un error detallado.
use std::fmt;
use std::io::BufRead;
use std::str::FromStr;

use super::resp_message::RespMessage;

/// Enum de errores posibles al parsear RESP.
#[derive(Debug, Clone, PartialEq)]
pub enum RespParserError {
    /// Error de lectura de línea
    IoError(String),
    /// Prefijo desconocido
    UnknownPrefix(char),
    /// Error de parseo de número
    ParseIntError(String),
    /// Error de parseo de booleano
    ParseBoolError(String),
    /// Longitud inválida
    InvalidLength,
    /// Error de parseo de double
    ParseDoubleError(String),
    /// Error de formato
    FormatError(String),
}

impl fmt::Display for RespParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RespParserError::IoError(e) => write!(f, "Error de IO: {}", e),
            RespParserError::UnknownPrefix(c) => write!(f, "Prefijo desconocido: {}", c),
            RespParserError::ParseIntError(e) => write!(f, "Error parseando número: {}", e),
            RespParserError::ParseBoolError(e) => write!(f, "Error parseando booleano: {}", e),
            RespParserError::InvalidLength => write!(f, "Longitud inválida"),
            RespParserError::ParseDoubleError(e) => write!(f, "Error parseando double: {}", e),
            RespParserError::FormatError(e) => write!(f, "Error de formato: {}", e),
        }
    }
}

impl std::error::Error for RespParserError {}

pub struct RespParser;

/// Parsea una línea RESP desde un BufRead.
///
/// # Returns
///
/// `Result<RespMessage, RespParserError>`
pub fn parse_resp_line<R: BufRead>(reader: &mut R) -> Result<RespMessage, RespParserError> {
    let mut line = String::new();
    let n = reader
        .read_line(&mut line)
        .map_err(|e| RespParserError::IoError(e.to_string()))?;

    if n == 0 {
        return Err(RespParserError::FormatError("Fin de stream".to_string()));
    }
    if !line.ends_with("\r\n") {
        return Err(RespParserError::FormatError(
            "Línea no termina en CRLF".to_string(),
        ));
    }

    let prefix = line
        .chars()
        .next()
        .ok_or_else(|| RespParserError::FormatError("Línea vacía".to_string()))?;
    let content = line[1..].trim_end_matches("\r\n");

    match prefix {
        // Arrays
        '*' => {
            if content == "-1" {
                return Ok(RespMessage::BulkString(None));
            }
            let count = usize::from_str(content).map_err(|e: std::num::ParseIntError| {
                RespParserError::ParseIntError(e.to_string())
            })?;
            let mut items: Vec<RespMessage> = Vec::with_capacity(count);
            for _ in 0..count {
                items.push(parse_resp_line(reader)?);
            }
            // Verificar si es un comando QUIT
            if count == 1 {
                if let Some(RespMessage::BulkString(Some(bytes))) = items.first() {
                    if bytes.eq(b"QUIT") {
                        return Ok(RespMessage::Disconnect);
                    }
                }
            }
            Ok(RespMessage::Array(items))
        }
        // Integer
        ':' => {
            let value = content
                .parse::<i64>()
                .map_err(|e: std::num::ParseIntError| {
                    RespParserError::ParseIntError(e.to_string())
                })?;
            Ok(RespMessage::Integer(value))
        }
        // Simple string
        '+' => Ok(RespMessage::SimpleString(content.to_string())),
        // Simple error
        '-' => Ok(RespMessage::SimpleError(content.to_string())),
        // Boolean
        '#' => Ok(RespMessage::Boolean(parse_boolean(content)?)),
        // Null
        '_' => Ok(RespMessage::Null(None)),
        // Double
        ',' => {
            let value = content
                .parse::<f64>()
                .map_err(|e: std::num::ParseFloatError| {
                    RespParserError::ParseDoubleError(e.to_string())
                })?;
            Ok(RespMessage::Doubles(value))
        }
        '!' => {
            let len: isize = content.parse().map_err(|e: std::num::ParseIntError| {
                RespParserError::ParseIntError(e.to_string())
            })?;
            if len == -1 {
                Ok(RespMessage::BulkError(None))
            } else {
                let mut buf: String = String::new();
                reader
                    .read_line(&mut buf)
                    .map_err(|e| RespParserError::IoError(e.to_string()))?;
                if !buf.ends_with("\r\n") {
                    return Err(RespParserError::FormatError(
                        "BulkError no termina en CRLF".to_string(),
                    ));
                }
                let value = buf.trim_end_matches("\r\n").to_string();
                if value.len() != len as usize {
                    return Err(RespParserError::InvalidLength);
                }
                Ok(RespMessage::BulkError(Some(value.into_bytes())))
            }
        }
        // Bulk string
        '$' => {
            let len: isize = content.parse().map_err(|e: std::num::ParseIntError| {
                RespParserError::ParseIntError(e.to_string())
            })?;
            if len == -1 {
                Ok(RespMessage::BulkString(None))
            } else {
                let mut buf: String = String::new();
                reader
                    .read_line(&mut buf)
                    .map_err(|e| RespParserError::IoError(e.to_string()))?;
                if !buf.ends_with("\r\n") {
                    return Err(RespParserError::FormatError(
                        "BulkString no termina en CRLF".to_string(),
                    ));
                }
                let value = buf.trim_end_matches("\r\n").to_string();
                if value.len() != len as usize {
                    return Err(RespParserError::InvalidLength);
                }
                Ok(RespMessage::BulkString(Some(value.into_bytes())))
            }
        }
        _ => Err(RespParserError::UnknownPrefix(prefix)),
    }
}

fn parse_boolean(content: &str) -> Result<bool, RespParserError> {
    match content {
        "t" => Ok(true),
        "f" => Ok(false),
        _ => Err(RespParserError::ParseBoolError(format!(
            "Formato inválido: se esperaba 't' o 'f', se recibió '{}'.",
            content
        ))),
    }
}

#[cfg(test)]
mod resp_parse_tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn test_parse_simple_string() {
        let input = b"+OKA\r\n";
        let mut reader = BufReader::new(&input[..]);
        let result = parse_resp_line(&mut reader).unwrap();
        match result {
            RespMessage::SimpleString(value) => assert_eq!(value, "OKA"),
            _ => panic!("Expected a simple string"),
        }
    }

    #[test]
    fn test_parse_integer_positive() {
        let input = b":+123\r\n";
        let mut reader = BufReader::new(&input[..]);
        let result = parse_resp_line(&mut reader).unwrap();
        match result {
            RespMessage::Integer(value) => assert_eq!(value, 123),
            _ => panic!("Expected an integer"),
        }
    }

    #[test]
    fn test_parse_integer_negative() {
        let input = b":-123\r\n";
        let mut reader = BufReader::new(&input[..]);
        let result = parse_resp_line(&mut reader).unwrap();
        match result {
            RespMessage::Integer(value) => assert_eq!(value, -123),
            _ => panic!("Expected an integer"),
        }
    }

    #[test]
    fn test_bulk_error_invalid_length() {
        let input = b"!3\r\nSYNTAX invalid syntax\r\n";
        let mut reader = BufReader::new(&input[..]);
        let result = parse_resp_line(&mut reader);
        assert!(matches!(result, Err(RespParserError::InvalidLength)));
    }

    #[test]
    fn test_bulk_string_invalid_length() {
        let input = b"$5\r\nHelloWorld\r\n";
        let mut reader = BufReader::new(&input[..]);
        let result = parse_resp_line(&mut reader);
        assert!(matches!(result, Err(RespParserError::InvalidLength)));
    }

    #[test]
    fn test_simple_error() {
        let input = b"-Error message\r\n";
        let mut reader = BufReader::new(&input[..]);
        let result = parse_resp_line(&mut reader).unwrap();
        match result {
            RespMessage::SimpleError(value) => assert_eq!(value, "Error message"),
            _ => panic!("Expected a simple error"),
        }
    }

    #[test]
    fn test_parse_bulk_string() {
        let input = b"$5\r\nHello\r\n";
        let mut reader = BufReader::new(&input[..]);
        let result = parse_resp_line(&mut reader).unwrap();
        match result {
            RespMessage::BulkString(value) => {
                assert_eq!(value, Some("Hello".as_bytes().to_vec()))
            }
            _ => panic!("Expected a simple string"),
        }
    }

    #[test]
    fn test_parse_bulk_error() {
        let input = b"!21\r\nSYNTAX invalid syntax\r\n";
        let mut reader = BufReader::new(&input[..]);
        let result = parse_resp_line(&mut reader).unwrap();
        match result {
            RespMessage::BulkError(value) => {
                assert_eq!(
                    value,
                    Some("SYNTAX invalid syntax".to_string().into_bytes())
                )
            }
            _ => panic!("Expected a simple string"),
        }
    }

    #[test]
    fn test_boolean_with_true() {
        let input = b"#t\r\n";
        let mut reader = BufReader::new(&input[..]);
        let result = parse_resp_line(&mut reader).unwrap();
        match result {
            RespMessage::Boolean(value) => assert_eq!(value, true),
            _ => panic!("Expected a boolean"),
        }
    }

    #[test]
    fn test_boolean_with_false() {
        let input = b"#f\r\n";
        let mut reader = BufReader::new(&input[..]);
        let result = parse_resp_line(&mut reader).unwrap();
        match result {
            RespMessage::Boolean(value) => assert_eq!(value, false),
            _ => panic!("Expected a boolean"),
        }
    }

    #[test]
    fn test_null() {
        let input = b"_\r\n";
        let mut reader = BufReader::new(&input[..]);
        let result = parse_resp_line(&mut reader).unwrap();
        match result {
            RespMessage::Null(None) => (), // Test pasa si se devuelve Null(None)
            _ => panic!("Expected a null value"),
        }
    }

    #[test]
    fn test_double() {
        let input = b",3.14\r\n";
        let mut reader = BufReader::new(&input[..]);
        let result = parse_resp_line(&mut reader).unwrap();
        match result {
            RespMessage::Doubles(value) => assert_eq!(value, 3.14),
            _ => panic!("Expected a double"),
        }
    }
}
