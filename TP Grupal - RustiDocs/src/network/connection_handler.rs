//! Módulo de manejo de conexiones de clientes
//!
//! Este módulo gestiona las conexiones TCP entrantes de clientes,
//! creando hilos separados para entrada y salida de cada cliente.
//!
//! # Características
//! - Gestión de múltiples conexiones concurrentes
//! - Generación automática de ids únicos para clientes
//! - Manejo de desconexiones y limpieza de recursos
//! - Comunicación asíncrona con el ejecutor de comandos

use std::{
    fmt,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, Sender, channel},
    },
    thread::{self, JoinHandle},
};

use super::{client_input::ClientInput, client_output::ClientOutput};

use crate::{
    command::Instruction,
    config::node_configs::NodeConfigs,
    logs::aof_logger::AofLogger,
    network::RespMessage,
    security::{
        tls_lite::{TlsServerConfig, TlsServerStream},
        users::user_base::UserBase,
    },
};

/// Enum para manejar diferentes tipos de streams
#[derive(Debug)]
enum ClientStream {
    Tcp(TcpStream),
    Tls(TlsServerStream),
}

impl Read for ClientStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            ClientStream::Tcp(stream) => stream.read(buf),
            ClientStream::Tls(stream) => stream.read(buf),
        }
    }
}

impl Write for ClientStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            ClientStream::Tcp(stream) => stream.write(buf),
            ClientStream::Tls(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            ClientStream::Tcp(stream) => stream.flush(),
            ClientStream::Tls(stream) => stream.flush(),
        }
    }
}

impl ClientStream {
    fn try_clone(&self) -> std::io::Result<ClientStream> {
        match self {
            // TODO: Posible problema a lvl cifrado cliente-user
            ClientStream::Tcp(stream) => stream.try_clone().map(ClientStream::Tcp),
            ClientStream::Tls(_) => {
                // TlsServerStream no implementa try_clone, así que devolvemos un error
                Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "TLS streams cannot be cloned",
                ))
            }
        }
    }
}

/// Error que puede ocurrir durante el manejo de conexiones.
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionHandlerError {
    /// Error al bindear el listener TCP
    BindError(String),
    /// Error al aceptar conexiones
    AcceptError(String),
    /// Error al clonar el stream TCP
    StreamCloneError(String),
    /// Error al enviar instrucciones
    InstructionSendError(String),
    /// Error al enviar señal de desconexión
    DisconnectSendError(String),
    /// Error al unir hilos
    JoinError(String),
    /// Error de lock en mutex
    LockError(String),
    /// Error en handshake TLS
    TlsError(String),
}

impl fmt::Display for ConnectionHandlerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionHandlerError::BindError(msg) => write!(f, "Error al bindear: {}", msg),
            ConnectionHandlerError::AcceptError(msg) => {
                write!(f, "Error al aceptar conexión: {}", msg)
            }
            ConnectionHandlerError::StreamCloneError(msg) => {
                write!(f, "Error al clonar stream: {}", msg)
            }
            ConnectionHandlerError::InstructionSendError(msg) => {
                write!(f, "Error al enviar instrucción: {}", msg)
            }
            ConnectionHandlerError::DisconnectSendError(msg) => {
                write!(f, "Error al enviar desconexión: {}", msg)
            }
            ConnectionHandlerError::JoinError(msg) => write!(f, "Error al unir hilos: {}", msg),
            ConnectionHandlerError::LockError(msg) => write!(f, "Error de lock: {}", msg),
            ConnectionHandlerError::TlsError(msg) => write!(f, "Error en handshake TLS: {}", msg),
        }
    }
}

impl std::error::Error for ConnectionHandlerError {}

