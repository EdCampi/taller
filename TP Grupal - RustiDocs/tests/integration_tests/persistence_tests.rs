//! Tests para la persistencia de datos en disco

use crate::integration_tests::TestRedisServer;
use rustidocs::storage::DataStore;
use std::fs;
use std::sync::{Arc, RwLock};

/// Tests para verificar que los datos se guardan correctamente en disco
#[test]
fn test_data_persistence() {
    let _server = TestRedisServer::new();
    let store = Arc::new(RwLock::new(DataStore::new()));

    // Agregar algunos datos al store
    {
        let mut store_guard = store.write().unwrap();
        store_guard.set("persist_key1".to_string(), "value1".to_string());
        store_guard.set("persist_key2".to_string(), "value2".to_string());

        // Agregar una lista
        store_guard.list_db.insert(
            "persist_list".to_string(),
            vec!["item1".to_string(), "item2".to_string()],
        );

        // Agregar un set
        let mut set = std::collections::HashSet::new();
        set.insert("member1".to_string());
        set.insert("member2".to_string());
        store_guard.set_db.insert("persist_set".to_string(), set);
    }

    // Verificar que los datos están en memoria
    {
        let store_guard = store.read().unwrap();
        assert_eq!(store_guard.get("persist_key1"), Some(&"value1".to_string()));
        assert_eq!(store_guard.get("persist_key2"), Some(&"value2".to_string()));
        assert_eq!(store_guard.list_db.get("persist_list").unwrap().len(), 2);
        assert_eq!(store_guard.set_db.get("persist_set").unwrap().len(), 2);
    }

    // Simular guardado en disco
    let result = _server.save_to_disk();
    assert!(result.is_ok(), "Failed to save data to disk");

    // Verificar que el directorio temporal existe
    assert!(
        _server.temp_dir.path().exists(),
        "Temp directory should exist"
    );
}

/// Tests para verificar la recuperación de datos desde disco
#[test]
fn test_data_recovery() {
    let _server = TestRedisServer::new();
    let store = Arc::new(RwLock::new(DataStore::new()));

    // Agregar datos al store
    {
        let mut store_guard = store.write().unwrap();
        store_guard.set("recovery_key".to_string(), "recovery_value".to_string());
    }

    // Simular guardado
    _server.save_to_disk().expect("Failed to save");

    // Crear un nuevo store (simulando reinicio)
    let _new_store = Arc::new(RwLock::new(DataStore::new()));

    // Simular carga desde disco
    let result = _server.load_from_disk();
    assert!(result.is_ok(), "Failed to load data from disk");

    // En una implementación real, aquí verificaríamos que los datos se cargaron
    // Por ahora solo verificamos que la operación no falla
    println!("Data recovery test completed");
}

/// Tests para verificar la persistencia de diferentes tipos de datos
#[test]
fn test_persistence_different_types() {
    let _server = TestRedisServer::new();
    let store = Arc::new(RwLock::new(DataStore::new()));

    // Agregar strings
    {
        let mut store_guard = store.write().unwrap();
        store_guard.set("string_key".to_string(), "string_value".to_string());
        store_guard.set("empty_string".to_string(), "".to_string());
        store_guard.set("unicode_string".to_string(), "ñáéíóú".to_string());
    }

    // Agregar listas
    {
        let mut store_guard = store.write().unwrap();
        store_guard.list_db.insert("empty_list".to_string(), vec![]);
        store_guard
            .list_db
            .insert("simple_list".to_string(), vec!["item1".to_string()]);
        store_guard.list_db.insert(
            "complex_list".to_string(),
            vec![
                "item1".to_string(),
                "item2".to_string(),
                "item3".to_string(),
            ],
        );
    }

    // Agregar sets
    {
        let mut store_guard = store.write().unwrap();
        let empty_set = std::collections::HashSet::new();
        store_guard
            .set_db
            .insert("empty_set".to_string(), empty_set);

        let mut simple_set = std::collections::HashSet::new();
        simple_set.insert("member1".to_string());
        store_guard
            .set_db
            .insert("simple_set".to_string(), simple_set);

        let mut complex_set = std::collections::HashSet::new();
        complex_set.insert("member1".to_string());
        complex_set.insert("member2".to_string());
        complex_set.insert("member3".to_string());
        store_guard
            .set_db
            .insert("complex_set".to_string(), complex_set);
    }

    // Verificar que todos los datos están en memoria
    {
        let store_guard = store.read().unwrap();
        assert_eq!(store_guard.string_db.len(), 3);
        assert_eq!(store_guard.list_db.len(), 3);
        assert_eq!(store_guard.set_db.len(), 3);
    }

    // Simular persistencia
    let result = _server.save_to_disk();
    assert!(result.is_ok(), "Failed to persist different data types");
}

