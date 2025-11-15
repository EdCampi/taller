//! Tests para los comandos de Redis (strings, lists, sets)

use crate::integration_tests::TestRedisServer;
use rustidocs::{
    command::{types::Command, types::ResponseType},
    storage::DataStore,
};
use std::sync::{Arc, RwLock};

/// Tests para comandos de strings
#[test]
fn test_string_commands() {
    let _server = TestRedisServer::new();
    let store = Arc::new(RwLock::new(DataStore::new()));

    // Crear un comando SET simple
    let set_cmd = Command::Set("string_key".to_string(), "hello".to_string());

    // Ejecutar el comando directamente en el store
    {
        let mut store_guard = store.write().unwrap();
        let result = set_cmd.execute_write(&mut *store_guard);
        assert!(result.is_ok());
    }

    // Verificar que se guardó correctamente
    {
        let store_guard = store.read().unwrap();
        let value = store_guard.get("string_key");
        assert!(value.is_some());
        assert_eq!(value.unwrap(), "hello");
    }

    // Crear un comando GET
    let get_cmd = Command::Get("string_key".to_string());

    // Ejecutar el comando de lectura
    {
        let store_guard = store.read().unwrap();
        let result = get_cmd.execute_read(&store_guard, None, None, None, None, None);
        assert!(result.is_ok());
        match result.unwrap() {
            ResponseType::Str(value) => assert_eq!(value, "hello"),
            _ => panic!("Expected Str response"),
        }
    }

    // Test GET de clave inexistente
    let get_cmd = Command::Get("nonexistent".to_string());
    {
        let store_guard = store.read().unwrap();
        let result = get_cmd.execute_read(&store_guard, None, None, None, None, None);
        assert!(result.is_ok());
        match result.unwrap() {
            ResponseType::Null(_) => {} // Esperado para clave inexistente
            _ => panic!("Expected Null response"),
        }
    }
}

/// Tests para comandos de lists
#[test]
fn test_list_commands() {
    let _server = TestRedisServer::new();
    let store = Arc::new(RwLock::new(DataStore::new()));

    // Test LPUSH
    let lpush_cmd = Command::Lpush(
        "list_key".to_string(),
        vec![
            "item1".to_string(),
            "item2".to_string(),
            "item3".to_string(),
        ],
    );

    {
        let mut store_guard = store.write().unwrap();
        let result = lpush_cmd.execute_write(&mut *store_guard);
        assert!(result.is_ok());
    }

    // Test LRANGE
    let lrange_cmd = Command::Lrange("list_key".to_string(), 0, -1);
    {
        let store_guard = store.read().unwrap();
        let result = lrange_cmd.execute_read(&store_guard, None, None, None, None, None);
        assert!(result.is_ok());
        match result.unwrap() {
            ResponseType::List(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], "item1"); // LPUSH inserta al inicio en orden de argumentos
                assert_eq!(items[1], "item2");
                assert_eq!(items[2], "item3");
            }
            _ => panic!("Expected List response"),
        }
    }

    // Test LLEN
    let llen_cmd = Command::Llen("list_key".to_string());
    {
        let store_guard = store.read().unwrap();
        let result = llen_cmd.execute_read(&store_guard, None, None, None, None, None);
        assert!(result.is_ok());
        match result.unwrap() {
            ResponseType::Int(len) => assert_eq!(len, 3),
            _ => panic!("Expected Int response"),
        }
    }
}

/// Tests para comandos de sets
#[test]
fn test_set_commands() {
    let _server = TestRedisServer::new();
    let store = Arc::new(RwLock::new(DataStore::new()));

    // Test SADD
    let sadd_cmd = Command::Sadd(
        "set_key".to_string(),
        vec![
            "member1".to_string(),
            "member2".to_string(),
            "member3".to_string(),
        ],
    );

    {
        let mut store_guard = store.write().unwrap();
        let result = sadd_cmd.execute_write(&mut *store_guard);
        assert!(result.is_ok());
    }

    // Test SMEMBERS
    let smembers_cmd = Command::Smembers("set_key".to_string());
    {
        let store_guard = store.read().unwrap();
        let result = smembers_cmd.execute_read(&store_guard, None, None, None, None, None);
        assert!(result.is_ok());
        match result.unwrap() {
            ResponseType::Set(members) => {
                assert_eq!(members.len(), 3);
                assert!(members.contains("member1"));
                assert!(members.contains("member2"));
                assert!(members.contains("member3"));
            }
            _ => panic!("Expected Set response"),
        }
    }

    // Test SISMEMBER
    let sismember_cmd = Command::Sismember("set_key".to_string(), "member1".to_string());
    {
        let store_guard = store.read().unwrap();
        let result = sismember_cmd.execute_read(&store_guard, None, None, None, None, None);
        assert!(result.is_ok());
        match result.unwrap() {
            ResponseType::Int(exists) => assert_eq!(exists, 1),
            _ => panic!("Expected Int response"),
        }
    }

    let sismember_cmd = Command::Sismember("set_key".to_string(), "nonexistent".to_string());
    {
        let store_guard = store.read().unwrap();
        let result = sismember_cmd.execute_read(&store_guard, None, None, None, None, None);
        assert!(result.is_ok());
        match result.unwrap() {
            ResponseType::Int(exists) => assert_eq!(exists, 0),
            _ => panic!("Expected Int response"),
        }
    }
}

