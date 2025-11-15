use crate::{
    app::microservice::llm::utils::LLMRequest,
    app::microservice::llm::utils::LLMResponse,
    client_lib::cluster_manager::ClusterManager,
    network::resp_message::RespMessage,
    network::resp_parser::parse_resp_line,
};
use serde_json;
use std::io::{BufReader, Read};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

const LLM_CHANNEL: &str = "LLM_REQUESTS";
const LLM_RESPONSES_CHANNEL: &str = "LLM_RESPONSES";

/// Cliente LLM que maneja las solicitudes de AI
pub struct LLMClient {
    cluster: ClusterManager,
    //response_receiver: Option<Receiver<LLMResponse>>,
}

impl LLMClient {
    pub fn new(
        redis_address: &str,
        user: &str,
        password: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let cluster = ClusterManager::new(
            redis_address.to_string(),
            user.to_string(),
            password.to_string(),
        )
        .map_err(|e| format!("Error conectando al cluster: {:?}", e))?;

        Ok(LLMClient {
            cluster,
            //response_receiver: None,
        })
    }

    /// Envía una solicitud de AI para insertar texto en una posición específica
    pub fn request_ai_insert(
        &mut self,
        document_id: String,
        prompt: String,
        position: usize,
        client_id: u64,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let request = LLMRequest {
            document_id,
            prompt,
            position: Some(position),
            selected_text: None,
            request_id: Uuid::new_v4().to_string(),
            client_id,
        };

        self.send_request(request)
    }

    /// Envía una solicitud de AI para reemplazar todo el documento
    pub fn request_ai_replace(
        &mut self,
        document_id: String,
        prompt: String,
        client_id: u64,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let request = LLMRequest {
            document_id,
            prompt,
            position: None,
            selected_text: None,
            request_id: Uuid::new_v4().to_string(),
            client_id,
        };

        self.send_request(request)
    }

    /// Envía una solicitud de AI para reemplazar texto seleccionado
    pub fn request_ai_replace_selected(
        &mut self,
        document_id: String,
        prompt: String,
        selected_text: String,
        client_id: u64,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let request = LLMRequest {
            document_id,
            prompt,
            position: None,
            selected_text: Some(selected_text),
            request_id: Uuid::new_v4().to_string(),
            client_id,
        };

        self.send_request(request)
    }

    fn send_request(&mut self, request: LLMRequest) -> Result<String, Box<dyn std::error::Error>> {
    let request_json = serde_json::to_string(&request)?;
    let request_id = request.request_id.clone();

    println!("[send_request] Publicando solicitud en canal {}", LLM_CHANNEL);

    
    // Suscribirse al canal de respuestas para recibir la respuesta
    let mut response_stream = self
        .cluster
        .subscribe(LLM_RESPONSES_CHANNEL)
        .map_err(|e| {
            println!("[send_request] Error suscribiéndose al canal de respuestas: {:?}", e);
            format!("Error suscribiéndose al canal de respuestas: {:?}", e)
        })?;

        // Enviar la solicitud
    self.cluster
        .publish(LLM_CHANNEL, request_json.as_bytes())
        .map_err(|e| {
            println!("[send_request] Error publicando solicitud: {:?}", e);
            format!("Error publicando solicitud: {:?}", e)
        })?;

    println!("[send_request] Suscribiéndose al canal de respuestas {}", LLM_RESPONSES_CHANNEL);


        // HAGO QUE EL READ SEA NO BLOQUEANTE
    response_stream
    .set_nonblocking(true)
    .map_err(|e| format!("No se pudo poner el stream en modo no bloqueante: {:?}", e))?;

    // Esperar la respuesta del microservicio
    let mut buffer = vec![0; 4096];
    let mut attempts = 0;
    let max_attempts = 300; // 30 segundos de timeout

    while attempts < max_attempts {
        println!("[send_request] Intento {} esperando respuesta...", attempts + 1);
        match response_stream.read(&mut buffer) {
            Ok(0) => {
                println!("[send_request] Conexión cerrada por el servidor");
                return Err("Conexión cerrada por el servidor".into());
            }
            Ok(n) => {
                println!("[send_request] Leídos {} bytes del canal de respuesta", n);
                let mut reader = BufReader::new(&buffer[..n]);
                match parse_resp_line(&mut reader) {
                    Ok(RespMessage::SimpleString(content)) => {
                        println!("[send_request] Recibido SimpleString: {}", content);
                        // Intentar parsear la respuesta como JSON
                        if let Ok(response) = serde_json::from_str::<LLMResponse>(&content) {
                            println!("[send_request] Parseado LLMResponse correctamente (SimpleString)");
                            if response.request_id == request_id {
                                println!("[send_request] request_id coincide (SimpleString)");
                                if let Some(error) = response.error {
                                    println!("[send_request] Error de AI: {}", error);
                                    return Err(format!("Error de AI: {}", error).into());
                                }
                                println!("[send_request] Respuesta exitosa (SimpleString)");
                                return Ok(response.generated_text);
                            } else {
                                println!("[send_request] request_id NO coincide (SimpleString)");
                            }
                        } else {
                            println!("[send_request] No se pudo parsear LLMResponse (SimpleString)");
                        }
                    }
                    Ok(RespMessage::BulkString(Some(content))) => {
                        let content_str = String::from_utf8_lossy(&content);
                        println!("[send_request] Recibido BulkString: {}", content_str);
                        if let Ok(response) = serde_json::from_str::<LLMResponse>(&content_str) {
                            println!("[send_request] Parseado LLMResponse correctamente (BulkString)");
                            if response.request_id == request_id {
                                println!("[send_request] request_id coincide (BulkString)");
                                if let Some(error) = response.error {
                                    println!("[send_request] Error de AI: {}", error);
                                    return Err(format!("Error de AI: {}", error).into());
                                }
                                println!("[send_request] Respuesta exitosa (BulkString)");
                                return Ok(response.generated_text);
                            } else {
                                println!("[send_request] request_id NO coincide (BulkString)");
                            }
                        } else {
                            println!("[send_request] No se pudo parsear LLMResponse (BulkString)");
                        }
                    }
                    Ok(other) => {
                        println!("[send_request] Recibido otro tipo de mensaje: {:?}", other);
                        // Continuar esperando
                    }
                    Err(e) => {
                        println!("[send_request] Error parseando respuesta: {:?}", e);
                        // Continuar esperando
                    }
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    thread::sleep(Duration::from_millis(100));
                } else {
                    println!("[send_request] Error leyendo respuesta: {}", e);
                    return Err(format!("Error leyendo respuesta: {}", e).into());
                }
            }
        }
        attempts += 1;
    }

    println!("[send_request] Timeout esperando respuesta del microservicio LLM");
    Err("Timeout esperando respuesta del microservicio LLM".into())
}

}
