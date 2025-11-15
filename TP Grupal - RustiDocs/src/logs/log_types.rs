//! Tipos y funciones relacionadas con la generación de mensajes para logs.
//!
//! Ejemplo de Redis log: ``1533:M 08 May 2025 21:20:45.746 * RDB age 15734 seconds``.
//!
//! - `1533`, Número de proceso que inicia la escritura.
//! - `M`, Rol del proceso (Master, Slave, Sentinel, etc.).
//! - `08 May 2025`, Fecha de la escritura.
//! - `21:20:45.746`, Hora de la escritura.
//! - `*`, Nivel del log (DEBUG, VERBOSE, NOTICE, WARNING)
//!     - `-` Eventos generales y Error,
//!     - `#` Warning; Problema sin detener servicio,
//!     - `*` Notificaciones importantes,
//!     - `=` Debug logs.
//! - `RDB age 15734`, Información sobre el log triggereado.
//!
//! **Levels:** warning < notice < verbose < debug

// IMPORTS
use chrono::Local;

// CÓDIGO

/// Formatea el mensaje de log
#[derive(Debug, Clone, PartialEq)]
pub enum LogType {
    Notice(String, String),
    Error(String, String),
    Debug(String, String),
    RegEvent(String, String),
    Warn(String, String),
    Shutdown,
}

/// Función auxiliar para obtener el timestamp de los logs.
fn get_date() -> String {
    let curr_time = Local::now();
    let msg_time = format!("{}", curr_time.format("%d %b %Y %H:%M:%S%.3f"));
    msg_time
}

impl LogType {
    /// Genera los mensajes de logs respetando el tipo de log añadiendo
    /// el PID generador, y el momento exacto de loggeo al mismo.
    ///
    /// # Returns
    /// * Mensaje tipo `String` respestando formato de Redis.
    /// `PID:ROLE DATE TYPE MESSAGE`
    /// * `None` si el tipo de log es `Shutdown`.`
    pub fn get_log_msg(self) -> Option<String> {
        let date = get_date();
        let pid = std::process::id();
        let format_log = |symbol: &str, msg: String, role: String| {
            Some(format!("{}:{} {} {} {}", pid, role, date, symbol, msg))
        };
        match self {
            LogType::Notice(msg, role) => format_log("*", msg, role),
            LogType::Error(msg, role) => format_log("-", msg, role),
            LogType::Debug(msg, role) => format_log(".", msg, role),
            LogType::RegEvent(msg, role) => format_log("-", msg, role),
            LogType::Warn(msg, role) => format_log("#", msg, role),
            LogType::Shutdown => None,
        }
    }

    /// Obtiene el mensaje del log sin el formato completo.
    ///
    /// # Returns
    ///
    /// `Option<String>` - El mensaje del log o None si es Shutdown
    pub fn get_message(&self) -> Option<String> {
        match self {
            LogType::Notice(msg, _) => Some(msg.clone()),
            LogType::Error(msg, _) => Some(msg.clone()),
            LogType::Debug(msg, _) => Some(msg.clone()),
            LogType::RegEvent(msg, _) => Some(msg.clone()),
            LogType::Warn(msg, _) => Some(msg.clone()),
            LogType::Shutdown => None,
        }
    }

    /// Obtiene el rol del log.
    ///
    /// # Returns
    ///
    /// `Option<String>` - El rol del log o None si es Shutdown
    pub fn get_role(&self) -> Option<String> {
        match self {
            LogType::Notice(_, role) => Some(role.clone()),
            LogType::Error(_, role) => Some(role.clone()),
            LogType::Debug(_, role) => Some(role.clone()),
            LogType::RegEvent(_, role) => Some(role.clone()),
            LogType::Warn(_, role) => Some(role.clone()),
            LogType::Shutdown => None,
        }
    }

    /// Verifica si el log es de tipo Shutdown.
    ///
    /// # Returns
    ///
    /// `bool` - True si es Shutdown, False en caso contrario
    pub fn is_shutdown(&self) -> bool {
        matches!(self, LogType::Shutdown)
    }

