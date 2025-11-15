use crate::app::microservice::llm::utils::LLMRequest;
use crate::app::microservice::llm::utils::LLMResponse;
use crate::app::microservice::llm::provider::provider::LLMProvider;

#[derive(Debug, Clone)]

pub struct MockProvider;

impl MockProvider {
    pub fn new() -> Self {
        MockProvider
    }
}

impl LLMProvider for MockProvider{
    fn proccess_request(&self, request: &LLMRequest) -> LLMResponse {
        LLMResponse {
            document_id: request.document_id.clone(),
            request_id: request.request_id.clone(),
            client_id: request.client_id,
            generated_text: "Bottom Text".to_string(),
            position: request.position,
            selected_text: request.selected_text.clone(),
            error: None, 
        }

        
    }
}