/// Estructura que maneja las conexiones de clientes al servidor.
///
/// Esta estructura gestiona múltiples conexiones TCP concurrentes,
/// creando hilos separados para cada cliente y manteniendo un registro
/// de todas las conexiones activas.
pub struct Handler {
    /// Id del próximo cliente a conectar
    next_id: String,
    /// Lista de conexiones activas (id, input_handle, output_handle)
    connections: Vec<(String, JoinHandle<()>, JoinHandle<()>)>,
    /// Canal para enviar instrucciones al ejecutor de comandos
    instruction_sender: Sender<(String, Instruction, Sender<RespMessage>)>,
    /// Canal para enviar señales de desconexión
    disconnect_sender: Sender<String>,
    /// Canal para recibir señales de desconexión
    disconnect_receiver: Receiver<String>,
    /// Configuración del nodo
    configs: NodeConfigs,
    /// Logger para eventos del servidor
    logger: Arc<AofLogger>,
    user_base: Arc<UserBase>,
}

impl Handler {
    /// Crea una nueva instancia del controlador de conexiones.
    ///
    /// # Arguments
    ///
    /// * `instruction_sender` - Canal para enviar instrucciones al ejecutor
    /// * `configs` - Configuración del nodo
    /// * `logger` - Logger para eventos del servidor
    ///
    /// # Returns
    ///
    /// Nueva instancia de Handler
    pub fn new(
        instruction_sender: Sender<(String, Instruction, Sender<RespMessage>)>,
        configs: NodeConfigs,
        logger: Arc<AofLogger>,
        user_base: UserBase,
    ) -> Self {
        let (disconnect_sender, disconnect_receiver) = channel();
        /*let mut supervisor = Supervisor::new(disconnect_receiver);
        let supervisor_thread = thread::spawn( move ||{
            supervisor.init();
        });*/

        Self {
            next_id: "AAA000".to_string(),
            connections: Vec::new(),
            instruction_sender,
            disconnect_sender,
            disconnect_receiver,
            configs,
            logger,
            user_base: Arc::new(user_base),
        }
    }

    /// Inicializa el controlador de conexiones.
    ///
    /// Este mét-odo inicia el listener TCP y comienza a aceptar conexiones
    /// de clientes. También inicia un hilo para manejar las desconexiones.
    pub fn init(self) -> Result<(), ConnectionHandlerError> {
        let handler_ref = Arc::new(Mutex::new(self));
        let handler_clone = Arc::clone(&handler_ref);

        // Hilo que escucha desconexiones
        thread::spawn(move || {
            loop {
                let client_id_opt = {
                    let handler = handler_clone
                        .lock()
                        .map_err(|e| ConnectionHandlerError::LockError(e.to_string()))?;
                    handler.disconnect_receiver.recv().ok()
                };

                if let Some(client_id) = client_id_opt {
                    let mut handler = handler_clone
                        .lock()
                        .map_err(|e| ConnectionHandlerError::LockError(e.to_string()))?;
                    handler.close_connection(client_id)?;
                } else {
                    break; // Se cerró el canal
                }
            }
            Ok::<(), ConnectionHandlerError>(())
        });

        // Continuar la ejecución principal
        let mut handler = handler_ref
            .lock()
            .map_err(|e| ConnectionHandlerError::LockError(e.to_string()))?;
        handler.receive_connection()
    }

    /// Inicializa la conexión de un nuevo cliente al servidor.
    ///
    /// Este mét-odo bindea un listener TCP y comienza a aceptar conexiones
    /// de clientes, creando hilos separados para cada uno.
    ///
    /// # Returns
    ///
    /// `Result<(), ConnectionHandlerError>` - Resultado de la operación
    fn receive_connection(&mut self) -> Result<(), ConnectionHandlerError> {
        let addr = self.configs.get_addr();
        let listener = TcpListener::bind(addr)
            .map_err(|e| ConnectionHandlerError::BindError(e.to_string()))?;

        self.logger
            .log_notice(format!("Server listening on {}", self.configs.get_addr()));

        loop {
            let (client_stream, socket_addr) = listener
                .accept()
                .map_err(|e| ConnectionHandlerError::AcceptError(e.to_string()))?;

            self.logger.log_event(format!(
                "Accepted {}:{} connected, ID {}",
                socket_addr.ip(),
                socket_addr.port(),
                self.next_id,
            ));

            self.handle_new_connection(client_stream)?;
        }
    }

