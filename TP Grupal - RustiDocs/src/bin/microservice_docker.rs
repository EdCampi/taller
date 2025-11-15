use rustidocs::{app::microservice::index::Index, client_lib::cluster_manager::ClusterManager};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let addr = if args.len() > 1 {
        args[1].clone()
    } else {
        "node_1:7001".to_string()
    };

    println!("🚀 Iniciando microservicio de indexación...");
    println!("📡 Conectando a Redis: {}", addr);

    // Para Docker, usamos solo la dirección inicial sin cluster switching
    match ClusterManager::new(addr.clone(), "super".to_string(), "1234".to_string()) {
        Ok(cluster) => {
            println!("✅ Conectado exitosamente al cluster");
            let mut index_service = Index::new(cluster);
            
            // Forzar que siempre use node_1 para evitar problemas de switching en Docker
            println!("🔧 Configurando para entorno Docker - deshabilitando cluster switching");
            
            index_service.run();
        }
        Err(e) => {
            eprintln!("❌ Error conectando al cluster: {:?}", e);
            std::process::exit(1);
        }
    }
}
