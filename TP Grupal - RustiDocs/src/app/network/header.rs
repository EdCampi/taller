use crate::app::operation::generic::{Instruction, ParsableBytes};

/// Define si la instrucción es una solicitud o una respuesta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionType {
    Request = 0,
    Response = 1,
}

const INSTRUCTION: u8 = 0;
const STATE: u8 = 1;
const INIT: u8 = 2;
const RESYNC: u8 = 3;

#[derive(Debug, PartialEq)]
pub enum Message<D, O>
where
    O: Clone + ParsableBytes,
    D: ParsableBytes,
{
    Instruction(InstructionType, Instruction<O>),
    Init(u64),
    State(D, u64, u64),
    Resync,
}

impl<D, O> Message<D, O>
where
    O: Clone + ParsableBytes,
    D: ParsableBytes,
{
    pub fn create_request(instruction: Instruction<O>) -> Self {
        Message::Instruction(InstructionType::Request, instruction)
    }
    pub fn create_response(instruction: Instruction<O>) -> Self {
        Message::Instruction(InstructionType::Response, instruction)
    }

    pub fn message_to_pub(&self, channel_name: &str) -> Vec<u8> {
        match self {
            Message::Instruction(instruction_type, instruction) => {
                let mut argument: Vec<u8> = Vec::new();
                let instruction_bytes = instruction.to_bytes();
                argument.push(INSTRUCTION);
                argument.push(*instruction_type as u8);
                argument.extend_from_slice(&instruction_bytes);
                create_pub_string(channel_name.to_string(), &argument)
            }
            Message::State(state, version, client_id) => {
                let mut argument: Vec<u8> = Vec::new();
                argument.push(STATE);
                argument.extend_from_slice(&client_id.to_le_bytes()); // u64 como 8 bytes little endian
                argument.extend_from_slice(&version.to_le_bytes()); // u64 como 8 bytes little endian
                argument.extend_from_slice(&state.to_bytes()); // bytes del state
                create_pub_string(channel_name.to_string(), &argument)
            }
            Message::Init(client_id) => {
                let mut argument: Vec<u8> = Vec::new();
                argument.push(INIT);
                argument.extend_from_slice(&client_id.to_le_bytes());
                create_pub_string(channel_name.to_string(), &argument)
            }
            Message::Resync => {
                let argument = vec![RESYNC];
                create_pub_string(channel_name.to_string(), &argument)
            }
        }
    }

    pub fn resp_to_message(resp_str: &str) -> Option<Message<D, O>> {
        // Asumimos que la entrada es una cadena hexadecimal que representa los bytes originales
        println!("Convirtiendo RESP a mensaje: ");
        println!("{}", resp_str);
        let resp = hex_string_to_bytes(resp_str)?;

        match resp.first() {
            Some(&INSTRUCTION) => {
                if resp.len() < 3 {
                    return None; // No hay suficiente contenido para procesar
                }
                let instruction_type = match resp[1] {
                    0 => InstructionType::Request,
                    1 => InstructionType::Response,
                    _ => return None, // Tipo de instrucción desconocido
                };
                let instruction_bytes = &resp[2..];
                let (instruction, _) = Instruction::<O>::from_bytes(instruction_bytes)?;
                Some(Message::Instruction(instruction_type, instruction))
            }
            Some(&STATE) => {
                // STATE | client_id (8 bytes) | version (8 bytes) | state_bytes
                if resp.len() < 1 + 8 + 8 {
                    return None;
                }
                let client_id = u64::from_le_bytes(resp[1..9].try_into().ok()?);
                let version = u64::from_le_bytes(resp[9..17].try_into().ok()?);
                let state_bytes = &resp[17..];
                let (state, _) = D::from_bytes(state_bytes)?;
                Some(Message::State(state, version, client_id))
            }
            Some(&INIT) => {
                // INIT | client_id (8 bytes)
                if resp.len() < 1 + 8 {
                    return None;
                }
                let client_id = u64::from_le_bytes(resp[1..9].try_into().ok()?);
                Some(Message::Init(client_id))
            }
            Some(&RESYNC) => {
                // Solo el byte RESYNC
                if resp.len() != 1 {
                    return None;
                }
                Some(Message::Resync)
            }
            _ => None, // No es un mensaje de instrucción
        }
    }
}

