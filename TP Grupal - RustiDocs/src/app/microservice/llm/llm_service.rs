use crate::app::utils::ThreadPool;
use crate::{client_lib::cluster_manager::ClusterManager, network::resp_parser::parse_resp_line};
use std::io::Read;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;
use std::{io::BufReader, net::TcpStream, thread};
use crate::app::microservice::llm::provider::provider::LLMProvider;
use crate::app::microservice::llm::utils::{LLMRequest, LLMResponse};
//const VERSION_TO_SAVE: u64 = 1;
const LLM_CHANNEL: &str = "LLM_REQUESTS";
const LLM_RESPONSES_CHANNEL: &str = "LLM_RESPONSES";



/// Microservicio LLM que maneja solicitudes de generación de texto usando Gemini
pub struct LLMService<T>
where T: LLMProvider  {
    cluster_data: ClusterManager,
    redis_stream: TcpStream,
    //control_service: ControlService<String, TextOperation>,
    provider: T,
    pool: ThreadPool,
    response_tx: Sender<LLMResponse>,
    response_rx: Receiver<LLMResponse>,
}

impl<T> LLMService<T>
where T: LLMProvider + Clone + Send + 'static {
    pub fn new(redis_address: String, provider: T) -> Result<Self, std::io::Error> {
        let mut cluster_data = ClusterManager::new(
            redis_address.clone(),
            "super".to_string(),
            "1234".to_string(),
        )
        .map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Cluster error: {:?}", e))
        })?;

        let sub_channel = cluster_data.subscribe(LLM_CHANNEL).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Subscribe error: {:?}", e),
            )
        })?;
        sub_channel.set_nonblocking(true)?;

        let (response_tx, response_rx) = channel::<LLMResponse>();

        Ok(LLMService {
            cluster_data,
            redis_stream: sub_channel,
            provider,
            pool: ThreadPool::new(10),
            response_tx,
            response_rx,
        })
    }

    pub fn run(&mut self) {
        let mut buffer = vec![0; 4096];
        loop {
            self.peek_and_publish_response();
            match self.redis_stream.read(&mut buffer) {
                Ok(0) => {
                    println!("[LLM_SERVICE] Conexión cerrada");
                    break;
                }
                Ok(n) => {
                    let mut reader = BufReader::new(&buffer[..n]);
                    match parse_resp_line(&mut reader) {
                        Err(e) => {
                            eprintln!("[LLM_SERVICE] Error leyendo del socket: {}", e);
                            break;
                        }
                        Ok(contenido) => {
                            if let crate::network::RespMessage::SimpleString(content) = contenido {
                                self.handle_message(content);
                            }
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    eprintln!("[LLM_SERVICE] Error leyendo del socket: {}", e);
                    break;
                }
            }
        }
    }

    fn peek_and_publish_response(&mut self) {
        if let Ok(response) = self.response_rx.try_recv() {
            if let Ok(response_json) = serde_json::to_string(&response) {
                let _ = self
                    .cluster_data
                    .publish(LLM_RESPONSES_CHANNEL, response_json.as_bytes());
                println!(
                    "[LLM_SERVICE] Respuesta publicada para documento: {}",
                    response.document_id
                );
            } else {
                eprintln!("[LLM_SERVICE] Error serializando respuesta");
            }
        }
    }

    fn handle_message(&mut self, contenido: String) {
        if let Ok(request) = serde_json::from_str::<LLMRequest>(&contenido) {
            println!("[LLM_SERVICE] Recibida solicitud LLM: {:?}", request);
            let sender = self.response_tx.clone();

            let provider = self.provider.clone();
            match self.pool.spawn(move || Self::process_llm_request(provider, request, sender)) {
                Ok(_) => {},
                Err(e) if e.to_string().contains("No hay suficientes threads disponibles") => {
                    eprintln!("[LLM_SERVICE] Thread queue limit reached: {}", e);
                    // Optionally, send a specific error response here
                }
                Err(e) => {
                    eprintln!("[LLM_SERVICE] Failed to spawn thread: {}", e);
                    // Optionally, send a generic error response here
                }
            }
        } else {
            println!("[LLM_SERVICE] Mensaje no reconocido como solicitud LLM");
        }
    }

    fn process_llm_request(
        provider: T,
        request: LLMRequest,
        response_sender: Sender<LLMResponse>,
    ) {
        let response = provider.proccess_request(&request); 
         
        if response_sender.send(response).is_err() {
            println!("Error al enviar la request al cliente");
        }
    }
}
