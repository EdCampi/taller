use std::collections::HashSet;
use std::fmt;

/// Error que puede ocurrir durante el parsing de respuestas RESP.
#[derive(Debug, Clone, PartialEq)]
pub enum ResponseParserError {
    /// Error al parsear un entero
    IntegerParseError(String),
    /// Error al parsear una cadena
    StringParseError(String),
    /// Error al parsear una lista
    ListParseError(String),
    /// Error al parsear un set
    SetParseError(String),
    /// Error al formatear un mensaje RESP
    FormatError(String),
    /// Error al procesar una respuesta de cliente
    ClientResponseError(String),
    /// Error al parsear un valor nulo
    NullParseError(String),
    /// Error de formato inválido
    InvalidFormatError(String),
}

impl fmt::Display for ResponseParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResponseParserError::IntegerParseError(msg) => {
                write!(f, "Error al parsear entero: {}", msg)
            }
            ResponseParserError::StringParseError(msg) => {
                write!(f, "Error al parsear cadena: {}", msg)
            }
            ResponseParserError::ListParseError(msg) => {
                write!(f, "Error al parsear lista: {}", msg)
            }
            ResponseParserError::SetParseError(msg) => {
                write!(f, "Error al parsear set: {}", msg)
            }
            ResponseParserError::FormatError(msg) => {
                write!(f, "Error al formatear mensaje RESP: {}", msg)
            }
            ResponseParserError::ClientResponseError(msg) => {
                write!(f, "Error al procesar respuesta de cliente: {}", msg)
            }
            ResponseParserError::NullParseError(msg) => {
                write!(f, "Error al parsear valor nulo: {}", msg)
            }
            ResponseParserError::InvalidFormatError(msg) => {
                write!(f, "Error de formato inválido: {}", msg)
            }
        }
    }
}

impl std::error::Error for ResponseParserError {}

/// Formatea un comando en formato RESP (Redis Serialization Protocol).
///
/// # Arguments
///
/// * `command` - El comando a formatear
///
/// # Returns
///
/// `Result<String, ResponseParserError>` - El comando formateado en RESP o un error
pub fn format_resp_message(command: &str) -> Result<String, ResponseParserError> {
    if command.trim().is_empty() {
        return Err(ResponseParserError::FormatError(
            "Comando vacío no puede ser formateado".to_string(),
        ));
    }

    let parts: Vec<&str> = command.split_whitespace().collect();
    let mut resp = format!("*{}\r\n", parts.len());

    for part in parts {
        resp.push_str(&format!("${}\r\n", part.len()));
        resp.push_str(part);
        resp.push_str("\r\n");
    }

    Ok(resp)
}

/// Parsea un set desde una respuesta RESP.
///
/// # Arguments
///
/// * `resp` - La respuesta RESP a parsear
///
/// # Returns
///
/// `Result<HashSet<String>, ResponseParserError>` - El set parseado o un error
pub fn parse_set(resp: &str) -> Result<HashSet<String>, ResponseParserError> {
    if !resp.starts_with('*') {
        return Err(ResponseParserError::SetParseError(
            "Respuesta no comienza con '*' para set".to_string(),
        ));
    }

    let lineas: Vec<&str> = resp.split("\r\n").collect();
    if lineas.len() < 3 {
        return Err(ResponseParserError::SetParseError(
            "Respuesta demasiado corta para ser un set válido".to_string(),
        ));
    }

    let mut set = HashSet::new();
    for i in 2..lineas.len() {
        if i % 2 != 0 {
            continue;
        }
        if i < lineas.len() {
            set.insert(lineas[i].to_string());
        }
    }

    Ok(set)
}

/// Convierte un set a string para display.
///
/// # Arguments
///
/// * `set` - El set a convertir
///
/// # Returns
///
/// `String` - La representación en string del set
fn display_set(set: HashSet<String>) -> String {
    let aux: Vec<String> = set.iter().cloned().collect();
    display_list(aux)
}