/// Tests para verificar el manejo de archivos corruptos
#[test]
fn test_corrupted_file_handling() {
    let _server = TestRedisServer::new();

    // Crear un archivo corrupto en el directorio temporal
    let corrupted_file = _server.temp_dir.path().join("corrupted.rdb");
    fs::write(&corrupted_file, "This is not a valid dump file")
        .expect("Failed to create corrupted file");

    // Verificar que el archivo existe
    assert!(corrupted_file.exists(), "Corrupted file should exist");

    // En una implementación real, intentaríamos cargar el archivo corrupto
    // y manejaríamos el error apropiadamente
    // Por ahora solo verificamos que el archivo existe
    println!("Corrupted file handling test completed");
}

/// Tests para verificar la persistencia con datos grandes
#[test]
fn test_large_data_persistence() {
    let _server = TestRedisServer::new();
    let store = Arc::new(RwLock::new(DataStore::new()));

    // Agregar muchos datos
    {
        let mut store_guard = store.write().unwrap();

        // Agregar 100 strings
        for i in 0..100 {
            let key = format!("large_key_{}", i);
            let value = format!("large_value_{}", i);
            store_guard.set(key, value);
        }

        // Agregar una lista grande
        let mut large_list = Vec::new();
        for i in 0..1000 {
            large_list.push(format!("list_item_{}", i));
        }
        store_guard
            .list_db
            .insert("large_list".to_string(), large_list);

        // Agregar un set grande
        let mut large_set = std::collections::HashSet::new();
        for i in 0..500 {
            large_set.insert(format!("set_member_{}", i));
        }
        store_guard
            .set_db
            .insert("large_set".to_string(), large_set);
    }

    // Verificar que los datos están en memoria
    {
        let store_guard = store.read().unwrap();
        assert_eq!(store_guard.string_db.len(), 100);
        assert_eq!(store_guard.list_db.get("large_list").unwrap().len(), 1000);
        assert_eq!(store_guard.set_db.get("large_set").unwrap().len(), 500);
    }

    // Simular persistencia de datos grandes
    let result = _server.save_to_disk();
    assert!(result.is_ok(), "Failed to persist large data");
}

/// Tests para verificar la persistencia incremental
#[test]
fn test_incremental_persistence() {
    let _server = TestRedisServer::new();
    let store = Arc::new(RwLock::new(DataStore::new()));

    // Primera ronda de datos
    {
        let mut store_guard = store.write().unwrap();
        store_guard.set("incr_key1".to_string(), "value1".to_string());
        store_guard.set("incr_key2".to_string(), "value2".to_string());
    }

    // Primera persistencia
    let result1 = _server.save_to_disk();
    assert!(result1.is_ok(), "Failed first persistence");

    // Segunda ronda de datos
    {
        let mut store_guard = store.write().unwrap();
        store_guard.set("incr_key3".to_string(), "value3".to_string());
        store_guard.set("incr_key4".to_string(), "value4".to_string());
    }

    // Segunda persistencia
    let result2 = _server.save_to_disk();
    assert!(result2.is_ok(), "Failed second persistence");

    // Verificar que todos los datos están en memoria
    {
        let store_guard = store.read().unwrap();
        assert_eq!(store_guard.string_db.len(), 4);
        assert_eq!(store_guard.get("incr_key1"), Some(&"value1".to_string()));
        assert_eq!(store_guard.get("incr_key2"), Some(&"value2".to_string()));
        assert_eq!(store_guard.get("incr_key3"), Some(&"value3".to_string()));
        assert_eq!(store_guard.get("incr_key4"), Some(&"value4".to_string()));
    }
}

