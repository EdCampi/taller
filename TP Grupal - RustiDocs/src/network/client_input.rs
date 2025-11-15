use super::resp_message::RespMessage;
use crate::command::Instruction;
use crate::logs::aof_logger::AofLogger;
use crate::network::resp_parser::parse_resp_line;
use crate::security::types::ValidationError;
use crate::security::users::permissions::Permissions;
use crate::security::users::user_base::UserBase;
use std::io::{BufReader, Read, Write};
use std::sync::Arc;
use std::sync::mpsc::Sender;

// Trait para streams que pueden leer y escribir
pub trait ClientConnection: Read + Write {}
impl<T: Read + Write> ClientConnection for T {}

pub struct ClientInput {
    client_id: String,
    connection: Box<dyn ClientConnection>,
    instruction_sender: Sender<(String, Instruction, Sender<RespMessage>)>,
    output_sender: Sender<RespMessage>,
    logger: Arc<AofLogger>,
    user_base: Arc<UserBase>,
    is_logged: bool,
    permission: Permissions,
}

impl ClientInput {
    pub fn new(
        client_id: String,
        instruction_sender: Sender<(String, Instruction, Sender<RespMessage>)>,
        connection: Box<dyn ClientConnection>,
        output_sender: Sender<RespMessage>,
        logger: Arc<AofLogger>,
        user_base: Arc<UserBase>,
    ) -> Self {
        Self {
            client_id,
            instruction_sender,
            connection,
            output_sender,
            logger,
            user_base,
            is_logged: false,
            permission: Permissions::new(),
        }
    }

