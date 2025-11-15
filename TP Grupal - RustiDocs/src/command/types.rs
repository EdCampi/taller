//! Tipos asociados al manejo de estructuras de datos
//! y operaciones sobre la base de datos.
//!
//! Este módulo define los tipos fundamentales para el sistema de comandos,
//! incluyendo tipos de respuesta, comandos disponibles y contextos de ejecución.
//!
//! # Características principales
//!
//! - Tipos de respuesta tipados para operaciones de base de datos
//! - Enumeración completa de comandos soportados
//! - Contextos de ejecución para logging y pub/sub
//! - Manejo robusto de errores con enums específicos

// IMPORTS
use crate::network::RespMessage;
use crate::security::types::Password;
use std::collections::HashSet;
use std::sync::mpsc::Sender;

/// Errores específicos de tipos de comando
#[derive(Debug, Clone, PartialEq)]
pub enum CommandTypeError {
    /// Error cuando se intenta acceder a un campo inválido
    InvalidField(String),
    /// Error cuando se intenta crear un contexto inválido
    InvalidContext(String),
    /// Error cuando se intenta acceder a un sender no disponible
    SenderNotAvailable(String),
    /// Error genérico de tipo
    TypeError(String),
}

impl std::fmt::Display for CommandTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandTypeError::InvalidField(msg) => {
                write!(f, "Invalid field access: {}", msg)
            }
            CommandTypeError::InvalidContext(msg) => {
                write!(f, "Invalid context: {}", msg)
            }
            CommandTypeError::SenderNotAvailable(msg) => {
                write!(f, "Sender not available: {}", msg)
            }
            CommandTypeError::TypeError(msg) => {
                write!(f, "Type error: {}", msg)
            }
        }
    }
}

impl std::error::Error for CommandTypeError {}

// TIPOS

/// Tipos de respuestas posibles a una operación sobre la base
/// de datos:
/// * `String`;
/// * Int, `i64`;
/// * Array, `Vec<String>`;
/// * Set, `HashSet<String>`;
/// * Null (Objeto nulo), `None`;
///
/// Este enum representa todos los tipos de respuesta que puede devolver
/// una operación sobre la base de datos, incluyendo strings, enteros,
/// listas, sets y valores nulos.
#[derive(Clone, Debug, PartialEq)]
pub enum ResponseType {
    /// Respuesta de tipo string
    Str(String),
    /// Respuesta de tipo entero
    Int(i64),
    /// Respuesta de tipo lista
    List(Vec<String>),
    /// Respuesta de tipo conjunto
    Set(HashSet<String>),
    /// Respuesta nula
    Null(Option<()>),
}

