#[derive(Clone, PartialEq, Debug)]
pub struct Permissions {
    autorized_instructions: Vec<String>,
}

impl Permissions {
    pub fn new() -> Self {
        Self {
            autorized_instructions: Vec::new(),
        }
    }

    pub fn is_permited(&self, instruction: &str) -> bool {
        self.autorized_instructions
            .contains(&instruction.to_string())
    }

    pub fn is_read_only(&self) -> bool {
        if self.autorized_instructions.is_empty()
            || !self.autorized_instructions.contains(&"SET".to_string())
        {
            return true;
        }
        false
    }

    pub fn add_instruction(&mut self, instruction: String) {
        self.autorized_instructions.push(instruction);
    }

    /// Declara al usuario como super usuario, con acceso a todas las
    /// instrucciones que existen
    pub fn set_super(&mut self) {
        // String commands
        self.autorized_instructions.push("APPEND".to_string());
        self.autorized_instructions.push("ECHO".to_string());
        self.autorized_instructions.push("GET".to_string());
        self.autorized_instructions.push("GETDEL".to_string());
        self.autorized_instructions.push("GETRANGE".to_string());
        self.autorized_instructions.push("SET".to_string());
        self.autorized_instructions.push("STRLEN".to_string());
        self.autorized_instructions.push("SUBSTR".to_string());

        // List commands
        self.autorized_instructions.push("DEL".to_string());
        self.autorized_instructions.push("LLEN".to_string());
        self.autorized_instructions.push("LPOP".to_string());
        self.autorized_instructions.push("LPUSH".to_string());
        self.autorized_instructions.push("LRANGE".to_string());
        self.autorized_instructions.push("RPOP".to_string());
        self.autorized_instructions.push("RPUSH".to_string());

        // Set commands
        self.autorized_instructions.push("SADD".to_string());
        self.autorized_instructions.push("SCARD".to_string());
        self.autorized_instructions.push("SISMEMBER".to_string());
        self.autorized_instructions.push("SMEMBERS".to_string());
        self.autorized_instructions.push("SMOVE".to_string());
        self.autorized_instructions.push("SPOP".to_string());

        // Database commands
        self.autorized_instructions.push("BGSAVE".to_string());
        self.autorized_instructions.push("SAVE".to_string());

        // PubSub commands
        self.autorized_instructions.push("SUBSCRIBE".to_string());
        self.autorized_instructions.push("UNSUBSCRIBE".to_string());
        self.autorized_instructions.push("PUBLISH".to_string());

        // Cluster commands
        self.autorized_instructions.push("MEET".to_string());
        self.autorized_instructions.push("CLUSTER".to_string());
        self.autorized_instructions.push("PING".to_string());
    }
}
