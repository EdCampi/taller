//! Implementación de la estructura ejecutora de comandos
//! y los comandos en sí.

//! Este módulo proporciona la funcionalidad principal para ejecutar comandos Redis
//! en un sistema distribuido con soporte para sharding y clustering.
//!
//! # Características principales
//!
//! - Ejecución de comandos de lectura y escritura
//! - Soporte para sharding basado en hash slots
//! - Redirección automática de comandos a nodos correctos
//! - Manejo de snapshots automáticos
//! - Integración con sistema PubSub

// IMPORTS
use crate::cluster::state::flags::{MASTER, NodeFlags};
use crate::cluster::types::get_node_ip_for_slot;
use crate::{
    cluster::{
        sharding::hash_slot::hash_slot,
        state::node_data::NodeData,
        types::{KnownNode, NodeId},
    },
    command::ResponseType,
    command::{
        Instruction,
        commands::*,
        types::{Command, PubSubContext},
    },
    config::node_configs::NodeConfigs,
    logs::aof_logger::AofLogger,
    network::resp_message::RespMessage,
    storage::{data_store::DataStore, snapshot_manager::create_dump},
};
use std::{
    collections::HashMap,
    sync::{
        Arc, RwLock,
        mpsc::{Receiver, Sender},
    },
};

/// Errores específicos que pueden ocurrir durante la ejecución de comandos.
#[derive(Debug)]
pub enum CommandExecutorError {
    /// Error al leer el lock del DataStore
    DataStoreReadError(String),

    /// Error al escribir en el DataStore
    DataStoreWriteError(String),

    /// Error al convertir instrucción a comando
    CommandConversionError(String),

    /// Error al calcular hash slot
    HashSlotError(String),

    /// Error al ejecutar comando de lectura
    ReadCommandError(String),

    /// Error al ejecutar comando de escritura
    WriteCommandError(String),

    /// Error al crear snapshot
    SnapshotError(String),

    /// Error al enviar respuesta
    ResponseSendError(String),

    /// Error al obtener configuración requerida
    MissingConfigError(String),

    /// Error al obtener dependencia requerida
    MissingDependencyError(String),

    /// Error de permisos
    NotEnoughPermissions(String),
}

impl std::fmt::Display for CommandExecutorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandExecutorError::DataStoreReadError(msg) => {
                write!(f, "Error al leer DataStore: {}", msg)
            }
            CommandExecutorError::DataStoreWriteError(msg) => {
                write!(f, "Error al escribir en DataStore: {}", msg)
            }
            CommandExecutorError::CommandConversionError(msg) => {
                write!(f, "Error al convertir instrucción a comando: {}", msg)
            }
            CommandExecutorError::HashSlotError(msg) => {
                write!(f, "Error al calcular hash slot: {}", msg)
            }
            CommandExecutorError::ReadCommandError(msg) => {
                write!(f, "Error al ejecutar comando de lectura: {}", msg)
            }
            CommandExecutorError::WriteCommandError(msg) => {
                write!(f, "Error al ejecutar comando de escritura: {}", msg)
            }
            CommandExecutorError::SnapshotError(msg) => {
                write!(f, "Error al crear snapshot: {}", msg)
            }
            CommandExecutorError::ResponseSendError(msg) => {
                write!(f, "Error al enviar respuesta: {}", msg)
            }
            CommandExecutorError::MissingConfigError(msg) => {
                write!(f, "Configuración requerida faltante: {}", msg)
            }
            CommandExecutorError::MissingDependencyError(msg) => {
                write!(f, "Dependencia requerida faltante: {}", msg)
            }
            CommandExecutorError::NotEnoughPermissions(msg) => {
                write!(
                    f,
                    "No tenés los permisos necesarios para ejecutar el comando {}",
                    msg
                )
            }
        }
    }
}

impl std::error::Error for CommandExecutorError {}

