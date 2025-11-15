use serde::{Deserialize, Serialize};

use crate::app::microservice::llm::{
    provider::provider::LLMProvider,
    utils::{LLMRequest, LLMResponse},
};

/// Estructura para la API de Gemini
#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    generation_config: GeminiGenerationConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
    role: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    max_output_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
    //finish_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GeminiProvider {
    api_key: String,
}

impl LLMProvider for GeminiProvider {
    fn proccess_request(&self, request: &LLMRequest) -> LLMResponse {
        match Self::call_api(request, &self.api_key) {
            Ok(generated_text) => {
                return LLMResponse {
                    document_id: request.document_id.clone(),
                    request_id: request.request_id.clone(),
                    client_id: request.client_id,
                    generated_text,
                    position: request.position,
                    selected_text: request.selected_text.clone(),
                    error: None,
                };
            }
            Err(e) => {
                return LLMResponse {
                    document_id: request.document_id.clone(),
                    request_id: request.request_id.clone(),
                    client_id: request.client_id,
                    generated_text: String::new(),
                    position: request.position,
                    selected_text: request.selected_text.clone(),
                    error: Some(e.to_string()),
                };
            }
        };
    }
}

impl GeminiProvider {
    pub fn new(api_key: String) -> Self {
        GeminiProvider { api_key }
    }

    fn build_prompt(request: &LLMRequest) -> String {
        // Construir el prompt según el tipo de solicitud
        let prompt_content = if let Some(selected_text) = &request.selected_text {
            format!(
                "# Tarea: Reescritura de texto\n\n\
                ## Contexto:\n\
                Necesito que reescribas el siguiente texto según las instrucciones específicas.\n\n\
                ## Texto original:\n\
                ```\n{}\n```\n\n\
                ## Instrucciones:\n\
                {}\n\n\
                ## Formato de salida:\n\
                - Proporciona ÚNICAMENTE el texto reescrito\n\
                - No incluyas explicaciones, comentarios o metadatos\n\
                - Mantén la codificación UTF-8\n\
                - No uses comillas ni formateo adicional\n\n\
                ## Resultado:",
                selected_text, request.prompt
            )
        } else if let Some(_position) = request.position {
            format!(
                "# Tarea: Generación de texto para inserción\n\n\
                ## Contexto:\n\
                Necesito que generes texto para insertar en un documento específico.\n\n\
                ## Instrucciones:\n\
                {}\n\n\
                ## Formato de salida:\n\
                - Proporciona ÚNICAMENTE el texto a insertar\n\
                - No incluyas explicaciones, comentarios o metadatos\n\
                - Mantén la codificación UTF-8\n\
                - No uses comillas ni formateo adicional\n\
                - El texto debe ser coherente para inserción\n\n\
                ## Resultado:",
                request.prompt
            )
        } else {
            format!(
                "# Tarea: Creación/modificación de documento completo\n\n\
                ## Contexto:\n\
                Necesito que generes o modifiques un documento completo según las instrucciones.\n\n\
                ## Instrucciones:\n\
                {}\n\n\
                ## Formato de salida:\n\
                - Proporciona ÚNICAMENTE el contenido final del documento\n\
                - No incluyas explicaciones, comentarios o metadatos\n\
                - Mantén la codificación UTF-8\n\
                - No uses comillas ni formateo adicional\n\
                - Asegúrate de que sea un documento completo y coherente\n\n\
                - IMPORTANTE: El texto no debe superar los 100 caracteres.\n\n\
                ## Resultado:",
                request.prompt
            )
        };

        prompt_content
    }

    fn call_api(request: &LLMRequest, api_key: &str) -> Result<String, Box<dyn std::error::Error>> {
        let client = reqwest::blocking::Client::new();

        let prompt_content = Self::build_prompt(request);

        let gemini_request = GeminiRequest {
            contents: vec![
                GeminiContent {
                    parts: vec![
                        GeminiPart {
                            text: "Eres un asistente de escritura especializado. Tu función es procesar texto siguiendo instrucciones precisas. Siempre respondes únicamente con el contenido solicitado, sin explicaciones adicionales y manteniendo solo la codificación UTF-8 .".to_string(),
                        },
                        GeminiPart {
                            text: prompt_content,
                        },
                    ],
                    role: Some("user".to_string()),
                },
            ],
            generation_config: GeminiGenerationConfig {
                max_output_tokens: 2048,
                temperature: 0.3,
            },
        };

        let response = client
            .post("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent")
            .header("Content-Type", "application/json")
            .header("X-goog-api-key", api_key)
            .json(&gemini_request)
            .send()?;

        if !response.status().is_success() {
            let error_text = response.text().unwrap_or_default();
            return Err(format!("Error de API Gemini: {}", error_text).into());
        }

        let gemini_response: GeminiResponse = response.json()?;

        if let Some(candidate) = gemini_response.candidates.first() {
            if let Some(part) = candidate.content.parts.first() {
                Ok(part.text.clone())
            } else {
                Err("No se recibió contenido válido de Gemini".into())
            }
        } else {
            Err("No se recibió respuesta válida de Gemini".into())
        }
    }
}
