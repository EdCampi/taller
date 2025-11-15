//! Implementación del logger y sus funciones/macros relacionadas.

// IMPORTS
use crate::config::node_configs::NodeConfigs;
use crate::logs::log_types::LogType;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

// CÓDIGOS DE NIVELES DE LOGS
const WARNING: i64 = 0;
pub const NOTICE: i64 = 1;
const VERBOSE: i64 = 2;
const DEBUG: i64 = 3;

// CÓDIGO

/// Logger del servidor que funciona -precondición- sobre archivos `.aof` (append-only file).
/// Cada instancia abre un hilo que maneja el file handler del archivo a appendear,
/// si no existe, lo crea. Y dentro de ese hilo se queda esperando mensajes de log.
///
/// La estructura posee:
///
/// * `level` Tipo de logs permitidos.
/// * `sender` Extremo sender del canal de comunicación con el nodo que loggea la información.
#[derive(Clone, Debug)]
pub struct AofLogger {
    level: i64,
    sender: Sender<LogType>,
    role: String,
}

impl Drop for AofLogger {
    fn drop(&mut self) {
        self.sender.send(LogType::Shutdown).unwrap();
    }
}

impl AofLogger {
    /// Método para loggear una operación.
    /// Precondición: **Debe ser llamado una única vez por instancia**
    pub fn start_log_operation(logfile: String, level: i64, receiver: Receiver<LogType>) {
        let file = create_append_log_file(logfile);
        let mut writer = BufWriter::new(file);
        loop {
            match receiver.recv() {
                Ok(LogType::Shutdown) => break,
                Ok(log) => process_log(log, level, &mut writer),
                Err(_) => break,
            };
        }
    }

    pub fn new(node_settings: NodeConfigs) -> Arc<AofLogger> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let logfile = node_settings.get_log_dst();
        let level = set_level(node_settings.get_log_level());
        let role = node_settings.get_role();
        let _ = thread::Builder::new()
            .name("Logger".to_string())
            .spawn(move || {
                AofLogger::start_log_operation(logfile, level, receiver);
            });
        sender
            .send(LogType::Notice(
                "AOF Logger started".to_string(),
                role.to_string(),
            ))
            .unwrap();
        Arc::new(AofLogger {
            level,
            sender,
            role: role.to_string(),
        })
    }

    /// Setea el nivel de loggeo permitido.
    pub fn set_level(&mut self, level: i64) {
        self.level = level;
    }

    /// Setea el rol del nodo que loggea. El cambio es guardado
    /// en un log.
    pub fn set_role(&mut self, role: &str) {
        let msg = format!("Node role changed from {} to {}", self.role, role);
        self.sender
            .send(LogType::Notice(msg, self.role.clone()))
            .unwrap();
        self.role = role.to_string();
    }

    /// Loggea eventos comunes (verbose).
    pub fn log_event(&self, msg: String) {
        self.sender
            .send(LogType::RegEvent(msg, self.role.clone()))
            .unwrap();
    }

    /// Loggea notificaciones importantes.
    pub fn log_notice(&self, msg: String) {
        self.sender
            .send(LogType::Notice(msg, self.role.clone()))
            .unwrap();
    }

    /// Loggea advertencias, (posibles) problemas que todavía no
    /// detengan el funcionamiento del nodo.
    pub fn log_warning(&self, msg: String) {
        self.sender
            .send(LogType::Warn(msg, self.role.clone()))
            .unwrap();
    }

    /// Logs para mostrar acciones detalladamente.
    pub fn log_debug(&self, msg: String) {
        self.sender
            .send(LogType::Debug(msg, self.role.clone()))
            .unwrap();
    }

    /// Logs para mostrar errores que compromenten el funcionamiento
    /// y/o consistencia de los datos.
    pub fn log_error(&self, msg: String) {
        self.sender
            .send(LogType::RegEvent(msg, self.role.clone()))
            .unwrap();
    }

    /// Detiene la ejecución del hilo listener y cierra el archivo `.aof`.
    pub fn shutdown(&self) {
        self.sender.send(LogType::Shutdown).unwrap();
    }

    /// Obtiene el nivel actual de logging.
    ///
    /// # Returns
    ///
    /// `i64` - Nivel actual de logging
    pub fn get_level(&self) -> i64 {
        self.level
    }

    /// Obtiene el rol actual del nodo.
    ///
    /// # Returns
    ///
    /// `&String` - Rol actual del nodo
    pub fn get_role(&self) -> &String {
        &self.role
    }

    /// Verifica si el logger está configurado para un nivel específico.
    ///
    /// # Arguments
    ///
    /// * `level` - Nivel a verificar
    ///
    /// # Returns
    ///
    /// `bool` - True si el logger está configurado para el nivel o superior
    pub fn is_level_enabled(&self, level: i64) -> bool {
        self.level >= level
    }
}