/// Parsea una lista desde una respuesta RESP.
///
/// # Arguments
///
/// * `resp` - La respuesta RESP a parsear
///
/// # Returns
///
/// `Result<Vec<String>, ResponseParserError>` - La lista parseada o un error
pub fn parse_list(resp: &str) -> Result<Vec<String>, ResponseParserError> {
    if !resp.starts_with('*') {
        return Err(ResponseParserError::ListParseError(
            "Respuesta no comienza con '*' para lista".to_string(),
        ));
    }

    let parts: Vec<&str> = resp.split("\r\n").collect();
    if parts.len() < 3 {
        return Err(ResponseParserError::ListParseError(
            "Respuesta demasiado corta para ser una lista válida".to_string(),
        ));
    }

    let mut list = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 && i % 2 == 0 && !part.starts_with('$') {
            list.push(part.to_string());
        }
    }

    Ok(list)
}

/// Convierte una lista a string para display.
///
/// # Arguments
///
/// * `list` - La lista a convertir
///
/// # Returns
///
/// `String` - La representación en string de la lista
fn display_list(list: Vec<String>) -> String {
    if list.is_empty() {
        return "(empty list)".to_string();
    }

    let mut buffer = Vec::new();
    for (i, value) in list.iter().enumerate() {
        buffer.push(format!("{}) \"{}\"", i + 1, value));
    }

    buffer.join("\n")
}

/// Parsea una cadena desde una respuesta RESP.
///
/// # Arguments
///
/// * `resp` - La respuesta RESP a parsear
///
/// # Returns
///
/// `Result<String, ResponseParserError>` - La cadena parseada o un error
pub fn parse_string(resp: &str) -> Result<String, ResponseParserError> {
    if resp.is_empty() {
        return Err(ResponseParserError::StringParseError(
            "Respuesta vacía no puede ser parseada".to_string(),
        ));
    }

    if resp.starts_with('+') {
        // Maneja SimpleString como +OK\r\n
        let result = resp
            .trim_start_matches('+')
            .trim_end_matches("\r\n")
            .to_string();
        return Ok(result);
    }

    if resp.starts_with('$') {
        // Maneja BulkString como $2\r\nOK\r\n
        let parts: Vec<&str> = resp.split("\r\n").collect();
        for (i, part) in parts.iter().enumerate() {
            if part.starts_with('$') && i + 1 < parts.len() {
                return Ok(parts[i + 1].to_string());
            }
        }
        return Err(ResponseParserError::StringParseError(
            "Formato de BulkString inválido".to_string(),
        ));
    }

    Err(ResponseParserError::StringParseError(
        "Formato de string no reconocido".to_string(),
    ))
}

/// Parsea un entero desde una respuesta RESP.
///
/// # Arguments
///
/// * `key` - La respuesta RESP a parsear
///
/// # Returns
///
/// `Result<i64, ResponseParserError>` - El entero parseado o un error
pub fn parse_int(key: &str) -> Result<i64, ResponseParserError> {
    if !key.starts_with(':') {
        return Err(ResponseParserError::IntegerParseError(
            "Respuesta no comienza con ':' para entero".to_string(),
        ));
    }

    let content = key.trim_end_matches("\r\n");
    let parts: Vec<&str> = content.split(':').collect();

    if parts.len() != 2 {
        return Err(ResponseParserError::IntegerParseError(
            "Formato de entero inválido".to_string(),
        ));
    }

    parts[1]
        .parse::<i64>()
        .map_err(|e| ResponseParserError::IntegerParseError(e.to_string()))
}

/// Convierte un entero a string para display.
///
/// # Arguments
///
/// * `n` - El entero a convertir
///
/// # Returns
///
/// `String` - La representación en string del entero
fn display_int(n: i64) -> String {
    n.to_string()
}

