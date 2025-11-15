//! Binario principal para iniciar un nodo del cluster Redis.
//!
//! Este binario permite iniciar un nodo del cluster con las siguientes opciones:
//! - Iniciar como primer nodo del cluster (master)
//! - Unirse a un cluster existente especificando un nodo conocido
//!
//! # Uso
//!
//! ```bash
//! # Iniciar como primer nodo del cluster
//! cargo run --bin node utils/nodes/node_1/node_1.conf
//!
//! # Unirse a un cluster existente
//! cargo run --bin node utils/nodes/node_2/node_2.conf 0.0.0.0:7001
//! ```
//!
//! # Argumentos
//!
//! - `config_path`: Ruta al archivo de configuración del nodo
//! - `known_node` (opcional): Dirección IP:puerto de un nodo conocido para unirse al cluster
//!
//! # Ejemplos de configuración
//!
//! Ver archivos de ejemplo en `nodes/` para diferentes configuraciones de nodos.

use rustidocs::cluster::cluster_node::ClusterNode;
use rustidocs::config::node_configs::NodeConfigs;
use std::io::Error;
use std::{env, io, process};

/// Función principal del binario.
///
/// Parsea los argumentos de línea de comandos e inicia el nodo del cluster.
/// Si hay errores en los argumentos o la configuración, muestra un mensaje
/// de ayuda y termina con código de error.
///
/// # Returns
///
/// * `Ok(())` - Nodo iniciado exitosamente (nunca retorna en condiciones normales)
/// * `Err(Error)` - Error durante la inicialización
fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();

    if let Err(e) = start_node(args) {
        eprintln!("Error: {}", e);
        print_usage();
        process::exit(1);
    }

    Ok(())
}

/// Inicia el nodo del cluster con los argumentos proporcionados.
///
/// Esta función maneja toda la lógica de inicialización del nodo:
/// 1. Parsea los argumentos de línea de comandos
/// 2. Carga la configuración del nodo
/// 3. Crea e inicia el nodo del cluster
/// 4. Se conecta al cluster existente o lo inicializa como primer nodo
///
/// # Arguments
///
/// * `args` - Vector de argumentos de línea de comandos
///
/// # Returns
///
/// * `Ok(())` - Nodo iniciado exitosamente
/// * `Err(Error)` - Error durante la inicialización
///
/// # Errors
///
/// Esta función puede fallar si:
/// - No se proporcionan suficientes argumentos
/// - El archivo de configuración no existe o es inválido
/// - No se puede crear o iniciar el nodo del cluster
fn start_node(args: Vec<String>) -> Result<(), Error> {
    // Validar argumentos mínimos
    if args.len() < 2 {
        return Err(Error::new(
            io::ErrorKind::InvalidInput,
            "Se requiere al menos un archivo de configuración",
        ));
    }

    // Parsear argumentos
    let config_path = &args[1];
    let known_node = if args.len() > 2 {
        Some(args[2].clone())
    } else {
        None
    };

    println!("Iniciando nodo del cluster...");
    println!("[AOF-LOGGER] Archivo de configuración: {}", config_path);

    if let Some(ref node) = known_node {
        println!("Nodo conocido para unirse: {}", node);
    } else {
        println!("[CLUSTER] Iniciando como primer nodo del cluster");
    }

    // Cargar configuración
    let config = parse_config(config_path)?;
    println!("Configuración cargada exitosamente");

    // Crear e iniciar nodo
    let mut node = ClusterNode::new(config)
        .map_err(|e| Error::new(io::ErrorKind::Other, format!("Error creando nodo: {}", e)))?;

    println!("[NODO] Nodo creado exitosamente, iniciando...");

    node.start(known_node)
        .map_err(|e| Error::new(io::ErrorKind::Other, format!("Error iniciando nodo: {}", e)))?;

    Ok(())
}

/// Parsea y carga la configuración del nodo desde un archivo.
///
/// Esta función lee el archivo de configuración especificado y crea
/// una instancia de `NodeConfigs` con los parámetros del nodo.
///
/// # Arguments
///
/// * `config_path` - Ruta al archivo de configuración
///
/// # Returns
///
/// * `Ok(NodeConfigs)` - Configuración cargada exitosamente
/// * `Err(Error)` - Error durante la carga de la configuración
///
/// # Errors
///
/// Esta función puede fallar si:
/// - El archivo de configuración no existe
/// - El archivo tiene un formato inválido
/// - Los parámetros de configuración son incorrectos
fn parse_config(config_path: &str) -> Result<NodeConfigs, Error> {
    NodeConfigs::new(config_path).map_err(|e| {
        Error::new(
            io::ErrorKind::Other,
            format!("Error cargando configuración: {}", e),
        )
    })
}

/// Imprime el mensaje de uso del binario.
///
/// Muestra información sobre cómo usar el binario, incluyendo
/// los argumentos requeridos y opcionales, así como ejemplos de uso.
fn print_usage() {
    println!();
    println!("Uso: cargo run --bin node <config_path> [nodo_conocido]");
    println!();
    println!("Argumentos:");
    println!("  config_path    Ruta al archivo de configuración del nodo");
    println!("  nodo_conocido  (Opcional) Dirección IP:puerto de un nodo conocido");
    println!();
    println!("Ejemplos:");
    println!("  cargo run --bin node nodes/node1.conf");
    println!("  cargo run --bin node nodes/node2.conf 0.0.0.0:7001");
    println!();
    println!("Archivos de configuración:");
    println!("  Ver archivos de ejemplo en nodes/ para diferentes configuraciones");
}