    /// Maneja una nueva conexión de cliente.
    ///
    /// Detecta automáticamente si la conexión es TLS o TCP normal.
    /// Crea hilos separados para entrada y salida del cliente.
    ///
    /// # Arguments
    ///
    /// * `client_stream` - Stream TCP del cliente
    ///
    /// # Returns
    ///
    /// `Result<(), ConnectionHandlerError>` - Resultado de la operación
    fn handle_new_connection(
        &mut self,
        client_stream: TcpStream,
    ) -> Result<(), ConnectionHandlerError> {
        // Detectar si la conexión es TLS o TCP normal
        let client_stream = self.detect_and_establish_connection(client_stream)?;

        let (output_sender, output_receiver) = channel();

        // Intentar clonar el stream para input y output
        let client_stream_clone = match client_stream.try_clone() {
            Ok(clone) => clone,
            Err(_) => {
                // Si no se puede clonar (TLS), usar el stream original solo para input
                self.handle_tls_connection(client_stream, output_sender)?;
                return Ok(());
            }
        };

        let instruction_sender_clone = self.instruction_sender.clone();
        let client_id = self.next_id.clone();
        let client_logger = self.logger.clone();
        let clone_user_base = self.user_base.clone();

        let input = create_client_input_thread(
            client_id,
            instruction_sender_clone,
            client_stream_clone,
            output_sender,
            client_logger,
            clone_user_base,
        );

        let client_stream_clone = client_stream
            .try_clone()
            .map_err(|e| ConnectionHandlerError::StreamCloneError(e.to_string()))?;
        let disconnect_sender_clone = self.disconnect_sender.clone();
        let client_id = self.next_id.clone();
        self.update_id();

        let output = thread::spawn(move || {
            let mut client = ClientOutput::new(
                client_id,
                Box::new(client_stream_clone),
                output_receiver,
                disconnect_sender_clone,
            );
            let _ = client.run();
        });

        let client_id = self.next_id.clone();
        self.connections.push((client_id, input, output));
        // TODO: Revisar si se queda self.disconnect_sender_si.send(SupervisorInstruction::Add(self.next_id.clone(), (input, output))).unwrap();
        Ok(())
    }

    /// Detecta si la conexión entrante es TLS o TCP normal
    fn detect_and_establish_connection(
        &self,
        mut tcp_stream: TcpStream,
    ) -> Result<ClientStream, ConnectionHandlerError> {
        // Configurar timeout para no bloquear indefinidamente
        tcp_stream
            .set_read_timeout(Some(std::time::Duration::from_millis(100)))
            .map_err(|e| {
                ConnectionHandlerError::TlsError(format!("Error configurando timeout: {}", e))
            })?;

        // Intentar leer el primer byte para detectar TLS
        let mut peek_buffer = [0u8; 1];
        match tcp_stream.read(&mut peek_buffer) {
            Ok(1) => {
                // Si el primer byte es 0x16, es un handshake TLS
                if peek_buffer[0] == 0x16 {
                    self.logger.log_notice(
                        "Detectada conexión TLS, estableciendo handshake...".to_string(),
                    );

                    // Restaurar timeout normal
                    tcp_stream.set_read_timeout(None).map_err(|e| {
                        ConnectionHandlerError::TlsError(format!(
                            "Error restaurando timeout: {}",
                            e
                        ))
                    })?;

                    // Establecer conexión TLS
                    let server_config = TlsServerConfig::new();
                    let tls_stream =
                        TlsServerStream::new(tcp_stream, server_config).map_err(|e| {
                            ConnectionHandlerError::TlsError(format!(
                                "Error en handshake TLS: {}",
                                e
                            ))
                        })?;

                    self.logger.log_notice("Handshake TLS exitoso".to_string());
                    Ok(ClientStream::Tls(tls_stream))
                } else {
                    // Es una conexión TCP normal
                    self.logger
                        .log_notice("Detectada conexión TCP normal".to_string());

                    // Restaurar timeout normal
                    tcp_stream.set_read_timeout(None).map_err(|e| {
                        ConnectionHandlerError::TlsError(format!(
                            "Error restaurando timeout: {}",
                            e
                        ))
                    })?;

                    Ok(ClientStream::Tcp(tcp_stream))
                }
            }
            _ => {
                // Si no se puede leer, asumir que es TCP normal
                self.logger
                    .log_notice("Asumiendo conexión TCP normal".to_string());
                // Restaurar timeout normal
                tcp_stream.set_read_timeout(None).map_err(|e| {
                    ConnectionHandlerError::TlsError(format!("Error restaurando timeout: {}", e))
                })?;
                Ok(ClientStream::Tcp(tcp_stream))
            }
        }
    }