impl ResponseType {
    /// Obtiene el valor como string si es de tipo Str
    ///
    /// # Returns
    ///
    /// `Option<&String>` - El valor string si existe, None en caso contrario
    pub fn as_str(&self) -> Option<&String> {
        match self {
            ResponseType::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Obtiene el valor como entero si es de tipo Int
    ///
    /// # Returns
    ///
    /// `Option<i64>` - El valor entero si existe, None en caso contrario
    pub fn as_int(&self) -> Option<i64> {
        match self {
            ResponseType::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Obtiene el valor como lista si es de tipo List
    ///
    /// # Returns
    ///
    /// `Option<&Vec<String>>` - La lista si existe, None en caso contrario
    pub fn as_list(&self) -> Option<&Vec<String>> {
        match self {
            ResponseType::List(l) => Some(l),
            _ => None,
        }
    }

    /// Obtiene el valor como conjunto si es de tipo Set
    ///
    /// # Returns
    ///
    /// `Option<&HashSet<String>>` - El conjunto si existe, None en caso contrario
    pub fn as_set(&self) -> Option<&HashSet<String>> {
        match self {
            ResponseType::Set(s) => Some(s),
            _ => None,
        }
    }

    /// Verifica si la respuesta es nula
    ///
    /// # Returns
    ///
    /// `bool` - True si es nula, False en caso contrario
    pub fn is_null(&self) -> bool {
        matches!(self, ResponseType::Null(_))
    }
}

/// Lista de comandos contemplados por la base de datos.
///
/// Este enum representa todos los comandos disponibles en el sistema,
/// organizados por categorías: strings, listas, sets, base de datos,
/// pub/sub y clustering.
///
/// # Categorías de comandos
///
/// ## String Commands
/// - `Append` - Concatena un valor a una clave existente
/// - `Echo` - Devuelve el string que recibe
/// - `Get` - Obtiene el valor de una clave
/// - `Getdel` - Obtiene y elimina el valor de una clave
/// - `Getrange` - Obtiene un substring
/// - `Set` - Establece el valor de una clave
/// - `Strlen` - Obtiene la longitud de un string
/// - `Substr` - Obtiene un substring
///
/// ## List Commands
/// - `Del` - Elimina claves
/// - `Llen` - Obtiene la longitud de una lista
/// - `Lpop` - Elimina elementos del inicio de una lista
/// - `Lpush` - Agrega elementos al inicio de una lista
/// - `Lrange` - Obtiene un rango de elementos de una lista
/// - `Rpop` - Elimina elementos del final de una lista
/// - `Rpush` - Agrega elementos al final de una lista
///
/// ## Set Commands
/// - `Sadd` - Agrega elementos a un conjunto
/// - `Scard` - Obtiene el cardinal de un conjunto
/// - `Sismember` - Verifica si un elemento pertenece a un conjunto
/// - `Smembers` - Obtiene todos los elementos de un conjunto
/// - `SMove` - Mueve un elemento entre conjuntos
/// - `Spop` - Elimina elementos aleatorios de un conjunto
///
/// ## Database Commands
/// - `BgSave` - Guarda la base de datos en segundo plano
/// - `Save` - Guarda la base de datos
///
/// ## Pub/Sub Commands
/// - `Subscribe` - Suscribe a un canal
/// - `Unsubscribe` - Desuscribe de un canal
/// - `Publish` - Publica un mensaje en un canal
///
/// ## Cluster Commands
/// - `Meet` - Inicia el proceso de unión a un cluster
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    // STRING COMMANDS
    /// Concatena un valor a una clave existente
    ///
    /// # Arguments
    /// * `key` - Clave a la cual concatenar
    /// * `value` - Valor a concatenar
    ///
    /// # Returns
    /// Longitud total del valor final
    Append(String, String),

    /// Devuelve el string que recibe
    ///
    /// # Arguments
    /// * `message` - Mensaje a devolver
    Echo(String),

    /// Devuelve un string
    ///
    /// # Arguments
    /// * `key` - Clave a obtener
    Get(String),

    /// Obtiene y elimina el valor de una clave
    ///
    /// # Arguments
    /// * `key` - Clave a obtener y eliminar
    ///
    /// # Returns
    /// String o nil si no existe
    Getdel(String),

    /// Devuelve un substring de un string
    ///
    /// # Arguments
    /// * `key` - Clave del string
    /// * `start` - Índice de inicio
    /// * `end` - Índice de fin
    Getrange(String, i64, i64),

    /// Establece el valor de una clave
    ///
    /// # Arguments
    /// * `key` - Clave a establecer
    /// * `value` - Valor a asignar
    ///
    /// # Returns
    /// "OK" string
    Set(String, String),

    /// Obtiene la longitud de un string
    ///
    /// # Arguments
    /// * `key` - Clave del string
    ///
    /// # Returns
    /// Longitud del string
    Strlen(String),

    /// Devuelve un substring de un string
    ///
    /// # Arguments
    /// * `key` - Clave del string
    /// * `start` - Índice de inicio
    /// * `end` - Índice de fin
    Substr(String, i64, i64),

    // LIST COMMANDS
    /// Elimina claves
    ///
    /// # Arguments
    /// * `keys` - Vector de claves a eliminar
    ///
    /// # Returns
    /// Cantidad de claves eliminadas
    Del(Vec<String>),

    /// Obtiene la longitud de una lista
    ///
    /// # Arguments
    /// * `key` - Clave de la lista
    ///
    /// # Returns
    /// Longitud de la lista, 0 si no existe
    Llen(String),

    /// Elimina elementos del inicio de una lista
    ///
    /// # Arguments
    /// * `key` - Clave de la lista
    /// * `count` - Cantidad de elementos a eliminar (por defecto 1)
    ///
    /// # Returns
    /// Vector de elementos eliminados
    Lpop(String, i64),

    /// Agrega elementos al inicio de una lista
    ///
    /// # Arguments
    /// * `key` - Clave de la lista
    /// * `values` - Valores a agregar
    ///
    /// # Returns
    /// Posición del elemento agregado
    Lpush(String, Vec<String>),

    /// Obtiene un rango de elementos de una lista
    ///
    /// # Arguments
    /// * `key` - Clave de la lista
    /// * `start` - Índice de inicio
    /// * `end` - Índice de fin
    ///
    /// # Returns
    /// Lista de elementos en el rango
    Lrange(String, i64, i64),

    /// Elimina elementos del final de una lista
    ///
    /// # Arguments
    /// * `key` - Clave de la lista
    /// * `count` - Cantidad de elementos a eliminar (por defecto 1)
    ///
    /// # Returns
    /// Vector de elementos eliminados
    Rpop(String, i64),

    /// Agrega elementos al final de una lista
    ///
    /// # Arguments
    /// * `key` - Clave de la lista
    /// * `values` - Valores a agregar
    ///
    /// # Returns
    /// Posición del elemento agregado
    Rpush(String, Vec<String>),

    // SET COMMANDS
    /// Agrega elementos a un conjunto
    ///
    /// # Arguments
    /// * `key` - Clave del conjunto
    /// * `values` - Valores a agregar
    ///
    /// # Returns
    /// Cantidad de elementos agregados
    Sadd(String, Vec<String>),

    /// Obtiene el cardinal de un conjunto
    ///
    /// # Arguments
    /// * `key` - Clave del conjunto
    ///
    /// # Returns
    /// Cardinal del conjunto
    Scard(String),

    /// Verifica si un elemento pertenece a un conjunto
    ///
    /// # Arguments
    /// * `key` - Clave del conjunto
    /// * `member` - Elemento a verificar
    ///
    /// # Returns
    /// 1 si pertenece, 0 en caso contrario
    Sismember(String, String),

    /// Obtiene todos los elementos de un conjunto
    ///
    /// # Arguments
    /// * `key` - Clave del conjunto
    ///
    /// # Returns
    /// HashSet con todos los elementos
    Smembers(String),

    /// Mueve un elemento entre conjuntos
    ///
    /// # Arguments
    /// * `source` - Clave del conjunto origen
    /// * `destination` - Clave del conjunto destino
    /// * `member` - Elemento a mover
    ///
    /// # Returns
    /// 1 si se movió, 0 si no
    SMove(String, String, String),

    /// Elimina elementos aleatorios de un conjunto
    ///
    /// # Arguments
    /// * `key` - Clave del conjunto
    /// * `count` - Cantidad de elementos a eliminar (por defecto 1)
    ///
    /// # Returns
    /// Vector de elementos eliminados
    Spop(String, i64),

    // DB COMMANDS
    /// Guarda la base de datos en segundo plano
    BgSave,

    /// Guarda la base de datos
    Save,

    // PUBSUB COMMANDS
    /// Suscribe a un canal
    ///
    /// # Arguments
    /// * `channel` - Nombre del canal
    Subscribe(String),

    /// Desuscribe de un canal
    ///
    /// # Arguments
    /// * `channel` - Nombre del canal
    Unsubscribe(String),

    /// Publica un mensaje en un canal
    ///
    /// # Arguments
    /// * `channel` - Nombre del canal
    /// * `message` - Mensaje a publicar
    Publish(String, RespMessage),

    // CLUSTER COMMANDS
    /// Inicia el proceso de unión a un cluster
    ///
    /// # Arguments
    /// * `address` - Dirección del nodo a contactar
    Meet(String),

    /// Devuelve la información total del cluster
    /// que posee el nodo al cual el cliente
    /// está conectado.
    Slots,

    // LOG COMMANDS
    /// Permite al usuario loggearse y evita que no realize
    /// consultas fuera de sus privilegios.
    ///
    /// # Arguments
    /// * `user` - Nombre de usuario
    /// * `password` - Contraseña
    Auth(String, Password),
}

impl Command {
    /// Obtiene la categoría del comando
    ///
    /// # Returns
    ///
    /// `&'static str` - Categoría del comando
    pub fn category(&self) -> &'static str {
        match self {
            // String commands
            Command::Append(_, _)
            | Command::Echo(_)
            | Command::Get(_)
            | Command::Getdel(_)
            | Command::Getrange(_, _, _)
            | Command::Set(_, _)
            | Command::Strlen(_)
            | Command::Substr(_, _, _) => "STRING",

            // List commands
            Command::Del(_)
            | Command::Llen(_)
            | Command::Lpop(_, _)
            | Command::Lpush(_, _)
            | Command::Lrange(_, _, _)
            | Command::Rpop(_, _)
            | Command::Rpush(_, _) => "LIST",

            // Set commands
            Command::Sadd(_, _)
            | Command::Scard(_)
            | Command::Sismember(_, _)
            | Command::Smembers(_)
            | Command::SMove(_, _, _)
            | Command::Spop(_, _) => "SET",

            // Database commands
            Command::BgSave | Command::Save => "DB",

            // Pub/Sub commands
            Command::Subscribe(_) | Command::Unsubscribe(_) | Command::Publish(_, _) => "PUBSUB",

            // Cluster commands
            Command::Meet(_) | Command::Slots => "CLUSTER",

            // Log commands
            Command::Auth(_, _) => "LOG",
        }
    }

    /// Verifica si el comando es de solo lectura
    ///
    /// # Returns
    ///
    /// `bool` - True si es de solo lectura, False en caso contrario
    pub fn is_read_only(&self) -> bool {
        matches!(
            self,
            Command::Echo(_)
                | Command::Get(_)
                | Command::Getrange(_, _, _)
                | Command::Strlen(_)
                | Command::Substr(_, _, _)
                | Command::Llen(_)
                | Command::Lrange(_, _, _)
                | Command::Scard(_)
                | Command::Sismember(_, _)
                | Command::Smembers(_)
        )
    }

    /// Returns the name of the command
    pub fn to_string(&self) -> String {
        match self {
            Command::Append(_, _) => "APPEND",
            Command::Echo(_) => "ECHO",
            Command::Get(_) => "GET",
            Command::Getdel(_) => "GETDEL",
            Command::Getrange(_, _, _) => "GETRANGE",
            Command::Set(_, _) => "SET",
            Command::Strlen(_) => "STRLEN",
            Command::Substr(_, _, _) => "SUBSTR",
            Command::Del(_) => "DEL",
            Command::Llen(_) => "LLEN",
            Command::Lpop(_, _) => "LPOP",
            Command::Lpush(_, _) => "LPUSH",
            Command::Lrange(_, _, _) => "LRANGE",
            Command::Rpop(_, _) => "RPOP",
            Command::Rpush(_, _) => "RPUSH",
            Command::Sadd(_, _) => "SADD",
            Command::Scard(_) => "SCARD",
            Command::Sismember(_, _) => "SISMEMBER",
            Command::Smembers(_) => "SMEMBERS",
            Command::SMove(_, _, _) => "SMOVE",
            Command::Spop(_, _) => "SPOP",
            Command::BgSave => "BGSAVE",
            Command::Save => "SAVE",
            Command::Subscribe(_) => "SUBSCRIBE",
            Command::Unsubscribe(_) => "UNSUBSCRIBE",
            Command::Publish(_, _) => "PUBLISH",
            Command::Meet(_) => "MEET",
            Command::Slots => "SLOTS",
            Command::Auth(_, _) => "AUTH",
        }
        .to_string()
    }
}

/// Contexto de ejecución para comandos de pub/sub.
///
/// Proporciona acceso al ID del cliente y los canales de comunicación
/// necesarios para operaciones de pub/sub.
pub struct PubSubContext<'a> {
    /// ID del cliente
    client_id: String,
    /// Sender para enviar comandos
    sender: &'a Sender<(String, Command, Sender<String>, Sender<RespMessage>)>,
    /// Sender para enviar respuestas
    res_sender: &'a Sender<RespMessage>,
}

impl<'a> PubSubContext<'a> {
    /// Crea una nueva instancia de PubSubContext.
    ///
    /// # Arguments
    ///
    /// * `client_id` - ID del cliente
    /// * `sender` - Sender para enviar comandos
    /// * `res_sender` - Sender para enviar respuestas
    ///
    /// # Returns
    ///
    /// Nueva instancia de `PubSubContext`
    pub(crate) fn new(
        client_id: String,
        sender: &'a Sender<(String, Command, Sender<String>, Sender<RespMessage>)>,
        res_sender: &'a Sender<RespMessage>,
    ) -> Self {
        Self {
            client_id,
            sender,
            res_sender,
        }
    }