/// Tests para comandos de eliminación
#[test]
fn test_delete_commands() {
    let _server = TestRedisServer::new();
    let store = Arc::new(RwLock::new(DataStore::new()));

    // Crear algunos datos
    {
        let mut store_guard = store.write().unwrap();
        let set_cmd = Command::Set("delete_key".to_string(), "value".to_string());
        set_cmd.execute_write(&mut *store_guard).unwrap();

        let lpush_cmd = Command::Lpush("delete_list".to_string(), vec!["item1".to_string()]);
        lpush_cmd.execute_write(&mut *store_guard).unwrap();

        let sadd_cmd = Command::Sadd("delete_set".to_string(), vec!["member1".to_string()]);
        sadd_cmd.execute_write(&mut *store_guard).unwrap();
    }

    // Test DELETE individual
    let del_cmd = Command::Del(vec!["delete_key".to_string()]);
    {
        let mut store_guard = store.write().unwrap();
        let result = del_cmd.execute_write(&mut *store_guard);
        assert!(result.is_ok());
        match result.unwrap() {
            ResponseType::Int(deleted) => assert_eq!(deleted, 1),
            _ => panic!("Expected Int response"),
        }
    }

    // Verificar que se eliminó
    {
        let store_guard = store.read().unwrap();
        let value = store_guard.get("delete_key");
        assert!(value.is_none());
    }

    // Test DELETE múltiple
    let del_cmd = Command::Del(vec!["delete_list".to_string(), "delete_set".to_string()]);
    {
        let mut store_guard = store.write().unwrap();
        let result = del_cmd.execute_write(&mut *store_guard);
        assert!(result.is_ok());
        match result.unwrap() {
            ResponseType::Int(deleted) => assert_eq!(deleted, 2),
            _ => panic!("Expected Int response"),
        }
    }
}

/// Tests para operaciones de strings
#[test]
fn test_string_operations() {
    let _server = TestRedisServer::new();
    let store = Arc::new(RwLock::new(DataStore::new()));

    // Test múltiples operaciones de strings
    {
        let mut store_guard = store.write().unwrap();

        // SET múltiples valores
        let set_cmd1 = Command::Set("key1".to_string(), "value1".to_string());
        let set_cmd2 = Command::Set("key2".to_string(), "value2".to_string());
        let set_cmd3 = Command::Set("key3".to_string(), "value3".to_string());

        assert!(set_cmd1.execute_write(&mut *store_guard).is_ok());
        assert!(set_cmd2.execute_write(&mut *store_guard).is_ok());
        assert!(set_cmd3.execute_write(&mut *store_guard).is_ok());
    }

    // Verificar todos los valores
    {
        let store_guard = store.read().unwrap();
        assert_eq!(store_guard.get("key1"), Some(&"value1".to_string()));
        assert_eq!(store_guard.get("key2"), Some(&"value2".to_string()));
        assert_eq!(store_guard.get("key3"), Some(&"value3".to_string()));
    }

    // Test GET de todos los valores
    {
        let store_guard = store.read().unwrap();
        let get_cmd1 = Command::Get("key1".to_string());
        let get_cmd2 = Command::Get("key2".to_string());
        let get_cmd3 = Command::Get("key3".to_string());

        match get_cmd1
            .execute_read(&store_guard, None, None, None, None, None)
            .unwrap()
        {
            ResponseType::Str(value) => assert_eq!(value, "value1"),
            _ => panic!("Expected Str response"),
        }

        match get_cmd2
            .execute_read(&store_guard, None, None, None, None, None)
            .unwrap()
        {
            ResponseType::Str(value) => assert_eq!(value, "value2"),
            _ => panic!("Expected Str response"),
        }

        match get_cmd3
            .execute_read(&store_guard, None, None, None, None, None)
            .unwrap()
        {
            ResponseType::Str(value) => assert_eq!(value, "value3"),
            _ => panic!("Expected Str response"),
        }
    }
}

