use rustidocs::network::{RespMessage, resp_parser::parse_resp_line};
use std::io::BufReader;

// Tests para el protocolo RESP (Redis Serialization Protocol)
#[test]
fn test_resp_simple_string() {
    // Test simple string: "+OK\r\n"
    let input = b"+OK\r\n";
    let mut reader = BufReader::new(&input[..]);
    let result = parse_resp_line(&mut reader).expect("Failed to parse simple string");
    assert!(matches!(result, RespMessage::SimpleString(ref s) if s == "OK"));
}

#[test]
fn test_resp_error() {
    // Test error: "-ERR unknown command\r\n"
    let input = b"-ERR unknown command\r\n";
    let mut reader = BufReader::new(&input[..]);
    let result = parse_resp_line(&mut reader).expect("Failed to parse error");
    assert!(matches!(result, RespMessage::SimpleError(ref s) if s == "ERR unknown command"));
}

#[test]
fn test_resp_integer() {
    // Test integer: ":1000\r\n"
    let input = b":1000\r\n";
    let mut reader = BufReader::new(&input[..]);
    let result = parse_resp_line(&mut reader).expect("Failed to parse integer");
    assert!(matches!(result, RespMessage::Integer(1000)));

    // Test negative integer: ":-1000\r\n"
    let input = b":-1000\r\n";
    let mut reader = BufReader::new(&input[..]);
    let result = parse_resp_line(&mut reader).expect("Failed to parse negative integer");
    assert!(matches!(result, RespMessage::Integer(-1000)));
}

#[test]
fn test_resp_bulk_string() {
    // Test bulk string: "$6\r\nfoobar\r\n"
    let input = b"$6\r\nfoobar\r\n";
    let mut reader = BufReader::new(&input[..]);
    let result = parse_resp_line(&mut reader).expect("Failed to parse bulk string");
    assert!(matches!(result, RespMessage::BulkString(Some(ref bytes)) if bytes == b"foobar"));

    // Test empty bulk string: "$0\r\n\r\n"
    let input = b"$0\r\n\r\n";
    let mut reader = BufReader::new(&input[..]);
    let result = parse_resp_line(&mut reader).expect("Failed to parse empty bulk string");
    assert!(matches!(result, RespMessage::BulkString(Some(ref bytes)) if bytes.is_empty()));

    // Test null bulk string: "$-1\r\n"
    let input = b"$-1\r\n";
    let mut reader = BufReader::new(&input[..]);
    let result = parse_resp_line(&mut reader).expect("Failed to parse null bulk string");
    assert!(matches!(result, RespMessage::BulkString(None)));
}

#[test]
fn test_resp_array() {
    // Test array: "*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"
    let input = b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n";
    let mut reader = BufReader::new(&input[..]);
    let result = parse_resp_line(&mut reader).expect("Failed to parse array");
    match result {
        RespMessage::Array(items) => {
            assert_eq!(items.len(), 2);
            assert!(
                matches!(items[0], RespMessage::BulkString(Some(ref bytes)) if bytes == b"foo")
            );
            assert!(
                matches!(items[1], RespMessage::BulkString(Some(ref bytes)) if bytes == b"bar")
            );
        }
        _ => panic!("Expected Array response"),
    }

    // Test empty array: "*0\r\n"
    let input = b"*0\r\n";
    let mut reader = BufReader::new(&input[..]);
    let result = parse_resp_line(&mut reader).expect("Failed to parse empty array");
    match result {
        RespMessage::Array(items) => {
            assert_eq!(items.len(), 0);
        }
        _ => panic!("Expected Array response"),
    }

    // Test null array: "*-1\r\n"
    let input = b"*-1\r\n";
    let mut reader = BufReader::new(&input[..]);
    let result = parse_resp_line(&mut reader).expect("Failed to parse null array");
    assert!(matches!(result, RespMessage::BulkString(None)));
}

