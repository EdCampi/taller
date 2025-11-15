use serde::{Deserialize, Serialize};


/// Estructura para solicitudes de LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub document_id: String,
    pub prompt: String,
    pub position: Option<usize>, // Posición específica en el documento (opcional)
    pub selected_text: Option<String>, // Texto seleccionado para reemplazar (opcional)
    pub request_id: String,
    pub client_id: u64,
}

/// Estructura para respuestas de LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub document_id: String,
    pub request_id: String,
    pub client_id: u64,
    pub generated_text: String,
    pub position: Option<usize>,
    pub selected_text: Option<String>, // Texto original seleccionado
    pub error: Option<String>,
}