// CÓDIGO
/// Estructura ejecutora de comandos, responsabilidades:
/// * Guardar la base de datos del nodo.
/// * Ejecutar las instrucciones recibidas.
/// * Manejar redirecciones de comandos basadas en hash slots.
/// * Crear snapshots automáticos.
/// * Coordinar con el sistema PubSub.
pub struct CommandExecutor {
    ds_guard: Arc<RwLock<DataStore>>,
    instruction_receiver: Receiver<(String, Instruction, Sender<RespMessage>)>,
    counter: u64,
    settings: NodeConfigs,
    logger: Arc<AofLogger>,
    pubsub_sender: Sender<(String, Command, Sender<String>, Sender<RespMessage>)>,
    nodes_list: Arc<RwLock<HashMap<NodeId, KnownNode>>>,
    data_lock: Arc<RwLock<NodeData>>,
}

impl CommandExecutor {
    /// Crea una nueva instancia del ejecutor de comandos.
    ///
    /// # Argumentos
    ///
    /// * `ds_guard` - Guard compartido del DataStore
    /// * `instruction_receiver` - Receptor de instrucciones
    /// * `settings` - Configuración del nodo
    /// * `logger` - Logger para operaciones AOF
    /// * `pubsub_sender` - Sender para comunicación PubSub
    /// * `nodes_list` - Lista de nodos conocidos
    /// * `data_lock` - Datos del nodo actual
    ///
    /// # Retorna
    ///
    /// Una nueva instancia de `CommandExecutor`
    pub fn new(
        ds_guard: Arc<RwLock<DataStore>>,
        instruction_receiver: Receiver<(String, Instruction, Sender<RespMessage>)>,
        settings: NodeConfigs,
        logger: Arc<AofLogger>,
        pubsub_sender: Sender<(String, Command, Sender<String>, Sender<RespMessage>)>,
        nodes_list: Arc<RwLock<HashMap<NodeId, KnownNode>>>,
        data_lock: Arc<RwLock<NodeData>>,
    ) -> Self {
        Self {
            ds_guard,
            instruction_receiver,
            logger,
            counter: 0,
            settings,
            pubsub_sender,
            nodes_list,
            data_lock,
        }
    }

    /// Ejecuta el bucle principal del ejecutor de comandos.
    ///
    /// Este método procesa instrucciones de forma continua hasta que
    /// recibe un client_id vacío, momento en el cual termina la ejecución.
    pub fn run(&mut self) {
        while let Ok((client_id, instruction, response_sender)) = self.instruction_receiver.recv() {
            if client_id.is_empty() {
                self.logger.log_debug("Closing executor thread".to_string());
                break;
            }
            let pubsub_sender = self.pubsub_sender.clone();
            let response =
                self.execute_instruction(client_id, instruction, &pubsub_sender, &response_sender);
            if let Err(e) = response_sender.send(response) {
                self.logger
                    .log_error(format!("Error sending response: {}", e));
            }
        }
    }

    /// Formatea un error de lectura con contexto.
    ///
    /// # Argumentos
    ///
    /// * `ins_type` - Tipo de instrucción
    /// * `args` - Argumentos de la instrucción
    /// * `e` - Mensaje de error
    ///
    /// # Retorna
    ///
    /// String formateada con el error
    fn format_reading_error(ins_type: &str, args: &[String], e: &dyn std::fmt::Display) -> String {
        format!(
            "ERROR when trying to read on {} with {:?}, {}",
            ins_type, args, e
        )
    }

    /// Formatea un error de operación con contexto.
    ///
    /// # Argumentos
    ///
    /// * `ins_type` - Tipo de instrucción
    /// * `args` - Argumentos de la instrucción
    /// * `e` - Mensaje de error
    ///
    /// # Retorna
    ///
    /// String formateada con el error
    fn format_op_error(ins_type: &str, args: &[String], e: &dyn std::fmt::Display) -> String {
        format!("ERROR on {} with {:?}, {}", ins_type, args, e)
    }

