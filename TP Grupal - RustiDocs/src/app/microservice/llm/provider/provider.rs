use crate::app::microservice::llm::utils::LLMResponse;
use crate::app::microservice::llm::utils::LLMRequest;

pub trait LLMProvider {
    fn proccess_request(
        &self,
        request: &LLMRequest,
    ) -> LLMResponse; // El LLMResponse tiene el error integrado y manejado por el propio servicio
}   