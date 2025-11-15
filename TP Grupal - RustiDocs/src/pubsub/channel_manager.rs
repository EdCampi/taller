use crate::command::types::Command;
use crate::network::resp_message::RespMessage;
use std::collections::HashMap;
use std::fmt;
use std::sync::mpsc::{Receiver, Sender};

/// Error que puede ocurrir durante el manejo de canales Pub/Sub.
#[derive(Debug, Clone, PartialEq)]
pub enum ChannelManagerError {
    /// Error al recibir mensaje del receptor
    ReceiveError(String),
    /// Error al enviar respuesta al cliente
    SendResponseError(String),
    /// Error al enviar mensaje a suscriptor
    SendToSubscriberError(String),
    /// Error al suscribir cliente a canal
    SubscribeError(String),
    /// Error al desuscribir cliente de canal
    UnsubscribeError(String),
    /// Error al publicar mensaje en canal
    PublishError(String),
    /// Error de comando no soportado
    UnsupportedCommandError(String),
    /// Error de canal no encontrado
    ChannelNotFoundError(String),
    /// Error de cliente no encontrado
    ClientNotFoundError(String),
}

impl fmt::Display for ChannelManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChannelManagerError::ReceiveError(msg) => {
                write!(f, "Error al recibir mensaje: {}", msg)
            }
            ChannelManagerError::SendResponseError(msg) => {
                write!(f, "Error al enviar respuesta: {}", msg)
            }
            ChannelManagerError::SendToSubscriberError(msg) => {
                write!(f, "Error al enviar mensaje a suscriptor: {}", msg)
            }
            ChannelManagerError::SubscribeError(msg) => {
                write!(f, "Error al suscribir cliente: {}", msg)
            }
            ChannelManagerError::UnsubscribeError(msg) => {
                write!(f, "Error al desuscribir cliente: {}", msg)
            }
            ChannelManagerError::PublishError(msg) => {
                write!(f, "Error al publicar mensaje: {}", msg)
            }
            ChannelManagerError::UnsupportedCommandError(msg) => {
                write!(f, "Comando no soportado: {}", msg)
            }
            ChannelManagerError::ChannelNotFoundError(msg) => {
                write!(f, "Canal no encontrado: {}", msg)
            }
            ChannelManagerError::ClientNotFoundError(msg) => {
                write!(f, "Cliente no encontrado: {}", msg)
            }
        }
    }
}

impl std::error::Error for ChannelManagerError {}

/// Gestor de canales para el sistema Pub/Sub (Publish/Subscribe).
///
/// Maneja la suscripción y desuscripción de clientes a canales,
/// así como la publicación de mensajes a los suscriptores.
///
/// # Comportamiento
///
/// - Si se hace SUBSCRIBE de un canal que no existe, se crea automáticamente
/// - Un cliente puede hacer PUBLISH aunque no esté suscrito a un canal
/// - Si un canal se queda sin suscriptores, se elimina automáticamente
pub struct ChannelManager {
    /// Receptor de mensajes con tuplas (client_id, Command, response_sender, client_sender)
    receiver: Receiver<(String, Command, Sender<String>, Sender<RespMessage>)>,
    /// Mapa de canales: channel_id -> { client_id -> sender al cliente }
    channels: HashMap<String, HashMap<String, Sender<RespMessage>>>,
}

impl ChannelManager {
    /// Crea un nuevo gestor de canales.
    ///
    /// # Arguments
    ///
    /// * `receiver` - Receptor de mensajes para procesar comandos
    ///
    /// # Returns
    ///
    /// `ChannelManager` - Una nueva instancia del gestor de canales
    pub fn new(receiver: Receiver<(String, Command, Sender<String>, Sender<RespMessage>)>) -> Self {
        Self {
            receiver,
            channels: HashMap::new(),
        }
    }

    /// Ejecuta el bucle principal del gestor de canales.
    ///
    /// Procesa mensajes entrantes y maneja comandos de suscripción,
    /// desuscripción y publicación.
    ///
    /// # Returns
    ///
    /// `Result<(), ChannelManagerError>` - Resultado de la ejecución
    pub fn run(&mut self) -> Result<(), ChannelManagerError> {
        while let Ok((client_id, command, response_sender, client_sender)) = self.receiver.recv() {
            self.handle_command(client_id, command, response_sender, client_sender)?;
        }
        Ok(())
    }