    /// Ejecuta un comando de lectura.
    ///
    /// # Argumentos
    ///
    /// * `instruction` - Instrucción a ejecutar
    /// * `command` - Comando convertido
    /// * `client_id` - ID del cliente
    /// * `pubsub_sender` - Sender para PubSub
    /// * `response_sender` - Sender para respuesta
    ///
    /// # Retorna
    ///
    /// `Result<RespMessage, CommandExecutorError>`
    fn execute_read_command(
        &self,
        instruction: &Instruction,
        command: &Command,
        client_id: String,
        pubsub_sender: &Sender<(String, Command, Sender<String>, Sender<RespMessage>)>,
        response_sender: &Sender<RespMessage>,
    ) -> Result<RespMessage, CommandExecutorError> {
        let guard = self.ds_guard.read().map_err(|e| {
            CommandExecutorError::DataStoreReadError(Self::format_reading_error(
                &instruction.instruction_type,
                &instruction.arguments,
                &e,
            ))
        })?;

        let response = command
            .execute_read(
                &guard,
                Some(self.settings.clone()),
                Some(self.logger.clone()),
                Some(PubSubContext::new(
                    client_id,
                    pubsub_sender,
                    response_sender,
                )),
                Some(&self.data_lock),
                Some(&self.nodes_list),
            )
            .map_err(|e| {
                CommandExecutorError::ReadCommandError(Self::format_op_error(
                    &instruction.instruction_type,
                    &instruction.arguments,
                    &e,
                ))
            })?;

        Ok(RespMessage::from_response(response))
    }

    /// Ejecuta un comando de escritura.
    ///
    /// # Argumentos
    ///
    /// * `instruction` - Instrucción a ejecutar
    /// * `command` - Comando convertido
    ///
    /// # Retorna
    ///
    /// `Result<RespMessage, CommandExecutorError>`
    fn execute_write_command(
        &mut self,
        instruction: &Instruction,
        command: &Command,
    ) -> Result<RespMessage, CommandExecutorError> {
        let myself = self.data_lock.read().unwrap();
        if !NodeFlags::state_contains(myself.get_state(), MASTER) {
            return Err(CommandExecutorError::NotEnoughPermissions(format!(
                "{}",
                command.to_string()
            )));
        }
        drop(myself);

        let mut guard = self.ds_guard.write().map_err(|e| {
            CommandExecutorError::DataStoreWriteError(Self::format_reading_error(
                &instruction.instruction_type,
                &instruction.arguments,
                &e,
            ))
        })?;

        let response = command.execute_write(&mut *guard).map_err(|e| {
            CommandExecutorError::WriteCommandError(Self::format_op_error(
                &instruction.instruction_type,
                &instruction.arguments,
                &e,
            ))
        })?;

        self.counter += 1;
        Ok(RespMessage::from_response(response))
    }

    /// Intenta ejecutar una instrucción con manejo de redirección.
    ///
    /// # Argumentos
    ///
    /// * `client_id` - ID del cliente
    /// * `instruction` - Instrucción a ejecutar
    /// * `pubsub_sender` - Sender para PubSub
    /// * `response_sender` - Sender para respuesta
    ///
    /// # Retorna
    ///
    /// `Result<RespMessage, CommandExecutorError>`
    fn try_execute(
        &mut self,
        client_id: String,
        instruction: &Instruction,
        pubsub_sender: &Sender<(String, Command, Sender<String>, Sender<RespMessage>)>,
        response_sender: &Sender<RespMessage>,
    ) -> Result<RespMessage, CommandExecutorError> {
        let command = instruction.to_command().map_err(|e| {
            CommandExecutorError::CommandConversionError(Self::format_op_error(
                &instruction.instruction_type,
                &instruction.arguments,
                &e,
            ))
        })?;

        // Verificar si necesitamos redirigir el comando
        if let Some(key) = get_key_for_command(&command) {
            let slot =
                hash_slot(&key).map_err(|e| CommandExecutorError::HashSlotError(e.to_string()))?;

            let data = self
                .data_lock
                .read()
                .map_err(|e| CommandExecutorError::DataStoreReadError(e.to_string()))?;

            if !data.owns_slot(slot) {
                // El nodo no maneja este slot, se debe redirigir
                if let Some(redirect_ip) = get_node_ip_for_slot(slot, &self.nodes_list) {
                    return Ok(RespMessage::Error(format!(
                        "MOVED {} {}",
                        slot, redirect_ip
                    )));
                } else {
                    return Ok(RespMessage::Error(format!(
                        "Slot {} not handled and no known owner",
                        slot
                    )));
                }
            }
        }

        if command.writes_on_db() {
            return self.execute_write_command(instruction, &command);
        }

        self.execute_read_command(
            instruction,
            &command,
            client_id,
            pubsub_sender,
            response_sender,
        )
    }