#[test]
fn test_resp_mixed_array() {
    // Test array with mixed types: "*3\r\n:1\r\n:2\r\n:3\r\n"
    let input = b"*3\r\n:1\r\n:2\r\n:3\r\n";
    let mut reader = BufReader::new(&input[..]);
    let result = parse_resp_line(&mut reader).expect("Failed to parse mixed array");
    match result {
        RespMessage::Array(items) => {
            assert_eq!(items.len(), 3);
            assert!(matches!(items[0], RespMessage::Integer(1)));
            assert!(matches!(items[1], RespMessage::Integer(2)));
            assert!(matches!(items[2], RespMessage::Integer(3)));
        }
        _ => panic!("Expected Array response"),
    }
}

#[test]
fn test_resp_nested_array() {
    // Test nested array: "*2\r\n*3\r\n:1\r\n:2\r\n:3\r\n*2\r\n+Foo\r\n-Bar\r\n"
    let input = b"*2\r\n*3\r\n:1\r\n:2\r\n:3\r\n*2\r\n+Foo\r\n-Bar\r\n";
    let mut reader = BufReader::new(&input[..]);
    let result = parse_resp_line(&mut reader).expect("Failed to parse nested array");
    match result {
        RespMessage::Array(items) => {
            assert_eq!(items.len(), 2);

            // First nested array
            match &items[0] {
                RespMessage::Array(nested) => {
                    assert_eq!(nested.len(), 3);
                    assert!(matches!(nested[0], RespMessage::Integer(1)));
                    assert!(matches!(nested[1], RespMessage::Integer(2)));
                    assert!(matches!(nested[2], RespMessage::Integer(3)));
                }
                _ => panic!("Expected nested Array"),
            }

            // Second nested array
            match &items[1] {
                RespMessage::Array(nested) => {
                    assert_eq!(nested.len(), 2);
                    assert!(matches!(nested[0], RespMessage::SimpleString(ref s) if s == "Foo"));
                    assert!(matches!(nested[1], RespMessage::SimpleError(ref s) if s == "Bar"));
                }
                _ => panic!("Expected nested Array"),
            }
        }
        _ => panic!("Expected Array response"),
    }
}