    /// Maneja conexiones TLS que no se pueden clonar
    fn handle_tls_connection(
        &mut self,
        client_stream: ClientStream,
        output_sender: Sender<RespMessage>,
    ) -> Result<(), ConnectionHandlerError> {
        let instruction_sender_clone = self.instruction_sender.clone();
        let client_id = self.next_id.clone();
        let client_logger = self.logger.clone();
        let user_base = self.user_base.clone();

        let input = create_client_input_thread(
            client_id,
            instruction_sender_clone,
            client_stream,
            output_sender,
            client_logger,
            user_base,
        );

        let client_id = self.next_id.clone();
        self.update_id();

        // Para TLS, solo tenemos un thread (input), ya que no se puede clonar
        self.connections
            .push((client_id, input, thread::spawn(|| {})));
        Ok(())
    }

    /// Cierra la conexión de un cliente al servidor.
    ///
    /// # Arguments
    ///
    /// * `client_id` - ID del cliente a desconectar
    ///
    /// # Returns
    ///
    /// `Result<(), ConnectionHandlerError>` - Resultado de la operación
    fn close_connection(&mut self, client_id: String) -> Result<(), ConnectionHandlerError> {
        for i in 0..self.connections.len() {
            let (id, _, _) = &self.connections[i];
            if *id == client_id {
                let (_id, input_handle, output_handle) = self.connections.remove(i);

                input_handle.join().map_err(|e| {
                    ConnectionHandlerError::JoinError(format!("Input thread: {:?}", e))
                })?;
                output_handle.join().map_err(|e| {
                    ConnectionHandlerError::JoinError(format!("Output thread: {:?}", e))
                })?;
                break;
            }
        }
        Ok(())
    }

    /// Obtiene el número de conexiones activas.
    ///
    /// # Returns
    ///
    /// Número de conexiones activas
    pub fn get_connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Obtiene el ID del próximo cliente.
    ///
    /// # Returns
    ///
    /// Id del próximo cliente
    pub fn get_next_id(&self) -> &str {
        &self.next_id
    }

    /// Función para generar un nuevo ID.
    ///
    /// Incrementa el ID actual siguiendo el patrón AAA000, AAA001, ..., AAA999, AAB000, etc.
    fn update_id(&mut self) {
        let (letters, number) = self.split_id(&self.next_id);

        let new_number = if number == 999 { 0 } else { number + 1 };

        let new_letters = if new_number == 0 {
            self.increment_letters(letters)
        } else {
            letters
        };

        self.next_id = format!("{}{:03}", new_letters, new_number);
    }

    /// Divide el ID en letras y número.
    ///
    /// # Arguments
    ///
    /// * `id` - ID a dividir (formato: AAA000)
    ///
    /// # Returns
    ///
    /// Tupla con (letras, número)
    fn split_id(&self, id: &str) -> (String, usize) {
        let letters = &id[..3]; // Primeros 3 caracteres
        let number = id[3..].parse::<usize>().unwrap(); // Últimos 3 caracteres convertidos a número

        (letters.to_string(), number)
    }

