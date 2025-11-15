//! Implementación de la estructura Instruction y su conversión a Command.
//!
//! Este módulo maneja la conversión de instrucciones de texto a comandos tipados,
//! incluyendo validación de argumentos y parsing de tipos de datos.
//!
//
//! # Características principales
//!
//! - Conversión de strings a comandos tipados
//! - Validación de número de argumentos
//! - Parsing de enteros con manejo de errores
//! - Soporte para todos los comandos Redis implementados

use crate::command::types::Command;
use crate::network;

/// Errores específicos que pueden ocurrir durante el parsing de instrucciones.
#[derive(Debug)]
pub enum InstructionError {
    /// Número incorrecto de argumentos para el comando
    WrongArgumentCount(String),
    /// Error al parsear un entero
    ParseIntError(String),
    /// Comando desconocido
    UnknownCommand(String),
    /// Entero fuera del rango válido
    IntegerOutOfRange,
}

impl std::fmt::Display for InstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstructionError::WrongArgumentCount(cmd) => {
                write!(f, "Wrong number of arguments for {} command", cmd)
            }
            InstructionError::ParseIntError(context) => {
                write!(f, "Invalid integer in {}", context)
            }
            InstructionError::UnknownCommand(cmd) => {
                write!(f, "Unknown command: {}", cmd)
            }
            InstructionError::IntegerOutOfRange => {
                write!(f, "Integer out of range")
            }
        }
    }
}

impl std::error::Error for InstructionError {}

/// Estructura que representa una instrucción de comando.
///
/// Una instrucción contiene el tipo de comando y sus argumentos como strings,
/// que luego se convierten a comandos tipados.
#[derive(Debug)]
pub struct Instruction {
    /// Tipo de instrucción (ej: "GET", "SET", etc.)
    pub instruction_type: String,
    /// Lista de argumentos de la instrucción
    pub arguments: Vec<String>,
}

/// Crea un mensaje de error para número incorrecto de argumentos.
///
/// # Argumentos
///
/// * `cmd` - Nombre del comando
///
/// # Retorna
///
/// String con el mensaje de error formateado
fn wrong_arg_count(cmd: &str) -> InstructionError {
    InstructionError::WrongArgumentCount(cmd.to_string())
}

/// Parsea un string a entero con manejo de errores específico.
///
/// # Argumentos
///
/// * `s` - String a parsear
/// * `context` - Contexto para el mensaje de error
///
/// # Retorna
///
/// `Result<i64, InstructionError>`
fn parse_int(s: &str, context: &str) -> Result<i64, InstructionError> {
    s.parse::<i64>()
        .map_err(|_| InstructionError::ParseIntError(context.to_string()))?
        .try_into()
        .map_err(|_| InstructionError::IntegerOutOfRange)
}

impl Instruction {
    /// Crea una nueva instancia de Instruction.
    ///
    /// # Argumentos
    ///
    /// * `instruction_type` - Tipo de instrucción
    /// * `arguments` - Lista de argumentos
    ///
    /// # Retorna
    ///
    /// Nueva instancia de `Instruction`
    pub fn new(instruction_type: String, arguments: Vec<String>) -> Self {
        Self {
            instruction_type,
            arguments,
        }
    }