    /// Maneja un comando específico.
    ///
    /// # Arguments
    ///
    /// * `client_id` - ID del cliente que envía el comando
    /// * `command` - Comando a procesar
    /// * `response_sender` - Sender para enviar respuesta al cliente
    /// * `client_sender` - Sender para enviar mensajes al cliente
    ///
    /// # Returns
    ///
    /// `Result<(), ChannelManagerError>` - Resultado del procesamiento
    fn handle_command(
        &mut self,
        client_id: String,
        command: Command,
        response_sender: Sender<String>,
        client_sender: Sender<RespMessage>,
    ) -> Result<(), ChannelManagerError> {
        match command {
            Command::Subscribe(channel_id) => {
                self.handle_subscribe(client_id, channel_id, response_sender, client_sender)
            }
            Command::Unsubscribe(channel_id) => {
                self.handle_unsubscribe(client_id, channel_id, response_sender)
            }
            Command::Publish(channel_id, message) => {
                self.handle_publish(channel_id, message, response_sender)
            }
            _ => self.send_response(
                response_sender,
                "Comando no soportado en ChannelManager".to_string(),
            ),
        }
    }

    /// Maneja el comando de suscripción.
    ///
    /// # Arguments
    ///
    /// * `client_id` - ID del cliente que se suscribe
    /// * `channel_id` - ID del canal al que suscribirse
    /// * `response_sender` - Sender para enviar respuesta
    /// * `client_sender` - Sender para enviar mensajes al cliente
    ///
    /// # Returns
    ///
    /// `Result<(), ChannelManagerError>` - Resultado de la suscripción
    fn handle_subscribe(
        &mut self,
        client_id: String,
        channel_id: String,
        response_sender: Sender<String>,
        client_sender: Sender<RespMessage>,
    ) -> Result<(), ChannelManagerError> {
        // Crear el canal si no existe
        self.channels
            .entry(channel_id.clone())
            .or_insert_with(HashMap::new);

        // Verificar si el cliente ya está suscrito
        if self.channels[&channel_id].contains_key(&client_id) {
            return self.send_response(
                response_sender,
                "Ya estás suscripto a ese canal".to_string(),
            );
        }

        // Agregar el cliente al canal
        self.channels
            .get_mut(&channel_id)
            .ok_or_else(|| {
                ChannelManagerError::SubscribeError("No se pudo acceder al canal".to_string())
            })?
            .insert(client_id, client_sender);

        // Enviar confirmación de éxito
        self.send_response(response_sender, "".to_string())
    }

    /// Maneja el comando de desuscripción.
    ///
    /// # Arguments
    ///
    /// * `client_id` - ID del cliente que se desuscribe
    /// * `channel_id` - ID del canal del que desuscribirse
    /// * `response_sender` - Sender para enviar respuesta
    ///
    /// # Returns
    ///
    /// `Result<(), ChannelManagerError>` - Resultado de la desuscripción
    fn handle_unsubscribe(
        &mut self,
        client_id: String,
        channel_id: String,
        response_sender: Sender<String>,
    ) -> Result<(), ChannelManagerError> {
        if let Some(subs) = self.channels.get_mut(&channel_id) {
            if subs.remove(&client_id).is_some() {
                // Si el canal se queda vacío, eliminarlo
                if subs.is_empty() {
                    self.channels.remove(&channel_id);
                }
                self.send_response(response_sender, "".to_string())
            } else {
                self.send_response(
                    response_sender,
                    "No estabas suscripto a ese canal.".to_string(),
                )
            }
        } else {
            self.send_response(response_sender, "El canal no existe".to_string())
        }
    }

    /// Maneja el comando de publicación.
    ///
    /// # Arguments
    ///
    /// * `channel_id` - ID del canal donde publicar
    /// * `message` - Mensaje a publicar
    /// * `response_sender` - Sender para enviar respuesta
    ///
    /// # Returns
    ///
    /// `Result<(), ChannelManagerError>` - Resultado de la publicación
    fn handle_publish(
        &mut self,
        channel_id: String,
        message: RespMessage,
        response_sender: Sender<String>,
    ) -> Result<(), ChannelManagerError> {
        let mut subscriber_count = 0;

        if let Some(subs) = self.channels.get(&channel_id) {
            // Enviar mensaje a todos los suscriptores
            for (_sub_id, sub_sender) in subs {
                if let Err(_) = sub_sender.send(message.clone()) {
                    println!("[CHANNEL-MNG] Error al propagarle pubsub a {}", _sub_id);
                    //return Err(ChannelManagerError::SendToSubscriberError(e.to_string()));
                }
                subscriber_count += 1;
            }
        }
        // Si el canal no existe, subscriber_count será 0

        // Confirmar publicación exitosa con el número de suscriptores
        self.send_response(response_sender, subscriber_count.to_string())
    }