    /// Obtiene el símbolo del tipo de log.
    ///
    /// # Returns
    ///
    /// `Option<&'static str>` - El símbolo del tipo de log o None si es Shutdown
    pub fn get_symbol(&self) -> Option<&'static str> {
        match self {
            LogType::Notice(_, _) => Some("*"),
            LogType::Error(_, _) => Some("-"),
            LogType::Debug(_, _) => Some("."),
            LogType::RegEvent(_, _) => Some("-"),
            LogType::Warn(_, _) => Some("#"),
            LogType::Shutdown => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_type_notice() {
        let log = LogType::Notice("Test message".to_string(), "M".to_string());
        let msg = log.get_log_msg().unwrap();

        assert!(msg.contains("Test message"));
        assert!(msg.contains("M"));
        assert!(msg.contains("*"));
        assert!(msg.contains(&std::process::id().to_string()));
    }

    #[test]
    fn test_log_type_error() {
        let log = LogType::Error("Error message".to_string(), "S".to_string());
        let msg = log.get_log_msg().unwrap();

        assert!(msg.contains("Error message"));
        assert!(msg.contains("S"));
        assert!(msg.contains("-"));
        assert!(msg.contains(&std::process::id().to_string()));
    }

    #[test]
    fn test_log_type_debug() {
        let log = LogType::Debug("Debug message".to_string(), "M".to_string());
        let msg = log.get_log_msg().unwrap();

        assert!(msg.contains("Debug message"));
        assert!(msg.contains("M"));
        assert!(msg.contains("."));
        assert!(msg.contains(&std::process::id().to_string()));
    }

    #[test]
    fn test_log_type_reg_event() {
        let log = LogType::RegEvent("Event message".to_string(), "S".to_string());
        let msg = log.get_log_msg().unwrap();

        assert!(msg.contains("Event message"));
        assert!(msg.contains("S"));
        assert!(msg.contains("-"));
        assert!(msg.contains(&std::process::id().to_string()));
    }

    #[test]
    fn test_log_type_warn() {
        let log = LogType::Warn("Warning message".to_string(), "M".to_string());
        let msg = log.get_log_msg().unwrap();

        assert!(msg.contains("Warning message"));
        assert!(msg.contains("M"));
        assert!(msg.contains("#"));
        assert!(msg.contains(&std::process::id().to_string()));
    }

    #[test]
    fn test_log_type_shutdown() {
        let log = LogType::Shutdown;
        let msg = log.get_log_msg();

        assert!(msg.is_none());
    }

    #[test]
    fn test_get_message() {
        let notice = LogType::Notice("Test message".to_string(), "M".to_string());
        let error = LogType::Error("Error message".to_string(), "S".to_string());
        let shutdown = LogType::Shutdown;

        assert_eq!(notice.get_message(), Some("Test message".to_string()));
        assert_eq!(error.get_message(), Some("Error message".to_string()));
        assert_eq!(shutdown.get_message(), None);
    }

    #[test]
    fn test_get_role() {
        let notice = LogType::Notice("Test message".to_string(), "M".to_string());
        let error = LogType::Error("Error message".to_string(), "S".to_string());
        let shutdown = LogType::Shutdown;

        assert_eq!(notice.get_role(), Some("M".to_string()));
        assert_eq!(error.get_role(), Some("S".to_string()));
        assert_eq!(shutdown.get_role(), None);
    }

    #[test]
    fn test_is_shutdown() {
        let notice = LogType::Notice("Test message".to_string(), "M".to_string());
        let shutdown = LogType::Shutdown;

        assert!(!notice.is_shutdown());
        assert!(shutdown.is_shutdown());
    }

    #[test]
    fn test_get_symbol() {
        let notice = LogType::Notice("Test message".to_string(), "M".to_string());
        let error = LogType::Error("Error message".to_string(), "S".to_string());
        let debug = LogType::Debug("Debug message".to_string(), "M".to_string());
        let reg_event = LogType::RegEvent("Event message".to_string(), "S".to_string());
        let warn = LogType::Warn("Warning message".to_string(), "M".to_string());
        let shutdown = LogType::Shutdown;

        assert_eq!(notice.get_symbol(), Some("*"));
        assert_eq!(error.get_symbol(), Some("-"));
        assert_eq!(debug.get_symbol(), Some("."));
        assert_eq!(reg_event.get_symbol(), Some("-"));
        assert_eq!(warn.get_symbol(), Some("#"));
        assert_eq!(shutdown.get_symbol(), None);
    }

    #[test]
    fn test_log_type_clone() {
        let log = LogType::Notice("Test message".to_string(), "M".to_string());
        let cloned = log.clone();

        assert_eq!(log, cloned);
    }

    #[test]
    fn test_log_type_debug_trait() {
        let log = LogType::Notice("Test message".to_string(), "M".to_string());
        let debug_str = format!("{:?}", log);

        assert!(debug_str.contains("Notice"));
        assert!(debug_str.contains("Test message"));
        assert!(debug_str.contains("M"));
    }

    #[test]
    fn test_log_type_partial_eq() {
        let log1 = LogType::Notice("Test message".to_string(), "M".to_string());
        let log2 = LogType::Notice("Test message".to_string(), "M".to_string());
        let log3 = LogType::Notice("Different message".to_string(), "M".to_string());
        let log4 = LogType::Error("Test message".to_string(), "M".to_string());

        assert_eq!(log1, log2);
        assert_ne!(log1, log3);
        assert_ne!(log1, log4);
    }

    #[test]
    fn test_get_date_format() {
        let date = get_date();

        // Verificar que la fecha tiene el formato correcto
        // Debería ser algo como "08 May 2025 21:20:45.746"
        assert!(date.contains(" "));
        assert!(date.matches(" ").count() >= 3); // Al menos 3 espacios para separar día, mes, año, hora

        // Verificar que contiene números y letras
        assert!(date.chars().any(|c| c.is_ascii_digit()));
        assert!(date.chars().any(|c| c.is_ascii_alphabetic()));
    }

    #[test]
    fn test_log_message_format() {
        let log = LogType::Notice("Test message".to_string(), "M".to_string());
        let msg = log.get_log_msg().unwrap();

        // Verificar que el mensaje contiene todos los elementos necesarios
        assert!(msg.contains("M")); // Role
        assert!(msg.contains("*")); // Symbol
        assert!(msg.contains("Test message")); // Message
        assert!(msg.contains(&std::process::id().to_string())); // PID

        // Verificar que tiene el formato básico PID:ROLE DATE SYMBOL MESSAGE
        let parts: Vec<&str> = msg.split(' ').collect();
        assert!(parts.len() >= 5); // Al menos 5 partes

        // Verificar que la primera parte contiene PID:ROLE
        assert!(parts[0].contains(':'));
        let pid_role: Vec<&str> = parts[0].split(':').collect();
        assert_eq!(pid_role.len(), 2);
        assert!(pid_role[0].parse::<u32>().is_ok()); // PID debe ser un número
        assert_eq!(pid_role[1], "M"); // Role debe ser M

        // Verificar que el símbolo está presente en alguna parte
        assert!(msg.contains("*"));
    }
}