/// Tests para operaciones de lists
#[test]
fn test_list_operations() {
    let _server = TestRedisServer::new();
    let store = Arc::new(RwLock::new(DataStore::new()));

    // Test múltiples operaciones de lists
    {
        let mut store_guard = store.write().unwrap();

        // LPUSH múltiples listas
        let lpush_cmd1 =
            Command::Lpush("list1".to_string(), vec!["a".to_string(), "b".to_string()]);
        let lpush_cmd2 = Command::Lpush(
            "list2".to_string(),
            vec!["x".to_string(), "y".to_string(), "z".to_string()],
        );

        assert!(lpush_cmd1.execute_write(&mut *store_guard).is_ok());
        assert!(lpush_cmd2.execute_write(&mut *store_guard).is_ok());
    }

    // Verificar listas
    {
        let store_guard = store.read().unwrap();
        let lrange_cmd1 = Command::Lrange("list1".to_string(), 0, -1);
        let lrange_cmd2 = Command::Lrange("list2".to_string(), 0, -1);

        match lrange_cmd1
            .execute_read(&store_guard, None, None, None, None, None)
            .unwrap()
        {
            ResponseType::List(items) => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0], "a");
                assert_eq!(items[1], "b");
            }
            _ => panic!("Expected List response"),
        }

        match lrange_cmd2
            .execute_read(&store_guard, None, None, None, None, None)
            .unwrap()
        {
            ResponseType::List(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], "x");
                assert_eq!(items[1], "y");
                assert_eq!(items[2], "z");
            }
            _ => panic!("Expected List response"),
        }
    }
}

/// Tests para operaciones de sets
#[test]
fn test_set_operations() {
    let _server = TestRedisServer::new();
    let store = Arc::new(RwLock::new(DataStore::new()));

    // Test múltiples operaciones de sets
    {
        let mut store_guard = store.write().unwrap();

        // SADD múltiples sets
        let sadd_cmd1 = Command::Sadd(
            "set1".to_string(),
            vec!["alpha".to_string(), "beta".to_string()],
        );
        let sadd_cmd2 = Command::Sadd(
            "set2".to_string(),
            vec![
                "gamma".to_string(),
                "delta".to_string(),
                "epsilon".to_string(),
            ],
        );

        assert!(sadd_cmd1.execute_write(&mut *store_guard).is_ok());
        assert!(sadd_cmd2.execute_write(&mut *store_guard).is_ok());
    }

    // Verificar sets
    {
        let store_guard = store.read().unwrap();
        let smembers_cmd1 = Command::Smembers("set1".to_string());
        let smembers_cmd2 = Command::Smembers("set2".to_string());

        match smembers_cmd1
            .execute_read(&store_guard, None, None, None, None, None)
            .unwrap()
        {
            ResponseType::Set(members) => {
                assert_eq!(members.len(), 2);
                assert!(members.contains("alpha"));
                assert!(members.contains("beta"));
            }
            _ => panic!("Expected Set response"),
        }

        match smembers_cmd2
            .execute_read(&store_guard, None, None, None, None, None)
            .unwrap()
        {
            ResponseType::Set(members) => {
                assert_eq!(members.len(), 3);
                assert!(members.contains("gamma"));
                assert!(members.contains("delta"));
                assert!(members.contains("epsilon"));
            }
            _ => panic!("Expected Set response"),
        }
    }
}

/// Tests para argumentos inválidos
#[test]
fn test_invalid_arguments() {
    let _server = TestRedisServer::new();
    let store = Arc::new(RwLock::new(DataStore::new()));

    // Test LRANGE con índices inválidos
    {
        let store_guard = store.read().unwrap();
        let lrange_cmd = Command::Lrange("nonexistent".to_string(), 0, 10);
        let result = lrange_cmd.execute_read(&store_guard, None, None, None, None, None);
        assert!(result.is_ok());
        match result.unwrap() {
            ResponseType::List(items) => assert_eq!(items.len(), 0),
            _ => panic!("Expected empty List response"),
        }
    }
}

/// Tests para errores de tipo incorrecto
#[test]
fn test_wrong_type_errors() {
    let _server = TestRedisServer::new();
    let store = Arc::new(RwLock::new(DataStore::new()));

    // Crear un string
    {
        let mut store_guard = store.write().unwrap();
        let set_cmd = Command::Set("mixed_key".to_string(), "string_value".to_string());
        set_cmd.execute_write(&mut *store_guard).unwrap();
    }

    // Intentar usar comandos de list en un string (debería fallar)
    {
        let store_guard = store.read().unwrap();
        let lrange_cmd = Command::Lrange("mixed_key".to_string(), 0, -1);
        let result = lrange_cmd.execute_read(&store_guard, None, None, None, None, None);
        assert!(result.is_err()); // Debería fallar por tipo incorrecto
    }

    // Intentar usar comandos de set en un string (debería fallar)
    {
        let store_guard = store.read().unwrap();
        let smembers_cmd = Command::Smembers("mixed_key".to_string());
        let result = smembers_cmd.execute_read(&store_guard, None, None, None, None, None);
        assert!(result.is_err()); // Debería fallar por tipo incorrecto
    }
}