    /// Ejecuta una instrucción con manejo de snapshots automáticos.
    ///
    /// # Argumentos
    ///
    /// * `client_id` - ID del cliente
    /// * `instruction` - Instrucción a ejecutar
    /// * `pubsub_sender` - Sender para PubSub
    /// * `response_sender` - Sender para respuesta
    ///
    /// # Retorna
    ///
    /// `RespMessage` con el resultado de la ejecución
    pub fn execute_instruction(
        &mut self,
        client_id: String,
        instruction: Instruction,
        pubsub_sender: &Sender<(String, Command, Sender<String>, Sender<RespMessage>)>,
        response_sender: &Sender<RespMessage>,
    ) -> RespMessage {
        // Verificar si necesitamos crear un snapshot
        if self.counter > 0 && self.counter % self.settings.get_snapshot_k_changes() == 0 {
            if let Err(e) = self.create_auto_snapshot() {
                self.logger
                    .log_error(format!("Error creating auto-snapshot: {}", e));
            }
        }

        self.try_execute(client_id, &instruction, pubsub_sender, response_sender)
            .unwrap_or_else(|e| {
                self.logger.log_debug(format!("{}", e));
                RespMessage::Error(e.to_string())
            })
    }

    /// Crea un snapshot automático del DataStore.
    ///
    /// # Retorna
    ///
    /// `Result<(), CommandExecutorError>`
    fn create_auto_snapshot(&self) -> Result<(), CommandExecutorError> {
        let guard = self
            .ds_guard
            .read()
            .map_err(|e| CommandExecutorError::DataStoreReadError(e.to_string()))?;

        let dst = &self.settings.get_snapshot_dst();
        create_dump(&guard, dst).map_err(|e| CommandExecutorError::SnapshotError(e.to_string()))
    }
}

impl Command {
    /// Ejecuta la operación de **escritura** asociada sobre la base de datos seleccionada.
    ///
    /// # Argumentos
    ///
    /// * `store` - Referencia mutable al DataStore
    /// * `_` - Configuración del nodo (no utilizada)
    /// * `_` - Logger AOF (no utilizado)
    ///
    /// # Retorna
    ///
    /// `Result<ResponseType, CommandError>` - Respuesta de la operación
    ///
    /// # Errores
    ///
    /// * `WRONGTYPE` - El valor asociado a la clave no es del tipo soportado por la op
    /// * `CommandError` - Otros errores de ejecución
    pub fn execute_write(&self, store: &mut DataStore) -> Result<ResponseType, CommandError> {
        match self {
            // STRING COMMANDS
            Command::Append(key, val) => str_concat(store, key, val),
            Command::Del(keys) => bulk_delete(store, keys),
            Command::Getdel(key) => retrieve_delete(store, key),
            Command::Set(key, value) => set(store, key.clone(), value.clone()),

            // LIST COMMANDS
            Command::Lpop(key, amount) | Command::Rpop(key, amount) => {
                list_pop(store, key, amount, &self)
            }
            Command::Lpush(key, val) => left_push(store, key, val),
            Command::Rpush(key, values) => append(store, key.clone(), values.clone()),

            // SET COMMANDS
            Command::Sadd(key, values) => sadd(store, key.clone(), values.clone()),
            Command::SMove(source, destination, value) => {
                move_data_to_other_set(store, source, destination, value)
            }
            Command::Spop(key, amount) => set_pop(store, key, amount),

            _ => Err(CommandError::Custom("Error non write command".to_string())),
        }
    }