    /// Envía una respuesta al cliente.
    ///
    /// # Arguments
    ///
    /// * `response_sender` - Sender para enviar la respuesta
    /// * `message` - Mensaje de respuesta
    ///
    /// # Returns
    ///
    /// `Result<(), ChannelManagerError>` - Resultado del envío
    fn send_response(
        &self,
        response_sender: Sender<String>,
        message: String,
    ) -> Result<(), ChannelManagerError> {
        response_sender
            .send(message)
            .map_err(|e| ChannelManagerError::SendResponseError(e.to_string()))
    }

    /// Obtiene el número de canales activos.
    ///
    /// # Returns
    ///
    /// `usize` - Número de canales
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Obtiene el número de suscriptores en un canal específico.
    ///
    /// # Arguments
    ///
    /// * `channel_id` - ID del canal
    ///
    /// # Returns
    ///
    /// `Option<usize>` - Número de suscriptores si el canal existe
    pub fn subscriber_count(&self, channel_id: &str) -> Option<usize> {
        self.channels.get(channel_id).map(|subs| subs.len())
    }

    /// Verifica si un cliente está suscrito a un canal.
    ///
    /// # Arguments
    ///
    /// * `channel_id` - ID del canal
    /// * `client_id` - ID del cliente
    ///
    /// # Returns
    ///
    /// `bool` - `true` si el cliente está suscrito al canal
    pub fn is_subscribed(&self, channel_id: &str, client_id: &str) -> bool {
        self.channels
            .get(channel_id)
            .map(|subs| subs.contains_key(client_id))
            .unwrap_or(false)
    }

    /// Obtiene una lista de todos los canales activos.
    ///
    /// # Returns
    ///
    /// `Vec<String>` - Lista de IDs de canales
    pub fn get_channels(&self) -> Vec<String> {
        self.channels.keys().cloned().collect()
    }

