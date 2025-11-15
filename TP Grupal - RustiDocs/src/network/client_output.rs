//! Módulo de salida de cliente para envío de respuestas RESP
//!
//! Este módulo maneja el envío de respuestas a clientes conectados,
//! convirtiendo mensajes RESP internos a bytes y enviándolos por TCP.
//!
//! # Características
//! - Envío de respuestas RESP a clientes
//! - Manejo de desconexiones
//! - Comunicación asíncrona con canales
//! - Manejo robusto de errores de I/O

use super::resp_message::*;
use std::fmt;
use std::io::{Error as IoError, Write};
use std::sync::mpsc::{Receiver, SendError, Sender};

// Trait para streams que pueden escribir
pub trait ClientOutputStream: Write {}
impl<T: Write> ClientOutputStream for T {}

/// Error que puede ocurrir durante el procesamiento de salida del cliente.
#[derive(Debug, Clone, PartialEq)]
pub enum ClientOutputError {
    /// Error de entrada/salida al escribir al socket
    IoError(String),
    /// Error al enviar señal de desconexión
    DisconnectSendError(String),
    /// Cliente desconectado
    ClientDisconnected(String),
}

impl fmt::Display for ClientOutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientOutputError::IoError(msg) => write!(f, "Error de I/O: {}", msg),
            ClientOutputError::DisconnectSendError(msg) => {
                write!(f, "Error al enviar desconexión: {}", msg)
            }
            ClientOutputError::ClientDisconnected(id) => write!(f, "Cliente desconectado: {}", id),
        }
    }
}

impl std::error::Error for ClientOutputError {}

impl From<IoError> for ClientOutputError {
    fn from(err: IoError) -> Self {
        ClientOutputError::IoError(err.to_string())
    }
}

impl From<SendError<String>> for ClientOutputError {
    fn from(err: SendError<String>) -> Self {
        ClientOutputError::DisconnectSendError(err.to_string())
    }
}

/// Estructura que maneja la salida de mensajes a un cliente específico.
///
/// Esta estructura procesa mensajes RESP desde un canal y los envía
/// al cliente a través de una conexión TCP o TLS.
pub struct ClientOutput {
    /// ID único del cliente
    client_id: String,
    /// Conexión con el cliente (TCP o TLS)
    client_socket: Box<dyn ClientOutputStream>,
    /// Canal para recibir respuestas
    responses: Receiver<RespMessage>,
    /// Canal para enviar señales de desconexión
    disconnect_sender: Sender<String>,
    message_queue: Vec<RespMessage>,
}

impl ClientOutput {
    /// Crea una nueva instancia de ClientOutput.
    ///
    /// # Arguments
    ///
    /// * `client_id` - ID único del cliente
    /// * `client_socket` - Conexión con el cliente (TCP o TLS)
    /// * `responses` - Canal para recibir respuestas
    /// * `disconnect_sender` - Canal para enviar señales de desconexión
    ///
    /// # Returns
    ///
    /// Nueva instancia de ClientOutput
    pub fn new(
        client_id: String,
        client_socket: Box<dyn ClientOutputStream>,
        responses: Receiver<RespMessage>,
        disconnect_sender: Sender<String>,
    ) -> Self {
        Self {
            client_id,
            client_socket,
            responses,
            disconnect_sender,
            message_queue: Vec::new(),
        }
    }

    /// Ejecuta el bucle principal de envío de respuestas.
    ///
    /// Este método procesa mensajes desde el canal de respuestas y los envía
    /// al cliente. Cuando recibe un mensaje de desconexión, envía la señal
    /// correspondiente y termina la ejecución.
    ///
    /// # Returns
    ///
    /// `Result<(), ClientOutputError>` - Resultado de la ejecución
    pub fn run(&mut self) -> Result<(), ClientOutputError> {
        while let Ok(response) = self.responses.recv() {
            match response {
                RespMessage::Disconnect => {
                    self.handle_disconnect()?;
                    break;
                }
                _ => {
                    self.send_response(&response)?;
                }
            }
        }
        Ok(())
    }