    /// Ejecuta la operación de **lectura** asociada sobre la base de datos seleccionada.
    ///
    /// Precondición: Ejecutarla con un lock no bloqueante de tipo lectura del DataStore.
    ///
    /// # Argumentos
    ///
    /// * `store` - Referencia al DataStore
    /// * `settings` - Configuración del nodo
    /// * `logger` - Logger AOF
    /// * `pub_sub_context` - Contexto para operaciones PubSub
    ///
    /// # Retorna
    ///
    /// `Result<ResponseType, CommandError>` - Respuesta de la operación
    ///
    /// # Errores
    ///
    /// * `WRONGTYPE` - El valor asociado a la clave no es del tipo soportado por la op
    /// * `CommandError` - Otros errores de ejecución
    pub fn execute_read(
        &self,
        store: &DataStore,
        settings: Option<NodeConfigs>,
        logger: Option<Arc<AofLogger>>,
        pub_sub_context: Option<PubSubContext>,
        node_data: Option<&Arc<RwLock<NodeData>>>,
        known_nodes: Option<&Arc<RwLock<HashMap<NodeId, KnownNode>>>>,
    ) -> Result<ResponseType, CommandError> {
        match self {
            // STRING COMMANDS
            Command::Echo(val) => Ok(ResponseType::Str(format!("{}", val))),
            Command::Get(key) => get(store, key),
            Command::Substr(key, start, end) | Command::Getrange(key, start, end) => {
                string_slice(store, key, start, end)
            }
            Command::Strlen(key) => get_len(store, key, &self),

            // LIST COMMANDS
            Command::Llen(key) => get_len(store, key, &self),
            Command::Lrange(key, start, end) => get_slice(store, key, *start, *end),

            // SET COMMANDS
            Command::Scard(key) => get_len(store, key, &self),
            Command::Sismember(key, val) => get_set_data(store, key, val),
            Command::Smembers(key) => get_set_items(store, key),

            // PERSISTENCE COMMANDS
            Command::BgSave => {
                let settings =
                    settings.ok_or_else(|| CommandError::Custom("Settings missing".to_string()))?;
                let logger =
                    unwrap_or_fail_arc(logger, "logger").map_err(|e| CommandError::Custom(e))?;
                backup_ds(store, settings, logger, true)
            }
            Command::Save => {
                let settings =
                    settings.ok_or_else(|| CommandError::Custom("Settings missing".to_string()))?;
                let logger =
                    unwrap_or_fail_arc(logger, "logger").map_err(|e| CommandError::Custom(e))?;
                backup_ds(store, settings, logger, false)
            }

            // PUBSUB COMMANDS
            Command::Subscribe(channel_id) => {
                let context = pub_sub_context
                    .ok_or_else(|| CommandError::Custom("PubSub context missing".to_string()))?;
                subscribe(
                    context.get_cid(),
                    channel_id.clone(),
                    context.get_sender(),
                    &context.get_res_sender(),
                )
            }
            Command::Unsubscribe(channel_id) => {
                let context = pub_sub_context
                    .ok_or_else(|| CommandError::Custom("PubSub context missing".to_string()))?;
                unsubscribe(context.get_cid(), channel_id.clone(), context.get_sender())
            }
            Command::Publish(channel_id, message) => {
                let context = pub_sub_context
                    .ok_or_else(|| CommandError::Custom("PubSub context missing".to_string()))?;
                publish(
                    context.get_cid(),
                    channel_id.to_string(),
                    context.get_sender(),
                    message,
                )
            }
            Command::Meet(ip) => {
                let settings =
                    settings.ok_or_else(|| CommandError::Custom("Settings missing".to_string()))?;
                send_first_ping(ip, settings)
            }
            Command::Slots => {
                let data = node_data
                    .ok_or_else(|| CommandError::Custom("Node data missing".to_string()))?;
                let cluster_nodes = known_nodes
                    .ok_or_else(|| CommandError::Custom("PubSub context missing".to_string()))?;
                return_cluster_slots_data(data, cluster_nodes)
            }
            _ => Err(CommandError::Custom(
                "Error non only-read command".to_string(),
            )),
        }
    }

    /// Determina si el comando realiza operaciones de escritura en la base de datos.
    ///
    /// # Retorna
    ///
    /// `true` si el comando es de escritura, `false` en caso contrario
    pub fn writes_on_db(&self) -> bool {
        matches!(
            self,
            Command::Append(_, _)
                | Command::Del(_)
                | Command::Set(_, _)
                | Command::Getdel(_)
                | Command::Lpop(_, _)
                | Command::Rpop(_, _)
                | Command::Lpush(_, _)
                | Command::Rpush(_, _)
                | Command::Sadd(_, _)
                | Command::SMove(_, _, _)
                | Command::Spop(_, _)
        )
    }
}

