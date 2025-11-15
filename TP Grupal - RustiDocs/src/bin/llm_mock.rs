//! Este binario inicia el microservicio de LLM (Large Language Model).
//!
//! # Uso
//! cargo run --bin llm_service -- <redis_address> <gemini_api_key>
//!
//! Ejemplo:
//! cargo run --bin llm_service -- 0.0.0.0:7001 $GEMINI_API_KEY
//! (configurar variable de entorno GEMINI_API_KEY, con export GEMINI_API_KEY="your-gemini-api-key-here"
//! o pegar la api key directamente)

use rustidocs::app::microservice::llm::{llm_service::LLMService};
use std::env;
use rustidocs::app::microservice::llm::provider::mock::MockProvider;
fn main() {
    /*let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Uso: cargo run --bin llm_service -- <redis_address> <gemini_api_key>");
        eprintln!(
            "Ejemplo: cargo run --bin llm_service -- 0.0.0.0:7001 \"your-gemini-api-key-here\""
        );
        std::process::exit(1);
    }

    let redis_address = args[1].clone();
    let gemini_api_key = args[2].clone();

    println!("🚀 Iniciando microservicio LLM...");
    println!("📡 Conectando a Redis: {}", redis_address);
    println!("🤖 Configurando Gemini API...");

    let gemini_provider = GeminiProvider::new(gemini_api_key);*/
    let redis_address = "0.0.0.0:7001".to_string();
    let provider = MockProvider::new();

    match LLMService::new(redis_address, provider) {
        Ok(mut service) => {
            println!("✅ Microservicio LLM iniciado correctamente");
            println!("🎧 Escuchando solicitudes en el canal LLM_REQUESTS...");
            service.run();
        }
        Err(e) => {
            eprintln!("❌ Error iniciando el microservicio LLM: {}", e);
            std::process::exit(1);
        }
    }
}