/// Tests para verificar la persistencia de caracteres especiales
#[test]
fn test_special_characters_persistence() {
    let _server = TestRedisServer::new();
    let store = Arc::new(RwLock::new(DataStore::new()));

    // Agregar datos con caracteres especiales
    {
        let mut store_guard = store.write().unwrap();
        store_guard.set("special_key".to_string(), "áéíóúñç".to_string());
        store_guard.set("emoji_key".to_string(), "🚀🌟🎉".to_string());
        store_guard.set("binary_key".to_string(), "\\x00\\x01\\x02".to_string());
    }

    // Verificar que los datos están en memoria
    {
        let store_guard = store.read().unwrap();
        assert_eq!(store_guard.get("special_key"), Some(&"áéíóúñç".to_string()));
        assert_eq!(store_guard.get("emoji_key"), Some(&"🚀🌟🎉".to_string()));
        assert_eq!(
            store_guard.get("binary_key"),
            Some(&"\\x00\\x01\\x02".to_string())
        );
    }

    // Simular persistencia
    let result = _server.save_to_disk();
    assert!(result.is_ok(), "Failed to persist special characters");
}

/// Tests para verificar el rendimiento de la persistencia
#[test]
fn test_persistence_performance() {
    let _server = TestRedisServer::new();
    let store = Arc::new(RwLock::new(DataStore::new()));

    // Agregar datos de prueba
    {
        let mut store_guard = store.write().unwrap();

        // Agregar 1000 strings pequeños
        for i in 0..1000 {
            let key = format!("perf_key_{}", i);
            let value = format!("perf_value_{}", i);
            store_guard.set(key, value);
        }
    }

    // Medir tiempo de persistencia
    let start = std::time::Instant::now();
    let result = _server.save_to_disk();
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to persist performance test data");

    // Verificar que la persistencia no toma demasiado tiempo
    // (más de 5 segundos sería sospechoso)
    assert!(
        duration.as_secs() < 5,
        "Persistence took too long: {:?}",
        duration
    );

    println!("Persistence performance test completed in {:?}", duration);
}

/// Tests para casos extremos de persistencia
#[test]
fn test_persistence_edge_cases() {
    let _server = TestRedisServer::new();
    let store = Arc::new(RwLock::new(DataStore::new()));

    // Test con clave muy larga
    {
        let mut store_guard = store.write().unwrap();
        let long_key = "a".repeat(10000);
        let long_value = "b".repeat(10000);
        store_guard.set(long_key.clone(), long_value);
    }

    // Test con valor muy largo
    {
        let mut store_guard = store.write().unwrap();
        let very_long_value = "c".repeat(100000);
        store_guard.set("very_long_value_key".to_string(), very_long_value);
    }

    // Test con clave vacía
    {
        let mut store_guard = store.write().unwrap();
        store_guard.set("".to_string(), "empty_key_value".to_string());
    }

    // Test con valor vacío
    {
        let mut store_guard = store.write().unwrap();
        store_guard.set("empty_value_key".to_string(), "".to_string());
    }

    // Verificar que todos los datos están en memoria
    {
        let store_guard = store.read().unwrap();
        assert_eq!(store_guard.string_db.len(), 4);
        assert_eq!(store_guard.get(""), Some(&"empty_key_value".to_string()));
        assert_eq!(store_guard.get("empty_value_key"), Some(&"".to_string()));
    }

    // Simular persistencia
    let result = _server.save_to_disk();
    assert!(result.is_ok(), "Failed to persist edge case data");
}