/// Desenvuelve un Option<Arc<T>> o falla con un mensaje de error descriptivo.
///
/// # Argumentos
///
/// * `opt` - Option a desenvolver
/// * `name` - Nombre del recurso para el mensaje de error
///
/// # Retorna
///
/// `Result<Arc<T>, String>`
fn unwrap_or_fail_arc<T>(opt: Option<Arc<T>>, name: &str) -> Result<Arc<T>, String> {
    opt.ok_or_else(|| format!("Missing required dependency: {}", name))
}

/// Extrae la clave principal del comando si aplica (para hash slot).
///
/// # Argumentos
///
/// * `cmd` - Comando a analizar
///
/// # Retorna
///
/// `Option<String>` - Clave principal si aplica, None en caso contrario
fn get_key_for_command(cmd: &Command) -> Option<String> {
    match cmd {
        Command::Append(key, _)
        | Command::Get(key)
        | Command::Getdel(key)
        | Command::Set(key, _)
        | Command::Strlen(key)
        | Command::Substr(key, _, _)
        | Command::Getrange(key, _, _)
        | Command::Llen(key)
        | Command::Lpop(key, _)
        | Command::Rpop(key, _)
        | Command::Lpush(key, _)
        | Command::Rpush(key, _)
        | Command::Lrange(key, _, _)
        | Command::Scard(key)
        | Command::Sismember(key, _)
        | Command::Smembers(key)
        | Command::Sadd(key, _)
        | Command::Spop(key, _) => Some(key.clone()),

        //Command::Del(keys) => Some(keys),
        Command::SMove(source, destination, ..) => {
            // Requiere que ambos estén en el mismo slot
            let slot_src = match hash_slot(source) {
                Ok(slot) => slot,
                Err(_) => return Some(format!("ERR Invalid key: {}", source)),
            };
            let slot_dst = match hash_slot(destination) {
                Ok(slot) => slot,
                Err(_) => return Some(format!("ERR Invalid key: {}", destination)),
            };
            if slot_src != slot_dst {
                return Some(format!(
                    "CROSSSLOT Keys {} and {} hash to different slots",
                    source, destination
                ));
            }
            Some(source.clone()) // Usamos una para comprobar si el nodo lo maneja
        }

        // Comandos sin clave (como PING, QUIT, SUBSCRIBE, etc.)
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cluster::state::node_data::NodeData, command::Instruction,
        config::node_configs::NodeConfigs, logs::aof_logger::AofLogger,
        storage::data_store::DataStore,
    };
    use std::sync::mpsc;

    /// Crea un DataStore de prueba.
    fn create_test_datastore() -> Arc<RwLock<DataStore>> {
        Arc::new(RwLock::new(DataStore::new()))
    }

    /// Crea un logger de prueba.
    fn create_test_logger() -> Arc<AofLogger> {
        let settings = create_test_settings();
        AofLogger::new(settings)
    }

    /// Crea configuración de nodo de prueba.
    fn create_test_settings() -> NodeConfigs {
        // Crear un archivo de configuración temporal para el test
        let config_content = r#"
            bind 0.0.0.0
            port 6379
            role M
            maxclients 1000
            save 900 15
            dbfilename dump.rdb
            dir ./
            logfile redis.log
            loglevel notice
            node-id test_node_123
            hash-slots 0-16383
            "#;

        std::fs::write("test.conf", config_content).expect("Failed to write test nodes");
        let result = NodeConfigs::new("test.conf");
        std::fs::remove_file("test.conf").ok(); // Limpiar archivo temporal
        result.expect("Failed to create test nodes")
    }

    /// Crea un CommandExecutor de prueba.
    fn create_test_executor() -> (
        CommandExecutor,
        Sender<(String, Instruction, Sender<RespMessage>)>,
    ) {
        let (tx, rx) = mpsc::channel();
        let settings = create_test_settings();
        let node_data = NodeData::new(settings.clone());

        let executor = CommandExecutor::new(
            create_test_datastore(),
            rx,
            settings,
            create_test_logger(),
            mpsc::channel().0,
            Arc::new(RwLock::new(HashMap::new())),
            Arc::new(RwLock::new(node_data)),
        );
        (executor, tx)
    }

    /// Crea una instrucción de prueba.
    #[allow(dead_code)]
    fn create_test_instruction(cmd_type: &str, args: Vec<String>) -> Instruction {
        Instruction {
            instruction_type: cmd_type.to_string(),
            arguments: args,
        }
    }

    #[test]
    fn test_command_executor_new() {
        let (executor, _) = create_test_executor();
        assert_eq!(executor.counter, 0);
    }

    #[test]
    fn test_format_reading_error() {
        let error = CommandExecutor::format_reading_error(
            "GET",
            &["key1".to_string()],
            &"lock error".to_string(),
        );
        assert!(error.contains("ERROR when trying to read on GET"));
        assert!(error.contains("key1"));
        assert!(error.contains("lock error"));
    }

    #[test]
    fn test_format_op_error() {
        let error = CommandExecutor::format_op_error(
            "SET",
            &["key1".to_string(), "value1".to_string()],
            &"invalid type".to_string(),
        );
        assert!(error.contains("ERROR on SET"));
        assert!(error.contains("key1"));
        assert!(error.contains("value1"));
        assert!(error.contains("invalid type"));
    }

    #[test]
    fn test_get_key_for_command_string_commands() {
        let cmd = Command::Get("test_key".to_string());
        assert_eq!(get_key_for_command(&cmd), Some("test_key".to_string()));

        let cmd = Command::Set("test_key".to_string(), "test_value".to_string());
        assert_eq!(get_key_for_command(&cmd), Some("test_key".to_string()));
    }

    #[test]
    fn test_get_key_for_command_list_commands() {
        let cmd = Command::Lpush("test_key".to_string(), vec!["value1".to_string()]);
        assert_eq!(get_key_for_command(&cmd), Some("test_key".to_string()));

        let cmd = Command::Lrange("test_key".to_string(), 0, 10);
        assert_eq!(get_key_for_command(&cmd), Some("test_key".to_string()));
    }

    #[test]
    fn test_get_key_for_command_set_commands() {
        let cmd = Command::Sadd("test_key".to_string(), vec!["value1".to_string()]);
        assert_eq!(get_key_for_command(&cmd), Some("test_key".to_string()));

        let cmd = Command::Smembers("test_key".to_string());
        assert_eq!(get_key_for_command(&cmd), Some("test_key".to_string()));
    }

    #[test]
    fn test_get_key_for_command_no_key_commands() {
        let cmd = Command::Echo("test".to_string());
        assert_eq!(get_key_for_command(&cmd), None);

        let cmd = Command::BgSave;
        assert_eq!(get_key_for_command(&cmd), None);
    }

    #[test]
    fn test_command_writes_on_db() {
        assert!(Command::Set("key".to_string(), "value".to_string()).writes_on_db());
        assert!(Command::Del(vec!["key".to_string()]).writes_on_db());
        assert!(Command::Lpush("key".to_string(), vec!["value".to_string()]).writes_on_db());
        assert!(!Command::Get("key".to_string()).writes_on_db());
        assert!(!Command::Echo("test".to_string()).writes_on_db());
    }

    #[test]
    fn test_unwrap_or_fail_arc_success() {
        let arc = Arc::new("test");
        let result = unwrap_or_fail_arc(Some(arc.clone()), "test");
        assert!(result.is_ok());
        assert_eq!(*result.unwrap(), "test");
    }

    #[test]
    fn test_unwrap_or_fail_arc_failure() {
        let result: Result<Arc<String>, String> = unwrap_or_fail_arc(None, "test_dependency");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Missing required dependency: test_dependency")
        );
    }

    #[test]
    fn test_command_executor_error_display() {
        let error = CommandExecutorError::DataStoreReadError("test error".to_string());
        assert!(error.to_string().contains("Error al leer DataStore"));
        assert!(error.to_string().contains("test error"));
    }

    #[test]
    fn test_command_executor_error_debug() {
        let error = CommandExecutorError::CommandConversionError("test error".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("CommandConversionError"));
        assert!(debug_str.contains("test error"));
    }
}