    /// Convierte la instrucción a un comando tipado.
    ///
    /// Este método valida el número de argumentos y parsea los tipos
    /// de datos necesarios para crear el comando correspondiente.
    ///
    /// # Retorna
    ///
    /// `Result<Command, InstructionError>` - Comando tipado o error de parsing
    ///
    /// # Errores
    ///
    /// * `WrongArgumentCount` - Número incorrecto de argumentos
    /// * `ParseIntError` - Error al parsear enteros
    /// * `UnknownCommand` - Comando no reconocido
    /// * `IntegerOutOfRange` - Entero fuera del rango válido
    pub fn to_command(&self) -> Result<Command, InstructionError> {
        match self.instruction_type.to_uppercase().as_str() {
            "APPEND" => {
                if self.arguments.len() != 2 {
                    return Err(wrong_arg_count("APPEND"));
                }
                Ok(Command::Append(
                    self.arguments[0].clone(),
                    self.arguments[1].clone(),
                ))
            }
            "DEL" => {
                if self.arguments.is_empty() {
                    return Err(wrong_arg_count("DEL"));
                }
                Ok(Command::Del(self.arguments.clone()))
            }
            "ECHO" => {
                if self.arguments.len() != 1 {
                    return Err(wrong_arg_count("ECHO"));
                }
                Ok(Command::Echo(self.arguments[0].clone()))
            }
            "SET" => {
                if self.arguments.len() < 2 {
                    return Err(wrong_arg_count("SET"));
                }

                let key = self.arguments[0].clone();
                let value = self.arguments[1..].join(" ");

                Ok(Command::Set(key, value))
            }
            "GET" => {
                if self.arguments.len() != 1 {
                    return Err(wrong_arg_count("GET"));
                }
                Ok(Command::Get(self.arguments[0].clone()))
            }
            "GETDEL" => {
                if self.arguments.len() != 1 {
                    return Err(wrong_arg_count("GETDEL"));
                }
                Ok(Command::Getdel(self.arguments[0].clone()))
            }
            "STRLEN" => {
                if self.arguments.len() != 1 {
                    return Err(wrong_arg_count("STRLEN"));
                }
                Ok(Command::Strlen(self.arguments[0].clone()))
            }
            "GETRANGE" => {
                if self.arguments.len() != 3 {
                    return Err(wrong_arg_count("GETRANGE"));
                }
                let start = parse_int(&self.arguments[1], "start index for GETRANGE")?;
                let end = parse_int(&self.arguments[2], "end index for GETRANGE")?;
                Ok(Command::Getrange(self.arguments[0].clone(), start, end))
            }
            "SUBSTR" => {
                if self.arguments.len() != 3 {
                    return Err(wrong_arg_count("SUBSTR"));
                }
                let start = parse_int(&self.arguments[1], "start index for SUBSTR")?;
                let end = parse_int(&self.arguments[2], "end index for SUBSTR")?;
                Ok(Command::Substr(self.arguments[0].clone(), start, end))
            }
            "LLEN" => {
                if self.arguments.len() != 1 {
                    return Err(wrong_arg_count("LLEN"));
                }
                Ok(Command::Llen(self.arguments[0].clone()))
            }
            "LPOP" => {
                if self.arguments.len() != 2 {
                    return Err(wrong_arg_count("LPOP"));
                }
                let amount = parse_int(&self.arguments[1], "amount for LPOP")?;
                Ok(Command::Lpop(self.arguments[0].clone(), amount))
            }
            "RPOP" => {
                if self.arguments.len() != 2 {
                    return Err(wrong_arg_count("RPOP"));
                }
                let amount = parse_int(&self.arguments[1], "amount for RPOP")?;
                Ok(Command::Rpop(self.arguments[0].clone(), amount))
            }
            "LPUSH" => {
                if self.arguments.len() < 2 {
                    return Err(wrong_arg_count("LPUSH"));
                }
                Ok(Command::Lpush(
                    self.arguments[0].clone(),
                    self.arguments[1..].to_vec(),
                ))
            }
            "RPUSH" => {
                if self.arguments.len() < 2 {
                    return Err(wrong_arg_count("RPUSH"));
                }
                Ok(Command::Rpush(
                    self.arguments[0].clone(),
                    self.arguments[1..].to_vec(),
                ))
            }
            "LRANGE" => {
                if self.arguments.len() != 3 {
                    return Err(wrong_arg_count("LRANGE"));
                }
                let start = parse_int(&self.arguments[1], "start index for LRANGE")?;
                let end = parse_int(&self.arguments[2], "end index for LRANGE")?;
                Ok(Command::Lrange(self.arguments[0].clone(), start, end))
            }
            "SADD" => {
                if self.arguments.len() < 2 {
                    return Err(wrong_arg_count("SADD"));
                }
                Ok(Command::Sadd(
                    self.arguments[0].clone(),
                    self.arguments[1..].to_vec(),
                ))
            }
            "SMEMBERS" => {
                if self.arguments.len() != 1 {
                    return Err(wrong_arg_count("SMEMBERS"));
                }
                Ok(Command::Smembers(self.arguments[0].clone()))
            }
            "SCARD" => {
                if self.arguments.len() != 1 {
                    return Err(wrong_arg_count("SCARD"));
                }
                Ok(Command::Scard(self.arguments[0].clone()))
            }
            "SISMEMBER" => {
                if self.arguments.len() != 2 {
                    return Err(wrong_arg_count("SISMEMBER"));
                }
                Ok(Command::Sismember(
                    self.arguments[0].clone(),
                    self.arguments[1].clone(),
                ))
            }
            "SMOVE" => {
                if self.arguments.len() != 3 {
                    return Err(wrong_arg_count("SMOVE"));
                }
                Ok(Command::SMove(
                    self.arguments[0].clone(),
                    self.arguments[1].clone(),
                    self.arguments[2].clone(),
                ))
            }
            "SPOP" => {
                if self.arguments.len() != 2 {
                    return Err(wrong_arg_count("SPOP"));
                }
                let amount = parse_int(&self.arguments[1], "amount for SPOP")?;
                Ok(Command::Spop(self.arguments[0].clone(), amount))
            }
            "BGSAVE" => {
                if !self.arguments.is_empty() {
                    return Err(wrong_arg_count("BGSAVE"));
                }
                Ok(Command::BgSave)
            }
            "SAVE" => {
                if !self.arguments.is_empty() {
                    return Err(wrong_arg_count("SAVE"));
                }
                Ok(Command::Save)
            }
            "SUBSCRIBE" => {
                if self.arguments.len() != 1 {
                    return Err(wrong_arg_count("SUBSCRIBE"));
                }
                Ok(Command::Subscribe(self.arguments[0].clone()))
            }
            "UNSUBSCRIBE" => {
                if self.arguments.len() != 1 {
                    return Err(wrong_arg_count("UNSUBSCRIBE"));
                }
                Ok(Command::Unsubscribe(self.arguments[0].clone()))
            }
            "PUBLISH" => {
                if self.arguments.len() != 2 {
                    return Err(wrong_arg_count("PUBLISH"));
                }
                Ok(Command::Publish(
                    self.arguments[0].clone(),
                    network::resp_message::RespMessage::SimpleString(self.arguments[1].clone()),
                ))
            }
            "MEET" => {
                if self.arguments.len() != 1 {
                    return Err(wrong_arg_count("MEET"));
                }
                Ok(Command::Meet(self.arguments[0].clone()))
            }
            "CLUSTER" => {
                if self.arguments.len() != 1 {
                    return Err(wrong_arg_count("CLUSTER"));
                }
                if self.arguments[0].to_uppercase() == "SLOTS".to_string() {
                    return Ok(Command::Slots);
                }
                Err(InstructionError::UnknownCommand(
                    self.instruction_type.clone(),
                ))
            }
            "AUTH" => {
                if self.arguments.len() != 2 {
                    return Err(wrong_arg_count("AUTH"));
                }
                Ok(Command::Auth(
                    self.arguments[0].clone(),
                    self.arguments[1].clone(),
                ))
            }
            _ => Err(InstructionError::UnknownCommand(
                self.instruction_type.clone(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Crea una instrucción de prueba.
    fn create_test_instruction(cmd_type: &str, args: Vec<String>) -> Instruction {
        Instruction::new(cmd_type.to_string(), args)
    }

    #[test]
    fn test_instruction_new() {
        let instruction = Instruction::new("GET".to_string(), vec!["key1".to_string()]);
        assert_eq!(instruction.instruction_type, "GET");
        assert_eq!(instruction.arguments, vec!["key1"]);
    }

    #[test]
    fn test_to_command_get_success() {
        let instruction = create_test_instruction("GET", vec!["mykey".to_string()]);
        let result = instruction.to_command();
        assert!(result.is_ok());
        if let Ok(Command::Get(key)) = result {
            assert_eq!(key, "mykey");
        } else {
            panic!("Expected Command::Get");
        }
    }

    #[test]
    fn test_to_command_get_wrong_args() {
        let instruction = create_test_instruction("GET", vec![]);
        let result = instruction.to_command();
        assert!(result.is_err());
        if let Err(InstructionError::WrongArgumentCount(cmd)) = result {
            assert_eq!(cmd, "GET");
        } else {
            panic!("Expected WrongArgumentCount error");
        }
    }

    #[test]
    fn test_to_command_set_success() {
        let instruction =
            create_test_instruction("SET", vec!["key".to_string(), "value".to_string()]);
        let result = instruction.to_command();
        assert!(result.is_ok());
        if let Ok(Command::Set(key, value)) = result {
            assert_eq!(key, "key");
            assert_eq!(value, "value");
        } else {
            panic!("Expected Command::Set");
        }
    }

    #[test]
    fn test_to_command_set_multiple_values() {
        let instruction = create_test_instruction(
            "SET",
            vec![
                "key".to_string(),
                "value1".to_string(),
                "value2".to_string(),
            ],
        );
        let result = instruction.to_command();
        assert!(result.is_ok());
        if let Ok(Command::Set(key, value)) = result {
            assert_eq!(key, "key");
            assert_eq!(value, "value1 value2");
        } else {
            panic!("Expected Command::Set");
        }
    }

    #[test]
    fn test_to_command_lrange_with_ints() {
        let instruction = create_test_instruction(
            "LRANGE",
            vec!["list".to_string(), "0".to_string(), "10".to_string()],
        );
        let result = instruction.to_command();
        assert!(result.is_ok());
        if let Ok(Command::Lrange(key, start, end)) = result {
            assert_eq!(key, "list");
            assert_eq!(start, 0);
            assert_eq!(end, 10);
        } else {
            panic!("Expected Command::Lrange");
        }
    }

    #[test]
    fn test_to_command_lrange_invalid_int() {
        let instruction = create_test_instruction(
            "LRANGE",
            vec!["list".to_string(), "invalid".to_string(), "10".to_string()],
        );
        let result = instruction.to_command();
        assert!(result.is_err());
        if let Err(InstructionError::ParseIntError(context)) = result {
            assert!(context.contains("start index for LRANGE"));
        } else {
            panic!("Expected ParseIntError");
        }
    }

    #[test]
    fn test_to_command_unknown_command() {
        let instruction = create_test_instruction("UNKNOWN", vec![]);
        let result = instruction.to_command();
        assert!(result.is_err());
        if let Err(InstructionError::UnknownCommand(cmd)) = result {
            assert_eq!(cmd, "UNKNOWN");
        } else {
            panic!("Expected UnknownCommand error");
        }
    }

    #[test]
    fn test_to_command_case_insensitive() {
        let instruction = create_test_instruction("get", vec!["key".to_string()]);
        let result = instruction.to_command();
        assert!(result.is_ok());
        if let Ok(Command::Get(key)) = result {
            assert_eq!(key, "key");
        } else {
            panic!("Expected Command::Get");
        }
    }

    #[test]
    fn test_to_command_bgsave_no_args() {
        let instruction = create_test_instruction("BGSAVE", vec![]);
        let result = instruction.to_command();
        assert!(result.is_ok());
        if let Ok(Command::BgSave) = result {
            // Success
        } else {
            panic!("Expected Command::BgSave");
        }
    }

    #[test]
    fn test_to_command_bgsave_with_args() {
        let instruction = create_test_instruction("BGSAVE", vec!["arg".to_string()]);
        let result = instruction.to_command();
        assert!(result.is_err());
        if let Err(InstructionError::WrongArgumentCount(cmd)) = result {
            assert_eq!(cmd, "BGSAVE");
        } else {
            panic!("Expected WrongArgumentCount error");
        }
    }

    #[test]
    fn test_parse_int_success() {
        let result = parse_int("123", "test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 123);
    }

    #[test]
    fn test_parse_int_invalid() {
        let result = parse_int("invalid", "test");
        assert!(result.is_err());
        if let Err(InstructionError::ParseIntError(context)) = result {
            assert_eq!(context, "test");
        } else {
            panic!("Expected ParseIntError");
        }
    }

    #[test]
    fn test_instruction_error_display() {
        let error = InstructionError::WrongArgumentCount("GET".to_string());
        assert!(
            error
                .to_string()
                .contains("Wrong number of arguments for GET command")
        );
    }

    #[test]
    fn test_instruction_error_debug() {
        let error = InstructionError::UnknownCommand("TEST".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("UnknownCommand"));
        assert!(debug_str.contains("TEST"));
    }

    // TODO: Test para auth
}