// Funciones auxiliares para convertir entre bytes y String hexadecimal
fn bytes_to_hex_string(bytes: &[u8]) -> String {
    let mut hex_string = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        hex_string.push_str(&format!("{:02x}", byte));
    }
    hex_string
}

fn hex_string_to_bytes(hex_string: &str) -> Option<Vec<u8>> {
    // Verificar que la longitud sea par
    if hex_string.len() % 2 != 0 {
        return None;
    }

    let mut bytes = Vec::with_capacity(hex_string.len() / 2);
    let mut chars = hex_string.chars();

    while let (Some(a), Some(b)) = (chars.next(), chars.next()) {
        // Convertir dos caracteres hexadecimales en un byte
        let high = match a.to_digit(16) {
            Some(val) => val as u8,
            None => return None,
        };

        let low = match b.to_digit(16) {
            Some(val) => val as u8,
            None => return None,
        };

        bytes.push(high << 4 | low);
    }

    Some(bytes)
}

fn create_pub_string(channel_name: String, argument_bytes: &[u8]) -> Vec<u8> {
    let mut resp = Vec::new();
    resp.extend_from_slice(b"*3\r\n");
    resp.extend_from_slice(b"$7\r\nPUBLISH\r\n");
    resp.extend_from_slice(format!("${}\r\n", channel_name.len()).as_bytes());
    resp.extend_from_slice(channel_name.as_bytes());
    resp.extend_from_slice(b"\r\n");

    // Convertimos los bytes a representación hexadecimal para preservar la información
    let hex_data = bytes_to_hex_string(argument_bytes);
    resp.extend_from_slice(format!("${}\r\n", hex_data.len()).as_bytes());
    resp.extend_from_slice(hex_data.as_bytes());
    resp.extend_from_slice(b"\r\n");

    resp
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::{
        app::operation::{generic::InstructionId, text::TextOperation},
        network::resp_parser::parse_resp_line,
    };

    use super::*;

    #[test]
    fn test_hex_conversion() {
        let original = vec![0, 1, 2, 3, 255, 254];
        let hex = bytes_to_hex_string(&original);
        assert_eq!(hex, "00010203fffe");
        let bytes = hex_string_to_bytes(&hex).unwrap();
        assert_eq!(bytes, original);
    }

    /*  #[test]
    fn test_create_pub_string() {
        let channel_name = "test_channel".to_string();
        let argument = vec![INSTRUCTION, 0, 1, 2, 3];
        let pub_message = create_pub_string(channel_name.clone(), &argument);

        // El argumento ahora es una cadena hex "0000010203"
        let hex_argument = bytes_to_hex_string(&argument);
        let expected = format!(
            "*3\r\n$7\r\nPUBLISH\r\n${}\r\n{}\r\n${}\r\n{}\r\n",
            channel_name.len(),
            channel_name,
            hex_argument.len(),
            hex_argument
        );
        assert_eq!(pub_message, expected);
    }*/

    #[test]
    fn test_create_pub_string_to_message() {
        let operation = TextOperation::Delete { position: 20 };
        let instruction = Instruction {
            operation_id: InstructionId {
                client_id: 20,
                local_seq: 20,
            },
            base_version: 0,
            operation,
        };
        let message: Message<String, TextOperation> = Message::create_request(instruction.clone());

        let publish = message.message_to_pub("lol");

        let mut cursor = Cursor::new(publish);
        let x = parse_resp_line(&mut cursor).unwrap();

        // Llama a try_from para convertir RespMessage en instruccioón -> devuelve Instruction
        let instruction_command = crate::command::Instruction::try_from(x).unwrap();
        println!("{}", &instruction_command.arguments[1]);
        let mes: Message<String, TextOperation> =
            Message::resp_to_message(&instruction_command.arguments[1]).unwrap();

        assert_eq!(mes, Message::create_request(instruction));
    }
}