/// Establece el nivel de logging basado en el string de configuración.
///
/// # Arguments
///
/// * `level` - String que representa el nivel de logging
///
/// # Returns
///
/// `i64` - Código numérico del nivel de logging
pub fn set_level(level: String) -> i64 {
    match level.as_str() {
        "warning" => WARNING,
        "notice" => NOTICE,
        "verbose" => VERBOSE,
        "debug" => DEBUG,
        _ => WARNING,
    }
}

/// Función auxiliar, para abrir el file en append mode
/// o crearlo si no existe.
pub fn create_append_log_file(logfile: String) -> File {
    OpenOptions::new()
        .append(true)
        .create(true)
        .open(logfile)
        .unwrap()
}

/// Función auxuliar que procesa el dato recibido por el canal de logs,
/// verifica el nivel y loggea si el nivel es igual o mayor al tipo de log.
pub fn process_log(rec_log: LogType, level: i64, writer: &mut BufWriter<File>) {
    let should_log = match rec_log {
        LogType::Warn(_, _) | LogType::Error(_, _) if level >= WARNING => true,
        LogType::Notice(_, _) if level >= NOTICE => true,
        LogType::RegEvent(_, _) if level >= VERBOSE => true,
        LogType::Debug(_, _) if level >= DEBUG => true,
        _ => false,
    };
    if !should_log {
        return;
    }
    let msg = rec_log.get_log_msg();
    if let Some(msg) = msg {
        writeln!(writer, "{}", msg).unwrap();
        writer.flush().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    /// Función helper para crear una configuración de test
    fn create_test_config() -> NodeConfigs {
        // Crear un archivo de configuración temporal para testing
        let temp_file = NamedTempFile::new().unwrap();
        let config_content = r#"
            bind 0.0.0.0
            port 6379
            role M
            maxclients 1000
            save 900 15
            dbfilename dump.rdb
            dir ./
            logfile test.log
            loglevel notice
            node-id test123
            hash-slots 0-16383
            "#;
        std::fs::write(temp_file.path(), config_content).unwrap();

        // Crear la configuración desde el archivo temporal
        NodeConfigs::new(temp_file.path().to_string_lossy().as_ref()).unwrap()
    }

    #[test]
    fn test_set_level() {
        assert_eq!(set_level("warning".to_string()), WARNING);
        assert_eq!(set_level("notice".to_string()), NOTICE);
        assert_eq!(set_level("verbose".to_string()), VERBOSE);
        assert_eq!(set_level("debug".to_string()), DEBUG);
        assert_eq!(set_level("invalid".to_string()), WARNING); // Default case
        assert_eq!(set_level("".to_string()), WARNING); // Empty string
    }

    #[test]
    fn test_create_append_log_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let logfile = temp_file.path().to_string_lossy().to_string();

        let file = create_append_log_file(logfile.clone());
        assert!(file.metadata().is_ok());

        // Verificar que el archivo existe
        assert!(std::path::Path::new(&logfile).exists());
    }

    #[test]
    fn test_process_log_with_warning_level() {
        let temp_file = NamedTempFile::new().unwrap();
        let logfile = temp_file.path().to_string_lossy().to_string();
        let file = create_append_log_file(logfile.clone());
        let mut writer = BufWriter::new(file);

        // Test con nivel WARNING (0) - solo errores y warnings se escriben
        let error_log = LogType::Error("Test error".to_string(), "M".to_string());
        process_log(error_log, WARNING, &mut writer);

        // Verificar que se escribió algo al archivo
        drop(writer); // Flush the writer
        let content = std::fs::read_to_string(&logfile).unwrap();
        assert!(!content.is_empty());
    }

    #[test]
    fn test_process_log_with_debug_level() {
        let temp_file = NamedTempFile::new().unwrap();
        let logfile = temp_file.path().to_string_lossy().to_string();
        let file = create_append_log_file(logfile.clone());
        let mut writer = BufWriter::new(file);

        // Test con nivel DEBUG (3)
        let debug_log = LogType::Debug("Test debug".to_string(), "M".to_string());
        process_log(debug_log, DEBUG, &mut writer);

        // Verificar que se escribió algo al archivo
        drop(writer); // Flush the writer
        let content = std::fs::read_to_string(&logfile).unwrap();
        assert!(!content.is_empty());
    }

    #[test]
    fn test_process_log_level_filtering() {
        let temp_file = NamedTempFile::new().unwrap();
        let logfile = temp_file.path().to_string_lossy().to_string();
        let file = create_append_log_file(logfile.clone());
        let mut writer = BufWriter::new(file);

        // Con nivel NOTICE (1), los logs DEBUG no deberían escribirse
        let debug_log = LogType::Debug("Test debug".to_string(), "M".to_string());
        process_log(debug_log, NOTICE, &mut writer);

        // Verificar que NO se escribió nada al archivo
        drop(writer); // Flush the writer
        let content = std::fs::read_to_string(&logfile).unwrap();
        assert!(content.is_empty());
    }

    #[test]
    fn test_process_log_shutdown() {
        let temp_file = NamedTempFile::new().unwrap();
        let logfile = temp_file.path().to_string_lossy().to_string();
        let file = create_append_log_file(logfile.clone());
        let mut writer = BufWriter::new(file);

        // Shutdown no debería escribir nada
        let shutdown_log = LogType::Shutdown;
        process_log(shutdown_log, DEBUG, &mut writer);

        // Verificar que NO se escribió nada al archivo
        drop(writer); // Flush the writer
        let content = std::fs::read_to_string(&logfile).unwrap();
        assert!(content.is_empty());
    }

    #[test]
    fn test_aof_logger_getters() {
        let config = create_test_config();
        let logger = AofLogger::new(config);

        assert_eq!(logger.get_level(), NOTICE);
        assert_eq!(logger.get_role(), "M");
        assert!(logger.is_level_enabled(NOTICE));
        assert!(logger.is_level_enabled(WARNING));
        assert!(!logger.is_level_enabled(DEBUG));
    }

    #[test]
    fn test_aof_logger_set_level() {
        let config = create_test_config();
        let mut logger = AofLogger::new(config);
        Arc::get_mut(&mut logger).unwrap().set_level(DEBUG);

        assert_eq!(logger.get_level(), DEBUG);
        assert!(logger.is_level_enabled(DEBUG));
    }

    #[test]
    fn test_aof_logger_set_role() {
        let config = create_test_config();
        let mut logger = AofLogger::new(config);
        Arc::get_mut(&mut logger).unwrap().set_role("S");

        assert_eq!(logger.get_role(), "S");
    }

    #[test]
    fn test_aof_logger_clone() {
        let config = create_test_config();
        let logger = AofLogger::new(config);
        let cloned = logger.clone();

        assert_eq!(logger.get_level(), cloned.get_level());
        assert_eq!(logger.get_role(), cloned.get_role());
    }

    #[test]
    fn test_aof_logger_debug_trait() {
        let config = create_test_config();
        let logger = AofLogger::new(config);
        let debug_str = format!("{:?}", logger);

        assert!(debug_str.contains("AofLogger"));
        assert!(debug_str.contains("M"));
    }

    #[test]
    fn test_log_level_constants() {
        assert_eq!(WARNING, 0);
        assert_eq!(NOTICE, 1);
        assert_eq!(VERBOSE, 2);
        assert_eq!(DEBUG, 3);

        // Verificar que los niveles están en orden ascendente
        assert!(WARNING < NOTICE);
        assert!(NOTICE < VERBOSE);
        assert!(VERBOSE < DEBUG);
    }

    #[test]
    fn test_process_log_all_levels() {
        let temp_file = NamedTempFile::new().unwrap();
        let logfile = temp_file.path().to_string_lossy().to_string();
        let file = create_append_log_file(logfile.clone());
        let mut writer = BufWriter::new(file);

        // Test todos los tipos de log con nivel DEBUG
        let logs = vec![
            LogType::Notice("Test notice".to_string(), "M".to_string()),
            LogType::Error("Test error".to_string(), "M".to_string()),
            LogType::Debug("Test debug".to_string(), "M".to_string()),
            LogType::RegEvent("Test event".to_string(), "M".to_string()),
            LogType::Warn("Test warning".to_string(), "M".to_string()),
        ];

        for log in logs {
            process_log(log, DEBUG, &mut writer);
        }

        // Verificar que se escribió algo al archivo
        drop(writer); // Flush the writer
        let content = std::fs::read_to_string(&logfile).unwrap();
        assert!(!content.is_empty());

        // Verificar que hay múltiples líneas (una por cada log)
        let lines: Vec<&str> = content.lines().collect();
        assert!(lines.len() >= 5);
    }
}
