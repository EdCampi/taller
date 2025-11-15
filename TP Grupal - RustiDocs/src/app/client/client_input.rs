use crate::app::network::header::{InstructionType, Message};
use crate::app::network::redis_parser::content_to_message;
use crate::app::operation::generic::Instruction;
use crate::app::operation::generic::ParsableBytes;
use crate::network::resp_parser::parse_resp_line;
use std::io::BufReader;
use std::net::TcpStream;
use std::sync::mpsc::Sender;

use std::marker::PhantomData;

pub struct ClientInput<D, O>
where
    O: Clone + ParsableBytes,
    D: ParsableBytes,
{
    pub socket: TcpStream,
    pub sender: Sender<Instruction<O>>,
    _client_id: u64,
    _marker: PhantomData<D>,
}

impl<D, O> ClientInput<D, O>
where
    O: Clone + ParsableBytes + std::fmt::Debug,
    D: Clone + ParsableBytes,
{
    pub fn new(socket: TcpStream, sender: Sender<Instruction<O>>, client_id: u64) -> Self {
        ClientInput::<D, O> {
            socket,
            sender,
            _client_id: client_id,
            _marker: PhantomData,
        }
    }

    pub fn run(&mut self) {
        let mut reader = BufReader::new(self.socket.try_clone().unwrap());
        println!("ClientInput: Iniciando bucle de lectura para documento");

        loop {
            match parse_resp_line(&mut reader) {
                Err(e) => {
                    eprintln!("Error leyendo del socket: {}", e);
                    break;
                }
                Ok(contenido) => {
                    println!("ClientInput: Recibido mensaje del servidor");

                    if let Some(message) = content_to_message::<D, O>(contenido) {
                        match message {
                            Message::Instruction(InstructionType::Response, operation) => {
                                println!(
                                    "ClientInput: Recibida operación RESPONSE: {:?}",
                                    operation
                                );

                                // Procesar tanto REQUEST como RESPONSE
                                if let Err(err) = self.sender.send(operation) {
                                    eprintln!("Error enviando operación al canal: {}", err);
                                    break;
                                }
                            }
                            Message::Instruction(InstructionType::Request, operation) => {
                                println!(
                                    "ClientInput: Recibida operación REQUEST: {:?}",
                                    operation
                                );
                            }
                            _ => {
                                println!("ClientInput: Tipo de mensaje ignorado");
                                continue;
                            }
                        }
                    } else {
                        println!("ClientInput: No se pudo parsear el mensaje");
                        continue;
                    }
                }
            }
        }

        println!("ClientInput: Terminado bucle de lectura");
    }
}

#[cfg(test)]
mod tests {
    use crate::app::network::redis_parser::read_resp_bulk_string;
    use std::io::Cursor;

    #[test]
    fn test_read_resp_bulk_string() {
        let data = b"$5\r\nhello\r\n";
        let mut _cursor = Cursor::new(data);
        let data = b"$5\r\nhello\r\n$5\r\nworld\r\n";
        let mut cursor = Cursor::new(data);
        let result = read_resp_bulk_string(&mut cursor).unwrap();
        assert_eq!(result, b"hello");

        let result2 = read_resp_bulk_string(&mut cursor).unwrap();
        assert_eq!(result2, b"world");
    }

    #[test]
    fn test_read_resp_bulk_string_invalid_header() {
        let data = b"invalid header";
        let mut cursor = Cursor::new(data);
        assert!(read_resp_bulk_string(&mut cursor).is_err());
    }
}