#[test]
fn test_resp_serialization() {
    // Test serialization of different RESP types

    // Simple string
    let message = RespMessage::SimpleString("OK".to_string());
    let serialized = message.as_bytes();
    assert_eq!(serialized, b"+OK\r\n");

    // Error
    let message = RespMessage::Error("ERR unknown command".to_string());
    let serialized = message.as_bytes();
    assert_eq!(serialized, b"-ERR unknown command\r\n");

    // Integer
    let message = RespMessage::Integer(1000);
    let serialized = message.as_bytes();
    assert_eq!(serialized, b":1000\r\n");

    // Bulk string
    let message = RespMessage::BulkString(Some(b"foobar".to_vec()));
    let serialized = message.as_bytes();
    assert_eq!(serialized, b"$6\r\nfoobar\r\n");

    // Empty bulk string
    let message = RespMessage::BulkString(Some(b"".to_vec()));
    let serialized = message.as_bytes();
    assert_eq!(serialized, b"$0\r\n\r\n");

    // Null
    let message = RespMessage::BulkString(None);
    let serialized = message.as_bytes();
    assert_eq!(serialized, b"-1\r\n");

    // Array
    let message = RespMessage::Array(vec![
        RespMessage::BulkString(Some(b"foo".to_vec())),
        RespMessage::BulkString(Some(b"bar".to_vec())),
    ]);
    let serialized = message.as_bytes();
    assert_eq!(serialized, b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n");
}

#[test]
fn test_resp_round_trip() {
    // Test round trip: serialize -> parse -> should be equal
    let original = RespMessage::Array(vec![
        RespMessage::BulkString(Some(b"SET".to_vec())),
        RespMessage::BulkString(Some(b"key".to_vec())),
        RespMessage::BulkString(Some(b"value".to_vec())),
    ]);

    let serialized = original.as_bytes();
    let mut reader = BufReader::new(&serialized[..]);
    let parsed = parse_resp_line(&mut reader).expect("Failed to parse serialized message");

    // Compare the messages
    match (original, parsed) {
        (RespMessage::Array(orig_items), RespMessage::Array(parsed_items)) => {
            assert_eq!(orig_items.len(), parsed_items.len());
            for (orig, parsed) in orig_items.iter().zip(parsed_items.iter()) {
                match (orig, parsed) {
                    (
                        RespMessage::BulkString(Some(orig_bytes)),
                        RespMessage::BulkString(Some(parsed_bytes)),
                    ) => {
                        assert_eq!(orig_bytes, parsed_bytes);
                    }
                    _ => panic!("Expected BulkString in both"),
                }
            }
        }
        _ => panic!("Expected Array in both"),
    }
}

#[test]
fn test_resp_invalid_input() {
    // Test invalid input
    let invalid_inputs: Vec<&[u8]> = vec![
        b"",                // Empty input
        b"invalid",         // No RESP format
        b"+OK",             // Missing \r\n
        b"$5\r\nhello",     // Missing \r\n
        b"*2\r\n$3\r\nfoo", // Incomplete array
    ];

    for input in invalid_inputs {
        let mut reader = BufReader::new(&input[..]);
        let result = parse_resp_line(&mut reader);
        assert!(result.is_err(), "Should fail to parse: {:?}", input);
    }
}

#[test]
fn test_resp_partial_input() {
    // Test partial input that should be handled gracefully
    let partial_input = b"+OK\r\n$6\r\nfoobar\r\n";
    let mut reader = BufReader::new(&partial_input[..]);
    let result = parse_resp_line(&mut reader).expect("Failed to parse partial input");

    // Should parse the first complete message
    assert!(matches!(result, RespMessage::SimpleString(ref s) if s == "OK"));
}

#[test]
fn test_resp_redis_commands() {
    // Test SET command: "*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n"
    let set_command = b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n";
    let mut reader = BufReader::new(&set_command[..]);
    let result = parse_resp_line(&mut reader).expect("Failed to parse SET command");

    match result {
        RespMessage::Array(items) => {
            assert_eq!(items.len(), 3);
            assert!(
                matches!(items[0], RespMessage::BulkString(Some(ref bytes)) if bytes == b"SET")
            );
            assert!(
                matches!(items[1], RespMessage::BulkString(Some(ref bytes)) if bytes == b"key")
            );
            assert!(
                matches!(items[2], RespMessage::BulkString(Some(ref bytes)) if bytes == b"value")
            );
        }
        _ => panic!("Expected Array response"),
    }

    // Test GET command: "*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n"
    let get_command = b"*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n";
    let mut reader = BufReader::new(&get_command[..]);
    let result = parse_resp_line(&mut reader).expect("Failed to parse GET command");

    match result {
        RespMessage::Array(items) => {
            assert_eq!(items.len(), 2);
            assert!(
                matches!(items[0], RespMessage::BulkString(Some(ref bytes)) if bytes == b"GET")
            );
            assert!(
                matches!(items[1], RespMessage::BulkString(Some(ref bytes)) if bytes == b"key")
            );
        }
        _ => panic!("Expected Array response"),
    }
}

#[test]
fn test_resp_large_strings() {
    // Test with large strings
    let large_string = "x".repeat(10000);
    let input = format!("${}\r\n{}\r\n", large_string.len(), large_string);
    let mut reader = BufReader::new(input.as_bytes());
    let result = parse_resp_line(&mut reader).expect("Failed to parse large string");

    assert!(
        matches!(result, RespMessage::BulkString(Some(ref bytes)) if bytes == large_string.as_bytes())
    );
}

#[test]
fn test_resp_unicode_strings() {
    // Test with Unicode strings
    let unicode_string = "Hello, 世界! 🌍";
    let input = format!("${}\r\n{}\r\n", unicode_string.len(), unicode_string);
    let mut reader = BufReader::new(input.as_bytes());
    let result = parse_resp_line(&mut reader).expect("Failed to parse Unicode string");

    assert!(
        matches!(result, RespMessage::BulkString(Some(ref bytes)) if bytes == unicode_string.as_bytes())
    );
}