    pub fn run(&mut self) {
        let mut reader = BufReader::new(self.connection.as_mut());
        // self.output_sender.send(RespMessage::SimpleString("Debes iniciar sesion con AUTH user password".to_string()));  TODO: Ver si era la que daba problemas

        loop {
            // Llama a resp_parser para parsear el mensaje -> devuelve RespMessage
            let parsed = match parse_resp_line(&mut reader) {
                Ok(msg) => msg,
                Err(_) => {
                    self.logger
                        .log_notice(format!("Client {} disconnected", self.client_id));
                    eprintln!(
                        "Error al parsear el mensaje RESP o conexión cerrada de {}.",
                        self.client_id
                    );
                    break;
                }
            };

            // Llama a try_from para convertir RespMessage en instruccioón -> devuelve Instruction
            let instruction = match Instruction::try_from(parsed) {
                Ok(inst) => {
                    self.logger.log_debug(format!(
                        "Client {} issued {} with {:?}",
                        self.client_id, inst.instruction_type, inst.arguments
                    ));
                    inst
                }
                Err(e) => {
                    eprintln!("Error al convertir RespMessage a Instruction: {}", e);
                    let error_response = RespMessage::Error(format!("Error: {}", e));
                    if let Err(e) = self.output_sender.send(error_response) {
                        eprintln!("Error al enviar la respuesta de error al cliente: {}", e);
                        break;
                    }
                    continue;
                }
            };

            if instruction.instruction_type == "DISCONNECT" {
                if let Err(e) = self.output_sender.send(RespMessage::Disconnect) {
                    eprintln!("Error al enviar mensaje de desconexión: {}", e);
                }

                break; // Terminar ejecución
            }

            if self.is_logged {
                if self.permission.is_permited(&instruction.instruction_type) {
                    // Enviar la instruccion y el canal de respeusta al command executor
                    if let Err(e) = self.instruction_sender.send((
                        self.client_id.clone(),
                        instruction,
                        self.output_sender.clone(),
                    )) {
                        eprintln!("Error al enviar la instrucción al ejecutor: {}", e);
                        break;
                    }
                } else {
                    eprintln!("La instruccion no esta permitida para el usuario");
                    self.output_sender
                        .send(RespMessage::SimpleString(
                            "La instruccion no esta permitida para el usuario".to_string(),
                        ))
                        .unwrap();
                }
            } else {
                if instruction.instruction_type == "AUTH" {
                    match self
                        .user_base
                        .validate_user(&instruction.arguments[0], &instruction.arguments[1])
                    {
                        Ok(permissions) => {
                            self.permission = permissions;
                            self.is_logged = true;
                            self.logger.log_event(format!(
                                "Nuevo usuario {} conectado desde {}",
                                &instruction.arguments[0], self.client_id
                            ));
                            if self.permission.is_read_only() {
                                self.output_sender
                                    .send(RespMessage::SimpleString(
                                        "Usuario logeado correctamente - READ".to_string(),
                                    ))
                                    .unwrap();
                            } else {
                                self.output_sender
                                    .send(RespMessage::SimpleString(
                                        "Usuario logeado correctamente - WRITE".to_string(),
                                    ))
                                    .unwrap();
                            }
                        }
                        Err(ValidationError::IncorrectPassword) => {
                            println!("Contraseña incorrecta");
                            self.output_sender
                                .send(RespMessage::Error(
                                    "La contraseña ingresada es incorrecta".to_string(),
                                ))
                                .unwrap();
                        }
                        Err(ValidationError::UserNotFound) => {
                            println!("El usuario ingresado no existe");
                            self.output_sender
                                .send(RespMessage::Error(
                                    "El usuario ingresado no existe".to_string(),
                                ))
                                .unwrap();
                        }
                    }
                } else {
                    println!("Usuario no logeado trata de enviar instruccion");
                    self.output_sender
                        .send(RespMessage::Error(
                            "Debes iniciar sesion con AUTH user password".to_string(),
                        ))
                        .unwrap();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::node_configs::NodeConfigs;
    use crate::network::resp_message::RespMessage;
    use crate::security::users::user::User;
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    fn setup_listener_and_client(port: u16) -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();
        let addr = listener.local_addr().unwrap();
        let client = TcpStream::connect(addr).unwrap();
        let (server, _) = listener.accept().unwrap();
        (client, server)
    }

    #[test]
    fn test_client_input_ping() {
        let (mut client, server_socket) = setup_listener_and_client(12343);
        let (instruction_tx, instruction_rx) = mpsc::channel();
        let (output_tx, output_rx) = mpsc::channel();
        // Mensaje PING como RESP: *1\r\n$4\r\nPING\r\n

        let settings = NodeConfigs::new(&"./tests/utils/test_c_i_1.conf".to_string()).unwrap();
        let logger = AofLogger::new(settings);

        let mut permissions = Permissions::new();
        permissions.set_super();
        let user = User::new("user".to_string(), "pass".to_string(), permissions);
        let mut user_base = UserBase::new();
        user_base.add_user(user);

        // Hilo que corre el ClientInput
        let _ = thread::spawn(move || {
            let mut client_input = ClientInput::new(
                "AA000".to_string(),
                instruction_tx,
                Box::new(server_socket),
                output_tx,
                logger,
                Arc::new(user_base),
            );
            client_input.run();
        });
        let auth = b"*3\r\n$4\r\nAUTH\r\n$4\r\nuser\r\n$4\r\npass\r\n";
        client.write_all(auth).unwrap();
        client.flush().unwrap();
        let _ = output_rx.recv_timeout(Duration::from_secs(1)).unwrap();

        let ping_command = b"*1\r\n$4\r\nPING\r\n";
        client.write_all(ping_command).unwrap();
        client.flush().unwrap();

        // Ejecutar el mock del command_executor
        let (_, instr, responder) = instruction_rx.recv().unwrap();
        assert_eq!(instr.instruction_type, "PING");

        responder
            .send(RespMessage::SimpleString("PONG".into()))
            .unwrap();

        let response = output_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        match response {
            RespMessage::SimpleString(s) => assert_eq!(s, "PONG"),
            _ => panic!("Respuesta incorrecta"),
        }
    }

    #[test]
    fn test_client_input_disconnect() {
        use std::time::Duration;

        let (mut client, server_socket) = setup_listener_and_client(12342);

        let (instruction_tx, instruction_rx) = mpsc::channel();
        let (output_tx, output_rx) = mpsc::channel();

        // Comando RESP para DISCONNECT: *1\r\n$10\r\nDISCONNECT\r\n
        let settings = NodeConfigs::new(&"./tests/utils/test_c_i_2.conf".to_string()).unwrap();
        let logger = AofLogger::new(settings);

        let mut permissions = Permissions::new();
        permissions.set_super();
        let user = User::new("user".to_string(), "pass".to_string(), permissions);
        let mut user_base = UserBase::new();
        user_base.add_user(user);

        // Hilo que corre el ClientInput
        let _ = thread::spawn(move || {
            let mut client_input = ClientInput::new(
                "AA000".to_string(),
                instruction_tx,
                Box::new(server_socket),
                output_tx,
                logger,
                Arc::new(user_base),
            );
            client_input.run();
        });

        let auth = b"*3\r\n$4\r\nAUTH\r\n$4\r\nuser\r\n$4\r\npass\r\n";
        client.write_all(auth).unwrap();
        client.flush().unwrap();
        let _ = output_rx.recv_timeout(Duration::from_secs(1)).unwrap();

        let disconnect_command = b"*1\r\n$10\r\nDISCONNECT\r\n";
        client.write_all(disconnect_command).unwrap();
        client.flush().unwrap();
        // Verificar que NO haya llegado Instruction::Disconnect al command_executor
        assert!(instruction_rx.recv_timeout(Duration::from_secs(1)).is_err());

        // Verificar que se haya enviado RespMessage::Disconnect al cliente a través del output
        let received_response = output_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(received_response, RespMessage::Disconnect);

        // Verificar que no haya más interacciones (sin más respuestas al cliente)
        assert!(output_rx.recv_timeout(Duration::from_millis(500)).is_err());
    }
}
