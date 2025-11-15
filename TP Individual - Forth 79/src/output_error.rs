/// enum de Errores que pueden resultar de la ejecución del programa.
pub enum Error {
    Underflow,
    Overflow,
    DivisionByZero,
    InvalidWord,
    UnknownWord,
}

impl Error {
    /// Descriociones a imprimir de los errores.
    fn description(&self) -> String {
        match *self {
            Error::Underflow => "stack-underflow\n".to_string(), // Saldría al hacer POP
            Error::Overflow => "stack-overflow\n".to_string(),   // Saldría al hacer PUSH
            Error::DivisionByZero => "division-by-zero\n".to_string(),
            Error::InvalidWord => "invalid-word\n".to_string(),
            Error::UnknownWord => "?\n".to_string(),
        }
    }

    /// Levanta el error en la salida.
    pub fn throw_error(&self, buffer: &mut Vec<String>) -> bool {
        buffer.push(self.description());
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_underflow_description() {
        let error = Error::Underflow;
        assert_eq!(error.description(), "stack-underflow\n".to_string());
    }

    #[test]
    fn test_overflow_description() {
        let error = Error::Overflow;
        assert_eq!(error.description(), "stack-overflow\n".to_string());
    }

    #[test]
    fn test_division_by_zero_description() {
        let error = Error::DivisionByZero;
        assert_eq!(error.description(), "division-by-zero\n".to_string());
    }

    #[test]
    fn test_invalid_word_description() {
        let error = Error::InvalidWord;
        assert_eq!(error.description(), "invalid-word\n".to_string());
    }

    #[test]
    fn test_unknown_word_description() {
        let error = Error::UnknownWord;
        assert_eq!(error.description(), "?\n".to_string());
    }

    #[test]
    fn test_underflow_correctly_pushes_on_buffer() {
        let error = Error::Underflow;
        let mut buffer = Vec::new();

        assert_eq!(error.throw_error(&mut buffer), false);
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0], "stack-underflow\n".to_string());
    }

    #[test]
    fn test_overflow_correctly_pushes_on_buffer() {
        let error = Error::Overflow;
        let mut buffer = Vec::new();

        assert_eq!(error.throw_error(&mut buffer), false);
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0], "stack-overflow\n".to_string());
    }

    #[test]
    fn test_division_by_zero_correctly_pushes_on_buffer() {
        let error = Error::DivisionByZero;
        let mut buffer = Vec::new();

        assert_eq!(error.throw_error(&mut buffer), false);
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0], "division-by-zero\n".to_string());
    }

    #[test]
    fn test_invalid_word_correctly_pushes_on_buffer() {
        let error = Error::InvalidWord;
        let mut buffer = Vec::new();

        assert_eq!(error.throw_error(&mut buffer), false);
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0], "invalid-word\n".to_string());
    }

    #[test]
    fn test_unknown_word_correctly_pushes_on_buffer() {
        let error = Error::UnknownWord;
        let mut buffer = Vec::new();

        assert_eq!(error.throw_error(&mut buffer), false);
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0], "?\n".to_string());
    }
}
