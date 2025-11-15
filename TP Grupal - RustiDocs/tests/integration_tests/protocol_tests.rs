use crate::integration_tests::common::{parse_resp_from_bytes, serialize_resp_to_bytes};
use rustidocs::network::resp_message::RespMessage;

/// Tests para parsing de mensajes RESP simples
#[test]
fn test_resp_simple_string() {
    let test_message = "+OK\r\n";
    let parsed = parse_resp_from_bytes(test_message.as_bytes()).unwrap();
    match parsed {
        RespMessage::SimpleString(value) => assert_eq!(value, "OK"),
        _ => panic!("Expected SimpleString"),
    }
}

#[test]
fn test_resp_integer() {
    let test_message = ":123\r\n";
    let parsed = parse_resp_from_bytes(test_message.as_bytes()).unwrap();
    match parsed {
        RespMessage::Integer(value) => assert_eq!(value, 123),
        _ => panic!("Expected Integer"),
    }
}

#[test]
fn test_resp_error() {
    let test_message = "-Error message\r\n";
    let parsed = parse_resp_from_bytes(test_message.as_bytes()).unwrap();
    match parsed {
        RespMessage::SimpleError(value) => assert_eq!(value, "Error message"),
        _ => panic!("Expected SimpleError"),
    }
}

#[test]
fn test_resp_bulk_string() {
    let test_message = "$5\r\nHello\r\n";
    let parsed = parse_resp_from_bytes(test_message.as_bytes()).unwrap();
    match parsed {
        RespMessage::BulkString(Some(value)) => assert_eq!(value, b"Hello"),
        _ => panic!("Expected BulkString"),
    }
}

#[test]
fn test_resp_array() {
    let test_message = "*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n";
    let parsed = parse_resp_from_bytes(test_message.as_bytes()).unwrap();
    match parsed {
        RespMessage::Array(items) => {
            assert_eq!(items.len(), 3);
            if let RespMessage::BulkString(Some(cmd)) = &items[0] {
                assert_eq!(cmd, b"SET");
            } else {
                panic!("Expected first item to be BulkString");
            }
        }
        _ => panic!("Expected Array"),
    }
}

#[test]
fn test_resp_round_trip() {
    let original = RespMessage::SimpleString("Hello, World!".to_string());
    let serialized = serialize_resp_to_bytes(&original);
    let parsed = parse_resp_from_bytes(&serialized).unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn test_resp_serialization() {
    let original = RespMessage::BulkString(Some("Hello, World!".to_string().into()));
    let serialized = serialize_resp_to_bytes(&original);
    let parsed = parse_resp_from_bytes(&serialized).unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn test_resp_redis_commands() {
    let original = RespMessage::Array(vec![
        RespMessage::BulkString(Some("SET".to_string().into())),
        RespMessage::BulkString(Some("key".to_string().into())),
        RespMessage::BulkString(Some("value".to_string().into())),
    ]);
    let serialized = serialize_resp_to_bytes(&original);
    let parsed = parse_resp_from_bytes(&serialized).unwrap();
    assert_eq!(original, parsed);
}

/// Tests para casos edge y errores
#[test]
fn test_resp_partial_input() {
    let test_message = "*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue";
    let result = parse_resp_from_bytes(test_message.as_bytes());
    assert!(result.is_err());
}

#[test]
fn test_resp_invalid_input() {
    let test_messages = vec!["$5\r\nhello", "+OK"];

    for test_message in test_messages {
        let result = parse_resp_from_bytes(test_message.as_bytes());
        assert!(result.is_err(), "Should fail to parse: {:?}", test_message);
    }
}

/// Tests para arrays complejos
#[test]
fn test_resp_nested_array() {
    let original = RespMessage::Array(vec![
        RespMessage::BulkString(Some("SET".to_string().into())),
        RespMessage::BulkString(Some("test_key".to_string().into())),
        RespMessage::BulkString(Some("test_value".to_string().into())),
    ]);
    let serialized = serialize_resp_to_bytes(&original);
    let parsed = parse_resp_from_bytes(&serialized).unwrap();
    assert_eq!(original, parsed);
}

/// Tests para strings grandes
#[test]
fn test_resp_large_strings() {
    let large_string = "x".repeat(1000);
    let original = RespMessage::BulkString(Some(large_string.into()));
    let serialized = serialize_resp_to_bytes(&original);
    let parsed = parse_resp_from_bytes(&serialized).unwrap();
    assert_eq!(original, parsed);
}

/// Tests para strings Unicode
#[test]
fn test_resp_unicode_strings() {
    let unicode_string = "Hello, 世界! 🌍";
    let original = RespMessage::BulkString(Some(unicode_string.into()));
    let serialized = serialize_resp_to_bytes(&original);
    let parsed = parse_resp_from_bytes(&serialized).unwrap();
    assert_eq!(original, parsed);
}

/// Tests para arrays mixtos
#[test]
fn test_resp_mixed_array() {
    let original = RespMessage::Array(vec![
        RespMessage::SimpleString("OK".to_string()),
        RespMessage::Integer(42),
        RespMessage::BulkString(Some("test".to_string().into())),
    ]);
    let serialized = serialize_resp_to_bytes(&original);
    let parsed = parse_resp_from_bytes(&serialized).unwrap();
    assert_eq!(original, parsed);
}

/// Tests para respuestas de servidor
#[test]
fn test_server_response() {
    let response = RespMessage::SimpleString("OK".to_string());
    let serialized_response = serialize_resp_to_bytes(&response);
    assert_eq!(serialized_response, b"+OK\r\n");
}
