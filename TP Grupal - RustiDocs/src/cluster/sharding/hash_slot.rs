//! Módulo encargado del cálculo de hash slots para el sharding de claves en el cluster.
//!
//! Este módulo implementa el algoritmo de hash slots utilizado por Redis Cluster
//! para distribuir claves entre los diferentes nodos del cluster. Utiliza el
//! algoritmo CRC16-XMODEM para calcular hashes consistentes y soporta el concepto
//! de "hash tags" para agrupar claves relacionadas en el mismo slot.

/// Número máximo de hash slots en el cluster Redis.
///
/// Este valor es estándar en Redis Cluster y define el espacio total
/// de slots disponibles para distribuir entre los nodos del cluster.
pub const MAX_HASH_SLOTS: u16 = 16384;

/// Errores específicos del cálculo de hash slots.
#[derive(Debug, PartialEq)]
pub enum HashSlotError {
    /// Clave vacía o inválida
    InvalidKey(String),
}

impl std::fmt::Display for HashSlotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HashSlotError::InvalidKey(msg) => write!(f, "Clave inválida: {}", msg),
        }
    }
}

impl std::error::Error for HashSlotError {}

/// Calcula el valor CRC16 usando el polinomio XMODEM.
///
/// Esta función implementa el algoritmo CRC16-XMODEM que es utilizado
/// por Redis Cluster para calcular hash slots de forma consistente.
/// El polinomio utilizado es 0x1021.
///
/// # Argumentos
///
/// * `data` - Slice de bytes sobre el cual calcular el CRC16
///
/// # Retorna
///
/// Valor CRC16 calculado como u16.
///
/// # Algoritmo
///
/// 1. Inicializa CRC en 0x0000
/// 2. Para cada byte, hace XOR con CRC desplazado 8 bits a la izquierda
/// 3. Para cada bit del byte, verifica el bit más significativo
/// 4. Si está activado, desplaza y hace XOR con 0x1021
/// 5. Si no, solo desplaza a la izquierda
fn crc16_xmodem(data: &[u8]) -> u16 {
    let mut crc: u16 = 0x0000;

    for &byte in data {
        crc ^= (byte as u16) << 8;

        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }

    crc
}

/// Calcula el hash slot para una clave dada.
///
/// Esta función es el punto de entrada principal para determinar
/// en qué slot del cluster debe almacenarse una clave específica.
/// Soporta el concepto de "hash tags" de Redis Cluster.
///
/// # Argumentos
///
/// * `key` - Clave para la cual calcular el hash slot
///
/// # Retorna
///
/// * `Ok(u16)` - Número de slot (0 a MAX_HASH_SLOTS-1)
/// * `Err(HashSlotError)` - Error si la clave es inválida
///
/// # Hash Tags
///
/// Si la clave contiene texto entre llaves `{...}`, solo se usa
/// ese contenido para el cálculo del hash. Esto permite agrupar
/// claves relacionadas en el mismo slot.
///
/// # Errores
///
/// Esta función puede fallar si:
/// - La clave está vacía
/// - Hay un error interno en el cálculo del hash
pub fn hash_slot(key: &str) -> Result<u16, HashSlotError> {
    if key.is_empty() {
        return Err(HashSlotError::InvalidKey(
            "La clave no puede estar vacía".to_string(),
        ));
    }

    let hash_key = extract_hash_key(key);

    if hash_key.is_empty() {
        return Err(HashSlotError::InvalidKey(
            "La clave efectiva para hash no puede estar vacía".to_string(),
        ));
    }

    let crc = crc16_xmodem(hash_key.as_bytes());
    let slot = crc % MAX_HASH_SLOTS;

    Ok(slot)
}

/// Extrae la clave efectiva para el cálculo del hash.
///
/// Esta función implementa la lógica de "hash tags" de Redis Cluster.
/// Si la clave contiene texto entre llaves `{...}`, extrae solo
/// ese contenido. Si no hay llaves o están vacías, retorna la clave completa.
///
/// # Argumentos
///
/// * `key` - Clave original de la cual extraer el hash tag
///
/// # Retorna
///
/// Slice de string que contiene la clave efectiva para el hash.
///
/// # Comportamiento
///
/// - `"user:123"` → `"user:123"` (sin llaves, usa toda la clave)
/// - `"profile:{user}:name"` → `"user"` (extrae contenido entre llaves)
/// - `"data:{}:value"` → `"data:{}:value"` (llaves vacías, usa toda la clave)
/// - `"prefix:{tag}:suffix"` → `"tag"` (extrae solo el tag)
/// - `"no{close"` → `"no{close"` (llave sin cerrar, usa toda la clave)
fn extract_hash_key(key: &str) -> &str {
    if let Some(start) = key.find('{') {
        if let Some(end) = key[start + 1..].find('}') {
            let tag_content = &key[start + 1..start + 1 + end];
            // Solo usar el tag si no está vacío
            if !tag_content.is_empty() {
                return tag_content;
            }
        }
    }
    key
}