    /// Obtiene una lista de suscriptores de un canal específico.
    ///
    /// # Arguments
    ///
    /// * `channel_id` - ID del canal
    ///
    /// # Returns
    ///
    /// `Option<Vec<String>>` - Lista de IDs de clientes si el canal existe
    pub fn get_subscribers(&self, channel_id: &str) -> Option<Vec<String>> {
        self.channels
            .get(channel_id)
            .map(|subs| subs.keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::resp_message::RespMessage;
    use std::sync::mpsc;

    #[test]
    fn test_channel_manager_error_display() {
        let error = ChannelManagerError::SubscribeError("test error".to_string());
        assert_eq!(error.to_string(), "Error al suscribir cliente: test error");
    }

    #[test]
    fn test_channel_manager_error_debug() {
        let error = ChannelManagerError::PublishError("test error".to_string());
        assert_eq!(format!("{:?}", error), "PublishError(\"test error\")");
    }

    #[test]
    fn test_channel_manager_new() {
        let (sender, receiver) = mpsc::channel();
        let manager = ChannelManager::new(receiver);

        assert_eq!(manager.channel_count(), 0);
        drop(sender); // Cerrar el canal
    }

    #[test]
    fn test_channel_count() {
        let (sender, receiver) = mpsc::channel();
        let mut manager = ChannelManager::new(receiver);

        assert_eq!(manager.channel_count(), 0);

        // Simular creación de un canal
        manager
            .channels
            .insert("test_channel".to_string(), HashMap::new());
        assert_eq!(manager.channel_count(), 1);

        drop(sender);
    }

    #[test]
    fn test_subscriber_count() {
        let (sender, receiver) = mpsc::channel();
        let mut manager = ChannelManager::new(receiver);

        // Canal que no existe
        assert_eq!(manager.subscriber_count("nonexistent"), None);

        // Canal vacío
        manager
            .channels
            .insert("empty_channel".to_string(), HashMap::new());
        assert_eq!(manager.subscriber_count("empty_channel"), Some(0));

        // Canal con suscriptores
        let mut subs = HashMap::new();
        let (client_sender, _) = mpsc::channel();
        subs.insert("client1".to_string(), client_sender);
        manager.channels.insert("active_channel".to_string(), subs);
        assert_eq!(manager.subscriber_count("active_channel"), Some(1));

        drop(sender);
    }

    #[test]
    fn test_is_subscribed() {
        let (sender, receiver) = mpsc::channel();
        let mut manager = ChannelManager::new(receiver);

        // Cliente no suscrito a canal inexistente
        assert!(!manager.is_subscribed("nonexistent", "client1"));

        // Cliente no suscrito a canal existente
        manager
            .channels
            .insert("test_channel".to_string(), HashMap::new());
        assert!(!manager.is_subscribed("test_channel", "client1"));

        // Cliente suscrito
        let mut subs = HashMap::new();
        let (client_sender, _) = mpsc::channel();
        subs.insert("client1".to_string(), client_sender);
        manager.channels.insert("active_channel".to_string(), subs);
        assert!(manager.is_subscribed("active_channel", "client1"));

        drop(sender);
    }

    #[test]
    fn test_get_channels() {
        let (sender, receiver) = mpsc::channel();
        let mut manager = ChannelManager::new(receiver);

        // Sin canales
        assert_eq!(manager.get_channels(), Vec::<String>::new());

        // Con canales
        manager
            .channels
            .insert("channel1".to_string(), HashMap::new());
        manager
            .channels
            .insert("channel2".to_string(), HashMap::new());

        let mut channels = manager.get_channels();
        channels.sort(); // Ordenar para comparación determinística

        assert_eq!(channels, vec!["channel1", "channel2"]);

        drop(sender);
    }

    #[test]
    fn test_get_subscribers() {
        let (sender, receiver) = mpsc::channel();
        let mut manager = ChannelManager::new(receiver);

        // Canal que no existe
        assert_eq!(manager.get_subscribers("nonexistent"), None);

        // Canal vacío
        manager
            .channels
            .insert("empty_channel".to_string(), HashMap::new());
        assert_eq!(
            manager.get_subscribers("empty_channel"),
            Some(Vec::<String>::new())
        );

        // Canal con suscriptores
        let mut subs = HashMap::new();
        let (client_sender1, _) = mpsc::channel();
        let (client_sender2, _) = mpsc::channel();
        subs.insert("client1".to_string(), client_sender1);
        subs.insert("client2".to_string(), client_sender2);
        manager.channels.insert("active_channel".to_string(), subs);

        let mut subscribers = manager.get_subscribers("active_channel").unwrap();
        subscribers.sort(); // Ordenar para comparación determinística

        assert_eq!(subscribers, vec!["client1", "client2"]);

        drop(sender);
    }

    #[test]
    fn test_handle_subscribe_success() {
        let (sender, receiver) = mpsc::channel();
        let mut manager = ChannelManager::new(receiver);
        let (response_sender, response_receiver) = mpsc::channel();
        let (client_sender, _) = mpsc::channel();

        // Simular suscripción exitosa
        let result = manager.handle_subscribe(
            "client1".to_string(),
            "test_channel".to_string(),
            response_sender,
            client_sender,
        );

        assert!(result.is_ok());
        assert_eq!(manager.channel_count(), 1);
        assert_eq!(manager.subscriber_count("test_channel"), Some(1));
        assert!(manager.is_subscribed("test_channel", "client1"));

        // Verificar respuesta
        let response = response_receiver.recv().unwrap();
        assert_eq!(response, "");

        drop(sender);
    }

    #[test]
    fn test_handle_subscribe_duplicate() {
        let (sender, receiver) = mpsc::channel();
        let mut manager = ChannelManager::new(receiver);
        let (response_sender1, response_receiver1) = mpsc::channel();
        let (response_sender2, response_receiver2) = mpsc::channel();
        let (client_sender1, _) = mpsc::channel();
        let (client_sender2, _) = mpsc::channel();

        // Primera suscripción
        let result1 = manager.handle_subscribe(
            "client1".to_string(),
            "test_channel".to_string(),
            response_sender1,
            client_sender1,
        );
        assert!(result1.is_ok());

        // Segunda suscripción del mismo cliente
        let result2 = manager.handle_subscribe(
            "client1".to_string(),
            "test_channel".to_string(),
            response_sender2,
            client_sender2,
        );
        assert!(result2.is_ok());

        // Verificar respuestas
        let response1 = response_receiver1.recv().unwrap();
        let response2 = response_receiver2.recv().unwrap();

        assert_eq!(response1, "");
        assert_eq!(response2, "Ya estás suscripto a ese canal");

        drop(sender);
    }

    #[test]
    fn test_handle_unsubscribe_success() {
        let (sender, receiver) = mpsc::channel();
        let mut manager = ChannelManager::new(receiver);
        let (response_sender, response_receiver) = mpsc::channel();
        let (client_sender, _) = mpsc::channel();

        // Suscribir primero
        let mut subs = HashMap::new();
        subs.insert("client1".to_string(), client_sender);
        manager.channels.insert("test_channel".to_string(), subs);

        // Desuscribir
        let result = manager.handle_unsubscribe(
            "client1".to_string(),
            "test_channel".to_string(),
            response_sender,
        );

        assert!(result.is_ok());
        assert_eq!(manager.channel_count(), 0); // Canal eliminado automáticamente

        // Verificar respuesta
        let response = response_receiver.recv().unwrap();
        assert_eq!(response, "");

        drop(sender);
    }

    #[test]
    fn test_handle_unsubscribe_not_subscribed() {
        let (sender, receiver) = mpsc::channel();
        let mut manager = ChannelManager::new(receiver);
        let (response_sender, response_receiver) = mpsc::channel();

        // Crear canal sin suscriptores
        manager
            .channels
            .insert("test_channel".to_string(), HashMap::new());

        // Intentar desuscribir cliente no suscrito
        let result = manager.handle_unsubscribe(
            "client1".to_string(),
            "test_channel".to_string(),
            response_sender,
        );

        assert!(result.is_ok());

        // Verificar respuesta
        let response = response_receiver.recv().unwrap();
        assert_eq!(response, "No estabas suscripto a ese canal.");

        drop(sender);
    }

    #[test]
    fn test_handle_unsubscribe_channel_not_exists() {
        let (sender, receiver) = mpsc::channel();
        let mut manager = ChannelManager::new(receiver);
        let (response_sender, response_receiver) = mpsc::channel();

        // Intentar desuscribir de canal inexistente
        let result = manager.handle_unsubscribe(
            "client1".to_string(),
            "nonexistent".to_string(),
            response_sender,
        );

        assert!(result.is_ok());

        // Verificar respuesta
        let response = response_receiver.recv().unwrap();
        assert_eq!(response, "El canal no existe");

        drop(sender);
    }

    #[test]
    fn test_handle_publish_success() {
        let (sender, receiver) = mpsc::channel();
        let mut manager = ChannelManager::new(receiver);
        let (response_sender, response_receiver) = mpsc::channel();
        let (client_sender, client_receiver) = mpsc::channel();

        // Crear canal con suscriptor
        let mut subs = HashMap::new();
        subs.insert("client1".to_string(), client_sender);
        manager.channels.insert("test_channel".to_string(), subs);

        // Publicar mensaje
        let message = RespMessage::SimpleString("Hello World".to_string());
        let result =
            manager.handle_publish("test_channel".to_string(), message.clone(), response_sender);

        assert!(result.is_ok());

        // Verificar que el suscriptor recibió el mensaje
        let received_message = client_receiver.recv().unwrap();
        assert_eq!(received_message, message);

        // Verificar respuesta de confirmación
        let response = response_receiver.recv().unwrap();
        assert_eq!(response, "1");

        drop(sender);
    }

    #[test]
    fn test_handle_publish_channel_not_exists() {
        let (sender, receiver) = mpsc::channel();
        let mut manager = ChannelManager::new(receiver);
        let (response_sender, response_receiver) = mpsc::channel();

        // Publicar en canal inexistente
        let message = RespMessage::SimpleString("Hello World".to_string());
        let result = manager.handle_publish("nonexistent".to_string(), message, response_sender);

        assert!(result.is_ok());

        // Verificar respuesta de error
        let response = response_receiver.recv().unwrap();
        assert_eq!(response, "0");

        drop(sender);
    }

    #[test]
    fn test_send_response_success() {
        let (sender, receiver) = mpsc::channel();
        let manager = ChannelManager::new(receiver);
        let (response_sender, response_receiver) = mpsc::channel();

        let result = manager.send_response(response_sender, "test message".to_string());
        assert!(result.is_ok());

        let response = response_receiver.recv().unwrap();
        assert_eq!(response, "test message");

        drop(sender);
    }

    #[test]
    fn test_send_response_error() {
        let (sender, receiver) = mpsc::channel();
        let manager = ChannelManager::new(receiver);

        // Crear un sender sin receptor para causar error
        let (bad_sender, _) = mpsc::channel::<String>();

        let result = manager.send_response(bad_sender, "test message".to_string());
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ChannelManagerError::SendResponseError(_)
        ));

        drop(sender);
    }
}