/// Parsea un valor nulo desde una respuesta RESP.
///
/// # Arguments
///
/// * `resp` - La respuesta RESP a parsear
///
/// # Returns
///
/// `Result<Option<String>, ResponseParserError>` - El valor nulo parseado o un error
pub fn parse_null(resp: &str) -> Result<Option<String>, ResponseParserError> {
    if resp.starts_with('_') {
        return Ok(Some("(nil)".to_string()));
    }

    if resp.trim().is_empty() {
        return Ok(Some("(nil)".to_string()));
    }

    Ok(Some(resp.to_string()))
}

/// Procesa una respuesta de cliente y la convierte a formato legible.
///
/// # Arguments
///
/// * `res` - La respuesta del cliente
///
/// # Returns
///
/// `Result<String, ResponseParserError>` - La respuesta procesada o un error
pub fn parse_client_res(res: String) -> Result<String, ResponseParserError> {
    if res.is_empty() {
        return Err(ResponseParserError::ClientResponseError(
            "Respuesta vacía no puede ser procesada".to_string(),
        ));
    }

    let first_char = res.chars().next().ok_or_else(|| {
        ResponseParserError::ClientResponseError(
            "No se puede obtener el primer carácter".to_string(),
        )
    })?;

    match first_char {
        '+' => parse_string(&res),
        ':' => {
            let int_value = parse_int(&res)?;
            Ok(display_int(int_value))
        }
        '*' => {
            let list = parse_list(&res)?;
            Ok(display_list(list))
        }
        '~' => {
            let set = parse_set(&res)?;
            Ok(display_set(set))
        }
        '_' => {
            let null_value = parse_null(&res)?;
            Ok(null_value.unwrap_or_else(|| "(nil)".to_string()))
        }
        _ => Ok(res), // default: devolver la respuesta sin parsear
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_response_parser_error_display() {
        let error = ResponseParserError::IntegerParseError("test error".to_string());
        assert_eq!(error.to_string(), "Error al parsear entero: test error");
    }

    #[test]
    fn test_response_parser_error_debug() {
        let error = ResponseParserError::StringParseError("test error".to_string());
        assert_eq!(format!("{:?}", error), "StringParseError(\"test error\")");
    }

    #[test]
    fn test_format_resp_message_success() {
        let result = format_resp_message("SET key value");
        assert!(result.is_ok());
        let formatted = result.unwrap();
        assert_eq!(formatted, "*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n");
    }

    #[test]
    fn test_format_resp_message_empty() {
        let result = format_resp_message("");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ResponseParserError::FormatError(_)
        ));
    }

    #[test]
    fn test_format_resp_message_whitespace_only() {
        let result = format_resp_message("   ");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ResponseParserError::FormatError(_)
        ));
    }

    #[test]
    fn test_parse_set_success() {
        let resp = "*2\r\n$5\r\nHello\r\n$5\r\nWorld\r\n";
        let result = parse_set(resp);
        assert!(result.is_ok());
        let set = result.unwrap();
        assert_eq!(set.len(), 2);
        assert!(set.contains("Hello"));
        assert!(set.contains("World"));
    }

    #[test]
    fn test_parse_set_invalid_format() {
        let result = parse_set("+OK\r\n");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ResponseParserError::SetParseError(_)
        ));
    }

    #[test]
    fn test_parse_set_too_short() {
        let result = parse_set("*1\r\n");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ResponseParserError::SetParseError(_)
        ));
    }

    #[test]
    fn test_parse_list_success() {
        let resp = "*2\r\n$5\r\nHello\r\n$5\r\nWorld\r\n";
        let result = parse_list(resp);
        assert!(result.is_ok());
        let list = result.unwrap();
        assert_eq!(list, vec!["Hello", "World"]);
    }

    #[test]
    fn test_parse_list_invalid_format() {
        let result = parse_list("+OK\r\n");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ResponseParserError::ListParseError(_)
        ));
    }

    #[test]
    fn test_parse_list_too_short() {
        let result = parse_list("*1\r\n");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ResponseParserError::ListParseError(_)
        ));
    }

    #[test]
    fn test_display_list_empty() {
        let result = display_list(vec![]);
        assert_eq!(result, "(empty list)");
    }

    #[test]
    fn test_display_list_with_items() {
        let result = display_list(vec!["Hello".to_string(), "World".to_string()]);
        assert_eq!(result, "1) \"Hello\"\n2) \"World\"");
    }

    #[test]
    fn test_parse_string_simple_string() {
        let result = parse_string("+OK\r\n");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "OK");
    }

    #[test]
    fn test_parse_string_bulk_string() {
        let result = parse_string("$5\r\nHello\r\n");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello");
    }

    #[test]
    fn test_parse_string_empty() {
        let result = parse_string("");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ResponseParserError::StringParseError(_)
        ));
    }

    #[test]
    fn test_parse_string_invalid_format() {
        let result = parse_string("invalid");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ResponseParserError::StringParseError(_)
        ));
    }

    #[test]
    fn test_parse_int_success() {
        let result = parse_int(":42\r\n");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_parse_int_negative() {
        let result = parse_int(":-42\r\n");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), -42);
    }

    #[test]
    fn test_parse_int_invalid_format() {
        let result = parse_int("invalid");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ResponseParserError::IntegerParseError(_)
        ));
    }

    #[test]
    fn test_parse_int_invalid_number() {
        let result = parse_int(":not_a_number\r\n");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ResponseParserError::IntegerParseError(_)
        ));
    }

    #[test]
    fn test_display_int() {
        assert_eq!(display_int(42), "42");
        assert_eq!(display_int(-42), "-42");
        assert_eq!(display_int(0), "0");
    }

    #[test]
    fn test_parse_null_success() {
        let result = parse_null("_\r\n");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("(nil)".to_string()));
    }

    #[test]
    fn test_parse_null_empty() {
        let result = parse_null("");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("(nil)".to_string()));
    }

    #[test]
    fn test_parse_null_other_value() {
        let result = parse_null("some_value");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("some_value".to_string()));
    }

    #[test]
    fn test_parse_client_res_simple_string() {
        let result = parse_client_res("+OK\r\n".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "OK");
    }

    #[test]
    fn test_parse_client_res_integer() {
        let result = parse_client_res(":42\r\n".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "42");
    }

    #[test]
    fn test_parse_client_res_list() {
        let result = parse_client_res("*2\r\n$5\r\nHello\r\n$5\r\nWorld\r\n".to_string());
        assert!(result.is_ok());
        let expected = "1) \"Hello\"\n2) \"World\"";
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_parse_client_res_set() {
        let result = parse_client_res("*2\r\n$5\r\nHello\r\n$5\r\nWorld\r\n".to_string());
        assert!(result.is_ok());
        // El orden puede variar en sets, así que verificamos que contiene ambos elementos
        let output = result.unwrap();
        assert!(output.contains("Hello"));
        assert!(output.contains("World"));
    }

    #[test]
    fn test_parse_client_res_null() {
        let result = parse_client_res("_\r\n".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "(nil)");
    }

    #[test]
    fn test_parse_client_res_empty() {
        let result = parse_client_res("".to_string());
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ResponseParserError::ClientResponseError(_)
        ));
    }

    #[test]
    fn test_parse_client_res_unknown_format() {
        let result = parse_client_res("unknown_format".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "unknown_format");
    }

    #[test]
    fn test_display_set() {
        let mut set = HashSet::new();
        set.insert("Hello".to_string());
        set.insert("World".to_string());

        let result = display_set(set);
        // El orden puede variar, así que verificamos que contiene ambos elementos
        assert!(result.contains("Hello"));
        assert!(result.contains("World"));
        assert!(result.contains("1)"));
        assert!(result.contains("2)"));
    }

    #[test]
    fn test_integration_format_and_parse() {
        let command = "SET key value";
        let formatted = format_resp_message(command).unwrap();
        let parsed_list = parse_list(&formatted).unwrap();

        assert_eq!(parsed_list, vec!["SET", "key", "value"]);
    }
}