    /// Incrementa las letras del ID.
    ///
    /// # Arguments
    ///
    /// * `letters` - Letras actuales (3 caracteres)
    ///
    /// # Returns
    ///
    /// Nuevas letras incrementadas
    fn increment_letters(&self, letters: String) -> String {
        let mut carry = true;
        let mut new_letters = letters.chars().collect::<Vec<_>>();

        for i in (0..3).rev() {
            if carry {
                if new_letters[i] == 'Z' {
                    new_letters[i] = 'A';
                } else {
                    new_letters[i] = (new_letters[i] as u8 + 1) as char;
                    carry = false;
                }
            }
        }

        new_letters.iter().collect::<String>()
    }
}

fn create_client_input_thread(
    client_id: String,
    instruction_sender: Sender<(String, Instruction, Sender<RespMessage>)>,
    client_stream: ClientStream,
    output_sender: Sender<RespMessage>,
    client_logger: Arc<AofLogger>,
    clone_user: Arc<UserBase>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut client = ClientInput::new(
            client_id,
            instruction_sender,
            Box::new(client_stream),
            output_sender,
            client_logger,
            clone_user,
        );
        let _ = client.run();
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::node_configs::NodeConfigs;
    use crate::logs::aof_logger::AofLogger;
    use crate::network::connection_handler::Handler;
    use crate::security::users::user_base::UserBase;

    /// Crea un handler de prueba con configuración básica.
    fn create_test_handler() -> Handler {
        let (instruction_tx, _) = channel();
        let settings = NodeConfigs::new("./tests/utils/redis.conf").unwrap();
        let logger = AofLogger::new(settings.clone());
        let user_base = UserBase::new();

        Handler::new(instruction_tx, settings, logger, user_base)
    }

    #[test]
    fn test_handler_new() {
        let handler = create_test_handler();
        assert_eq!(handler.get_next_id(), "AAA000");
        assert_eq!(handler.get_connection_count(), 0);
    }

    #[test]
    fn test_handler_update_id() {
        let mut handler = create_test_handler();

        // Probar incremento normal
        handler.update_id();
        assert_eq!(handler.get_next_id(), "AAA001");

        // Probar incremento con carry
        handler.next_id = "AAA999".to_string();
        handler.update_id();
        assert_eq!(handler.get_next_id(), "AAB000");

        // Probar incremento con carry múltiple
        handler.next_id = "AZZ999".to_string();
        handler.update_id();
        assert_eq!(handler.get_next_id(), "BAA000");
    }

    #[test]
    fn test_handler_split_id() {
        let handler = create_test_handler();

        let (letters, number) = handler.split_id("AAA000");
        assert_eq!(letters, "AAA");
        assert_eq!(number, 0);

        let (letters, number) = handler.split_id("XYZ999");
        assert_eq!(letters, "XYZ");
        assert_eq!(number, 999);
    }

    #[test]
    fn test_handler_increment_letters() {
        let handler = create_test_handler();

        // Incremento simple
        let result = handler.increment_letters("AAA".to_string());
        assert_eq!(result, "AAB");

        // Incremento con carry
        let result = handler.increment_letters("AAZ".to_string());
        assert_eq!(result, "ABA");

        // Incremento con carry múltiple
        let result = handler.increment_letters("AZZ".to_string());
        assert_eq!(result, "BAA");
    }

    #[test]
    fn test_handler_error_display() {
        let error = ConnectionHandlerError::BindError("test error".to_string());
        assert_eq!(error.to_string(), "Error al bindear: test error");
    }

    #[test]
    fn test_handler_error_debug() {
        let error = ConnectionHandlerError::AcceptError("test error".to_string());
        assert_eq!(format!("{:?}", error), "AcceptError(\"test error\")");
    }

    #[test] // TODO: Ver nombre y contenido!!!
    fn test_todo() {
        // Este test requiere un listener real, por lo que se mantiene como estaba,
        // pero se adapta para usar el nuevo manejo de errores
        let handler = create_test_handler();
        assert_eq!(handler.get_connection_count(), 0);
    }
}
