//! Este binario inicia el microservicio de indexación.
//!
//! # Uso
//! cargo run --bin microservice

use rustidocs::{app::microservice::index::Index, client_lib::cluster_manager::ClusterManager};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let addr = if args.len() > 1 {
        args[1].clone()
    } else {
        "0.0.0.0:7001".to_string()
    };

    let cluster = ClusterManager::new(addr, "super".to_string(), "1234".to_string()).unwrap();
    let mut x = Index::new(cluster);
    x.run();
}