    /// Maneja la desconexión del cliente.
    ///
    /// Envía un mensaje de confirmación al cliente y notifica
    /// al sistema sobre la desconexión.
    ///
    /// # Returns
    ///
    /// `Result<(), ClientOutputError>` - Resultado de la operación
    fn handle_disconnect(&mut self) -> Result<(), ClientOutputError> {
        /*let res = format!("+{}\r\n", self.client_id);
        let _ = self.client_socket.write_all(res.as_bytes());
        let _ = self.disconnect_sender.send(SupervisorInstruction::Terminate(self.client_id.to_string()));*/
        let client_id = self.client_id.clone();
        let sender = self.disconnect_sender.clone();
        let disconnect_msg = b"+Desconectado con exito\r\n";
        self.client_socket.write_all(disconnect_msg)?;
        self.client_socket.flush()?;
        sender.send(client_id)?;
        Ok(())
    }

    /// Envía una respuesta al cliente.
    ///
    /// # Arguments
    ///
    /// * `response` - Mensaje RESP a enviar
    ///
    /// # Returns
    ///
    /// `Result<(), ClientOutputError>` - Resultado de la operación
    fn send_response(&mut self, response: &RespMessage) -> Result<(), ClientOutputError> {
        println!("Sending response: {:?}", response);
        self.message_queue.push(response.clone());

        while let Some(msg) = self.message_queue.pop() {
            let bytes = msg.as_bytes();
            self.client_socket.write_all(&bytes)?;
            self.client_socket.flush()?;
        }

        Ok(())
    }

    /// Obtiene el ID del cliente.
    ///
    /// # Returns
    ///
    /// Referencia al ID del cliente
    pub fn get_client_id(&self) -> &str {
        &self.client_id
    }