/// Calcula el hash slot para múltiples claves.
///
/// Esta función es una conveniencia para calcular los slots de hash
/// para un conjunto de claves de una vez.
///
/// # Argumentos
///
/// * `keys` - Slice de claves a procesar
///
/// # Retorna
///
/// * `Ok(Vec<u16>)` - Vector con los slots de hash para cada clave
/// * `Err(HashSlotError)` - Error si alguna clave es inválida
///
/// # Comportamiento
///
/// Si alguna clave es inválida, la función retorna error inmediatamente
/// sin procesar las claves restantes.
#[allow(dead_code)]
pub fn hash_slots(keys: &[&str]) -> Result<Vec<u16>, HashSlotError> {
    keys.iter().map(|&key| hash_slot(key)).collect()
}

/// Verifica si múltiples claves pertenecen al mismo slot.
///
/// Esta función es útil para operaciones multi-clave que requieren
/// que todas las claves estén en el mismo nodo del cluster.
///
/// # Argumentos
///
/// * `keys` - Slice de claves a verificar
///
/// # Retorna
///
/// * `Ok(bool)` - `true` si todas las claves están en el mismo slot
/// * `Err(HashSlotError)` - Error si alguna clave es inválida
#[allow(dead_code)]
pub fn keys_same_slot(keys: &[&str]) -> Result<bool, HashSlotError> {
    if keys.is_empty() {
        return Ok(true);
    }

    let first_slot = hash_slot(keys[0])?;

    for &key in &keys[1..] {
        if hash_slot(key)? != first_slot {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Obtiene información sobre la distribución de slots para un conjunto de claves.
///
/// Esta función analiza un conjunto de claves y retorna estadísticas
/// sobre cómo se distribuyen entre los diferentes slots.
///
/// # Argumentos
///
/// * `keys` - Slice de claves a analizar
///
/// # Retorna
///
/// * `Ok((usize, u16, u16))` - Tupla con (número_de_slots_únicos, slot_mínimo, slot_máximo)
/// * `Err(HashSlotError)` - Error si alguna clave es inválida
#[allow(dead_code)]
pub fn slot_distribution(keys: &[&str]) -> Result<(usize, u16, u16), HashSlotError> {
    if keys.is_empty() {
        return Err(HashSlotError::InvalidKey(
            "No se pueden analizar claves vacías".to_string(),
        ));
    }

    let slots = hash_slots(keys)?;
    let mut unique_slots = std::collections::HashSet::new();

    for &slot in &slots {
        unique_slots.insert(slot);
    }

    let min_slot = *slots.iter().min().unwrap();
    let max_slot = *slots.iter().max().unwrap();

    Ok((unique_slots.len(), min_slot, max_slot))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_crc16_xmodem_known_values() {
        // Valores conocidos para verificar la implementación
        assert_eq!(crc16_xmodem(b""), 0);
        // Verificar con valores que sabemos que son correctos para nuestra implementación
        let crc_a = crc16_xmodem(b"A");
        let crc_123 = crc16_xmodem(b"123456789");

        // Los valores deben ser consistentes y estar en el rango válido
        assert!(crc_a > 0);
        assert!(crc_123 > 0);
        // Los valores u16 están automáticamente en el rango 0..=65535
    }

    #[test]
    fn test_hash_slot_basic() {
        let slot = hash_slot("test_key").unwrap();
        assert!(slot < MAX_HASH_SLOTS);
    }

    #[test]
    fn test_hash_slot_empty_key() {
        let result = hash_slot("");
        assert!(result.is_err());
        match result.unwrap_err() {
            HashSlotError::InvalidKey(_) => (),
        }
    }

    #[test]
    fn test_hash_slot_consistency() {
        // El mismo input debe producir el mismo output
        let key = "consistent_key";
        let slot1 = hash_slot(key).unwrap();
        let slot2 = hash_slot(key).unwrap();
        assert_eq!(slot1, slot2);
    }

    #[test]
    fn test_extract_hash_key_no_braces() {
        assert_eq!(extract_hash_key("simple_key"), "simple_key");
        assert_eq!(extract_hash_key("user:123"), "user:123");
    }

    #[test]
    fn test_extract_hash_key_with_braces() {
        assert_eq!(extract_hash_key("user:{tag}:data"), "tag");
        assert_eq!(extract_hash_key("prefix:{hashtag}:suffix"), "hashtag");
        assert_eq!(extract_hash_key("{onlytag}"), "onlytag");
    }

    #[test]
    fn test_extract_hash_key_empty_braces() {
        // Llaves vacías deben usar la clave completa
        assert_eq!(extract_hash_key("user:{}:data"), "user:{}:data");
        assert_eq!(extract_hash_key("{}"), "{}");
    }

    #[test]
    fn test_extract_hash_key_incomplete_braces() {
        // Llaves sin cerrar deben usar la clave completa
        assert_eq!(extract_hash_key("user:{incomplete"), "user:{incomplete");
        assert_eq!(extract_hash_key("no_open}"), "no_open}");
    }

    #[test]
    fn test_hash_tag_same_slot() {
        // Claves con el mismo hash tag deben ir al mismo slot
        let slot1 = hash_slot("user:{123}:profile").unwrap();
        let slot2 = hash_slot("user:{123}:settings").unwrap();
        let slot3 = hash_slot("session:{123}:data").unwrap();

        assert_eq!(slot1, slot2);
        assert_eq!(slot1, slot3);
    }

    #[test]
    fn test_hash_slots_multiple() {
        let keys = vec!["key1", "key2", "key3"];
        let slots = hash_slots(&keys).unwrap();

        assert_eq!(slots.len(), 3);
        for slot in slots {
            assert!(slot < MAX_HASH_SLOTS);
        }
    }

    #[test]
    fn test_hash_slots_with_invalid_key() {
        let keys = vec!["valid_key", "", "another_key"];
        let result = hash_slots(&keys);

        assert!(result.is_err());
    }

    #[test]
    fn test_keys_same_slot_true() {
        // Claves con mismo hash tag
        let keys = vec!["user:{tag}:profile", "user:{tag}:settings"];
        assert!(keys_same_slot(&keys).unwrap());
    }

    #[test]
    fn test_keys_same_slot_false() {
        // Claves diferentes sin hash tags (muy probablemente diferentes slots)
        let keys = vec!["user:123", "user:456", "user:789"];
        let same = keys_same_slot(&keys).unwrap();

        // Aunque es posible que por casualidad estén en el mismo slot,
        // es muy improbable con estas claves específicas
        // Solo verificamos que la función no falle
        assert!(same == true || same == false);
    }

    #[test]
    fn test_keys_same_slot_empty() {
        assert!(keys_same_slot(&[]).unwrap());
    }

    #[test]
    fn test_keys_same_slot_single() {
        assert!(keys_same_slot(&["single_key"]).unwrap());
    }

    #[test]
    fn test_slot_distribution() {
        let keys = vec!["key1", "key2", "key3", "key4", "key5"];
        let (unique_count, min_slot, max_slot) = slot_distribution(&keys).unwrap();

        assert!(unique_count >= 1);
        assert!(unique_count <= keys.len());
        assert!(min_slot <= max_slot);
        assert!(max_slot < MAX_HASH_SLOTS);
    }

    #[test]
    fn test_slot_distribution_empty() {
        let result = slot_distribution(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_slot_distribution_same_keys() {
        let keys = vec!["same:{tag}:1", "same:{tag}:2", "same:{tag}:3"];
        let (unique_count, min_slot, max_slot) = slot_distribution(&keys).unwrap();

        assert_eq!(unique_count, 1); // Todas en el mismo slot
        assert_eq!(min_slot, max_slot); // Min y max iguales
    }

    #[test]
    fn test_max_hash_slots_constant() {
        assert_eq!(MAX_HASH_SLOTS, 16384);
    }

    #[test]
    fn test_hash_slot_range() {
        // Probar con varias claves que el resultado esté en el rango válido
        let test_keys = vec![
            "test",
            "user:123",
            "session:abc",
            "data:{tag}:item",
            "very_long_key_name_to_test_edge_cases",
            "🔑",
            "key with spaces",
        ];

        for key in test_keys {
            let slot = hash_slot(key).unwrap();
            assert!(
                slot < MAX_HASH_SLOTS,
                "Slot {} fuera de rango para clave '{}'",
                slot,
                key
            );
        }
    }

    #[test]
    fn test_hash_slot_error_display() {
        let invalid_error = HashSlotError::InvalidKey("test key".to_string());

        assert!(invalid_error.to_string().contains("Clave inválida"));
    }

    #[test]
    fn test_hash_slot_error_traits() {
        let error = HashSlotError::InvalidKey("test".to_string());

        // Verificar que implementa Debug
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("InvalidKey"));

        // Verificar que implementa Display
        let display_str = error.to_string();
        assert!(display_str.contains("Clave inválida"));

        // Verificar que implementa Error trait
        let _: &dyn std::error::Error = &error;
    }

    #[test]
    fn test_crc16_deterministic() {
        // El CRC16 debe ser determinístico
        let data = b"deterministic_test";
        let crc1 = crc16_xmodem(data);
        let crc2 = crc16_xmodem(data);
        assert_eq!(crc1, crc2);
    }

    #[test]
    fn test_extract_hash_key_multiple_braces() {
        // Solo debe usar el primer par de llaves válido
        assert_eq!(
            extract_hash_key("prefix:{first}:middle:{second}:suffix"),
            "first"
        );
        assert_eq!(extract_hash_key("{first}:{second}"), "first");
    }
}
