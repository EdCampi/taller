# Tests de Integración para Redis

Este directorio contiene tests de integración completos para verificar la funcionalidad de la implementación de Redis.

## Estructura de Tests

### `mod.rs`
- Módulo principal que define las estructuras comunes para los tests
- `TestRedisServer`: Simula un servidor Redis para testing
- `TestConfig`: Configuración común para todos los tests

### `common.rs`
- Utilidades comunes para todos los tests
- `TestClient`: Cliente de test para conectarse al servidor
- `TestServer`: Servidor de test simple
- Utilidades para concurrencia, rendimiento, persistencia y red

### `protocol_tests.rs`
Tests para el protocolo cliente/servidor RESP de Redis:
- Parsing y serialización de mensajes RESP
- Manejo de diferentes tipos de datos (strings, integers, arrays, etc.)
- Casos edge y entrada inválida
- Comandos Redis específicos

### `command_tests.rs`
Tests para los comandos de Redis:
- **Strings**: SET, GET, APPEND, STRLEN, GETRANGE, SETRANGE
- **Lists**: LPUSH, LRANGE, LLEN, LPOP, LINDEX, LSET, LINSERT
- **Sets**: SADD, SMEMBERS, SISMEMBER, SCARD, SINTER, SUNION, SDIFF
- **Operaciones generales**: DEL, EXISTS, TYPE
- Manejo de errores y casos edge

### `persistence_tests.rs`
Tests para el almacenamiento en disco:
- Guardado y recuperación de datos
- Persistencia de diferentes tipos de datos
- Manejo de archivos corruptos
- Tests de rendimiento
- Casos edge (caracteres especiales, claves vacías)

### `pubsub_tests.rs`
Tests para la funcionalidad Pub/Sub:
- Suscripciones y publicaciones básicas
- Múltiples suscripciones
- Concurrencia en Pub/Sub
- Rendimiento
- Integración con otros comandos

## Cómo Ejecutar los Tests

### Ejecutar todos los tests de integración:
```bash
cargo test --test integration_tests
```

### Ejecutar tests específicos:
```bash
# Solo tests de protocolo
cargo test --test integration_tests protocol_tests

# Solo tests de comandos
cargo test --test integration_tests command_tests

# Solo tests de persistencia
cargo test --test integration_tests persistence_tests

# Solo tests de Pub/Sub
cargo test --test integration_tests pubsub_tests
```

### Ejecutar un test específico:
```bash
cargo test test_basic_pubsub
```

### Ejecutar tests con output detallado:
```bash
cargo test --test integration_tests -- --nocapture
```

## Funcionalidades Verificadas

### 1. Protocolo Cliente/Servidor
- ✅ Implementación del protocolo RESP de Redis
- ✅ Parsing y serialización de mensajes
- ✅ Manejo de diferentes tipos de datos
- ✅ Casos edge y entrada inválida

### 2. Comandos de Redis
- ✅ **Strings**: SET, GET, APPEND, STRLEN, GETRANGE, SETRANGE
- ✅ **Lists**: LPUSH, LRANGE, LLEN, LPOP, LINDEX, LSET, LINSERT
- ✅ **Sets**: SADD, SMEMBERS, SISMEMBER, SCARD, SINTER, SUNION, SDIFF
- ✅ **Operaciones generales**: DEL, EXISTS, TYPE
- ✅ Manejo de errores y tipos incorrectos

### 3. Almacenamiento en Disco
- ✅ Guardado de datos en archivos de dump
- ✅ Recuperación de datos desde disco
- ✅ Persistencia de todos los tipos de datos
- ✅ Manejo de archivos corruptos
- ✅ Tests de rendimiento

### 4. Pub/Sub (Publish/Subscribe)
- ✅ Suscripciones y publicaciones básicas
- ✅ Múltiples suscripciones
- ✅ Comunicación entre múltiples clientes
- ✅ Concurrencia en operaciones Pub/Sub
- ✅ Integración con otros comandos

## Configuración

Los tests utilizan archivos temporales para evitar interferir con datos reales. Cada test crea su propio directorio temporal que se limpia automáticamente al finalizar.

### Configuración por defecto:
- Puerto del servidor: 6379
- Puertos del cluster: [6380, 6381, 6382]
- Archivo de dump: `dump.rdb`
- Archivo AOF: `appendonly.aof`

## Dependencias

Los tests requieren las siguientes dependencias (ya incluidas en `Cargo.toml`):
- `tempfile`: Para crear directorios temporales
- `serde`: Para serialización de mensajes Pub/Sub
- `chrono`: Para timestamps en mensajes Pub/Sub

## Notas de Implementación

### Estructura de Comandos
Los tests asumen que los comandos están implementados como enums en `rustidocs::command::Command`:

```rust
pub enum Command {
    Set { key: String, value: String },
    Get { key: String },
    Lpush { key: String, values: Vec<String> },
    // ... otros comandos
}
```

### Respuestas
Los tests esperan respuestas del tipo `rustidocs::command::ResponseType`:

```rust
pub enum ResponseType {
    Str(String),
    Int(i64),
    List(Vec<String>),
    Set(HashSet<String>),
    Null(String),
    // ... otros tipos
}
```

### Protocolo RESP
Los tests verifican que el protocolo RESP se implemente correctamente:
- Simple strings: `+OK\r\n`
- Errors: `-ERR message\r\n`
- Integers: `:1000\r\n`
- Bulk strings: `$6\r\nfoobar\r\n`
- Arrays: `*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n`

## Troubleshooting

### Tests lentos
Algunos tests de rendimiento pueden tardar más tiempo. Puedes ejecutarlos por separado:
```bash
cargo test test_persistence_performance -- --nocapture
cargo test test_pubsub_performance -- --nocapture
```

## Contribución

Para agregar nuevos tests:
1. Crea el test en el archivo correspondiente
2. Asegúrate de que siga el patrón de los tests existentes
3. Documenta el propósito del test
4. Ejecuta todos los tests para verificar que no rompas nada

Para modificar tests existentes:
1. Mantén la compatibilidad con la implementación actual
2. Actualiza la documentación si es necesario
3. Verifica que todos los tests sigan pasando 