    /// Verifica si la conexión está activa.
    ///
    /// # Returns
    ///
    /// `true` si la conexión está activa, `false` en caso contrario
    pub fn is_connection_active(&self) -> bool {
        // Para streams genéricos, asumimos que están activos si no hay errores de escritura
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    /// Configura un listener TCP y un cliente conectado para testing.
    ///
    /// # Returns
    ///
    /// Tupla con (cliente_stream, servidor_stream)
    fn setup_listener_and_client() -> Result<(TcpStream, TcpStream), ClientOutputError> {
        let listener = TcpListener::bind("0.0.0.0:0")
            .map_err(|e| ClientOutputError::IoError(e.to_string()))?;
        let addr = listener
            .local_addr()
            .map_err(|e| ClientOutputError::IoError(e.to_string()))?;
        let client =
            TcpStream::connect(addr).map_err(|e| ClientOutputError::IoError(e.to_string()))?;
        let (server, _) = listener
            .accept()
            .map_err(|e| ClientOutputError::IoError(e.to_string()))?;
        Ok((client, server))
    }

    #[test]
    fn test_client_output_new() {
        let (_client, server) = setup_listener_and_client().unwrap();
        let (_tx, rx) = mpsc::channel();
        let (disconnect_tx, _) = mpsc::channel();

        let client_output = ClientOutput::new(
            "test_client".to_string(),
            Box::new(server),
            rx,
            disconnect_tx,
        );

        assert_eq!(client_output.get_client_id(), "test_client");
        assert!(client_output.is_connection_active());
    }

    #[test]
    fn test_client_output_envia_respuesta() {
        let (mut client, server) = setup_listener_and_client().unwrap();
        let (tx, rx) = mpsc::channel();
        let (disconnect_tx, _) = mpsc::channel();
        let mensaje = RespMessage::SimpleString("Hola!".to_string());

        thread::spawn(move || {
            let mut client_output =
                ClientOutput::new("AAA000".to_string(), Box::new(server), rx, disconnect_tx);
            let _ = client_output.run();
        });

        tx.send(mensaje).unwrap();

        let mut buf = [0; 128];
        let n = client.read(&mut buf).unwrap();
        let recibido = std::str::from_utf8(&buf[..n]).unwrap();
        assert_eq!(recibido, "+Hola!\r\n");
    }

    #[test]
    fn test_client_output_desconecta_correctamente() {
        let (mut client, server) = setup_listener_and_client().unwrap();
        let (tx, rx) = mpsc::channel();
        let (disconnect_tx, disconnect_rx) = mpsc::channel();
        let client_id = "ABC123";

        thread::spawn(move || {
            let mut client_output =
                ClientOutput::new(client_id.to_string(), Box::new(server), rx, disconnect_tx);
            let _ = client_output.run();
        });

        tx.send(RespMessage::Disconnect).unwrap();

        let mut buf = [0; 128];
        let n = client.read(&mut buf).unwrap();
        let recibido = std::str::from_utf8(&buf[..n]).unwrap();

        assert_eq!(recibido, "+Desconectado con exito\r\n");
        let desconectado = disconnect_rx.recv().unwrap();
        assert_eq!(desconectado, client_id);
    }

    #[test]
    fn test_client_output_error_al_escribir_no_envia_disconnect() {
        let (client, server) = setup_listener_and_client().unwrap();
        let (tx, rx) = mpsc::channel::<RespMessage>();
        let (disconnect_tx, disconnect_rx) = mpsc::channel::<String>();

        // Paso 5: crear un id para el cliente
        let client_id = "ERR001".to_string();

        drop(client);

        let handle = thread::spawn(move || {
            let mut client_output =
                ClientOutput::new(client_id.clone(), Box::new(server), rx, disconnect_tx);
            let _ = client_output.run();
        });

        tx.send(RespMessage::SimpleString("Probando".to_string()))
            .unwrap();
        drop(tx);

        handle.join().unwrap();

        let disconnect_result = disconnect_rx.recv_timeout(Duration::from_millis(200));
        assert!(
            disconnect_result.is_err(),
            "No debería haberse enviado Disconnect"
        );
    }

    #[test]
    fn test_client_output_error_display() {
        let error = ClientOutputError::IoError("test error".to_string());
        assert_eq!(error.to_string(), "Error de I/O: test error");
    }

    #[test]
    fn test_client_output_error_from_io_error() {
        let io_error = IoError::new(std::io::ErrorKind::ConnectionRefused, "connection refused");
        let client_error: ClientOutputError = io_error.into();
        match client_error {
            ClientOutputError::IoError(_) => (),
            _ => panic!("Se esperaba error IoError"),
        }
    }

    #[test]
    fn test_client_output_error_from_send_error() {
        let (tx, _) = mpsc::channel::<String>();
        let tx_clone = tx.clone();
        drop(tx);

        let send_result = tx_clone.send("test".to_string());
        if let Err(send_error) = send_result {
            let client_error: ClientOutputError = send_error.into();
            match client_error {
                ClientOutputError::DisconnectSendError(_) => (),
                _ => panic!("Se esperaba error DisconnectSendError"),
            }
        } else {
            panic!("Se esperaba error de envío");
        }
    }

    #[test]
    fn test_client_output_send_response() {
        let (_, server) = setup_listener_and_client().unwrap();
        let (_tx, rx) = mpsc::channel();
        let (disconnect_tx, _) = mpsc::channel();

        let mut client_output = ClientOutput::new(
            "test_client".to_string(),
            Box::new(server),
            rx,
            disconnect_tx,
        );

        let response = RespMessage::SimpleString("test".to_string());
        let result = client_output.send_response(&response);
        assert!(result.is_ok());
    }

    #[test]
    fn test_client_output_handle_disconnect() {
        let (_, server) = setup_listener_and_client().unwrap();
        let (_tx, rx) = mpsc::channel();
        let (disconnect_tx, disconnect_rx) = mpsc::channel();

        let mut client_output = ClientOutput::new(
            "test_client".to_string(),
            Box::new(server),
            rx,
            disconnect_tx,
        );

        let result = client_output.handle_disconnect();
        assert!(result.is_ok());

        let disconnected_id = disconnect_rx.recv().unwrap();
        println!("Disconnected id: {:?}", disconnected_id); // -> b"+test_client\r\n"
        // TODO: assert_eq!(disconnected_id, "test_client");
    }
}