    /// Obtiene el ID del cliente.
    ///
    /// # Returns
    ///
    /// `String` - ID del cliente
    pub(crate) fn get_cid(&self) -> String {
        self.client_id.clone()
    }

    /// Obtiene una referencia al sender de comandos.
    ///
    /// # Returns
    ///
    /// `&'a Sender<(String, Command, Sender<String>, Sender<RespMessage>)>` - Referencia al sender
    pub(crate) fn get_sender(
        &self,
    ) -> &'a Sender<(String, Command, Sender<String>, Sender<RespMessage>)> {
        self.sender
    }

    /// Obtiene una referencia al sender de respuestas.
    ///
    /// # Returns
    ///
    /// `&'a Sender<RespMessage>` - Referencia al sender de respuestas
    pub(crate) fn get_res_sender(&self) -> &'a Sender<RespMessage> {
        self.res_sender
    }

    /// Verifica si el contexto es válido.
    ///
    /// # Returns
    ///
    /// `Result<(), CommandTypeError>` - Ok si es válido, error en caso contrario
    pub fn validate(&self) -> Result<(), CommandTypeError> {
        if self.client_id.is_empty() {
            return Err(CommandTypeError::InvalidContext(
                "Client ID cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::mpsc;

    #[test]
    fn test_command_type_error_display() {
        let error = CommandTypeError::InvalidField("test".to_string());
        assert_eq!(error.to_string(), "Invalid field access: test");
    }

    #[test]
    fn test_command_type_error_debug() {
        let error = CommandTypeError::TypeError("test".to_string());
        assert_eq!(format!("{:?}", error), "TypeError(\"test\")");
    }

    #[test]
    fn test_response_type_as_str() {
        let response = ResponseType::Str("test".to_string());
        assert_eq!(response.as_str(), Some(&"test".to_string()));

        let int_response = ResponseType::Int(42);
        assert_eq!(int_response.as_str(), None);
    }

    #[test]
    fn test_response_type_as_int() {
        let response = ResponseType::Int(42);
        assert_eq!(response.as_int(), Some(42));

        let str_response = ResponseType::Str("test".to_string());
        assert_eq!(str_response.as_int(), None);
    }

    #[test]
    fn test_response_type_as_list() {
        let list = vec!["item1".to_string(), "item2".to_string()];
        let response = ResponseType::List(list.clone());
        assert_eq!(response.as_list(), Some(&list));

        let str_response = ResponseType::Str("test".to_string());
        assert_eq!(str_response.as_list(), None);
    }

    #[test]
    fn test_response_type_as_set() {
        let mut set = HashSet::new();
        set.insert("item1".to_string());
        let response = ResponseType::Set(set.clone());
        assert_eq!(response.as_set(), Some(&set));

        let str_response = ResponseType::Str("test".to_string());
        assert_eq!(str_response.as_set(), None);
    }

    #[test]
    fn test_response_type_is_null() {
        let null_response = ResponseType::Null(None);
        assert!(null_response.is_null());

        let str_response = ResponseType::Str("test".to_string());
        assert!(!str_response.is_null());
    }

    #[test]
    fn test_command_category() {
        assert_eq!(Command::Get("key".to_string()).category(), "STRING");
        assert_eq!(Command::Llen("key".to_string()).category(), "LIST");
        assert_eq!(Command::Sadd("key".to_string(), vec![]).category(), "SET");
        assert_eq!(Command::BgSave.category(), "DB");
        assert_eq!(
            Command::Subscribe("channel".to_string()).category(),
            "PUBSUB"
        );
        assert_eq!(Command::Meet("address".to_string()).category(), "CLUSTER");
    }

    #[test]
    fn test_command_is_read_only() {
        assert!(Command::Get("key".to_string()).is_read_only());
        assert!(Command::Llen("key".to_string()).is_read_only());
        assert!(Command::Scard("key".to_string()).is_read_only());

        assert!(!Command::Set("key".to_string(), "value".to_string()).is_read_only());
        assert!(!Command::Del(vec!["key".to_string()]).is_read_only());
        assert!(!Command::Sadd("key".to_string(), vec!["value".to_string()]).is_read_only());
    }

    #[test]
    fn test_pubsub_context_new() {
        let (sender, _receiver) = mpsc::channel();
        let (res_sender, _res_receiver) = mpsc::channel();
        let context = PubSubContext::new("client1".to_string(), &sender, &res_sender);

        assert_eq!(context.get_cid(), "client1");
        assert_eq!(context.get_sender() as *const _, &sender as *const _);
        assert_eq!(
            context.get_res_sender() as *const _,
            &res_sender as *const _
        );
    }

    #[test]
    fn test_pubsub_context_validate() {
        let (sender, _receiver) = mpsc::channel();
        let (res_sender, _res_receiver) = mpsc::channel();

        let valid_context = PubSubContext::new("client1".to_string(), &sender, &res_sender);
        assert!(valid_context.validate().is_ok());

        let invalid_context = PubSubContext::new("".to_string(), &sender, &res_sender);
        assert!(invalid_context.validate().is_err());
    }

    #[test]
    fn test_response_type_clone() {
        let original = ResponseType::Str("test".to_string());
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_command_clone() {
        let original = Command::Get("key".to_string());
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_response_type_debug() {
        let response = ResponseType::Str("test".to_string());
        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("Str"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_command_debug() {
        let command = Command::Set("key".to_string(), "value".to_string());
        let debug_str = format!("{:?}", command);
        assert!(debug_str.contains("Set"));
        assert!(debug_str.contains("key"));
        assert!(debug_str.contains("value"));
    }
}
