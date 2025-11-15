pub const MASTER: u8 = 0b0000_0001; // 1
pub const SLAVE: u8 = 0b0000_0010; // 2
pub const PFAIL: u8 = 0b0000_0100; // 4
pub const FAIL: u8 = 0b0000_1000; // 8
pub const HANDSHAKE: u8 = 0b0001_0000; // 16
pub const NOADDR: u8 = 0b0010_0000; // 32 Mío, no se comparte en gossips
pub const ME: u8 = 0b0100_0000; // 64
pub const CONNECTED: u8 = 0b1000_0000; // 128

/// Flags de los nodos del cluster, incluye:
/// - MASTER;
/// - SLAVE, nodo réplica de un master;
/// - CONNECTED, nodo que supero la etapa conocido o de handshake;
/// - PFAIL, nodo posiblemente fallado;
/// - FAIL, nodo definitivamente perdido;
/// - HANDSHAKE, nodo que inicio comunicación por JOIN, sin goce de conexión plena;
/// - NOADDR, nodo que se conoce por gossip, pero nunca se trató directamente; Y
/// - ME, flag para indicar que ese nodo soy yo mismo.
#[derive(Debug, Clone)]
pub struct NodeFlags(u8);

impl NodeFlags {
    /// Devuelve un nuevo conjunto con todas las get_flags desactivadas.
    pub fn new() -> Self {
        NodeFlags(0)
    }

    pub fn get_state(&self) -> u8 {
        self.0
    }

    /// Setea una flag comoa activa.
    pub fn set(&mut self, flag: u8) {
        self.0 |= flag;
        match flag {
            CONNECTED => {
                self.unset(PFAIL);
                self.unset(FAIL);
                self.unset(HANDSHAKE);
                self.unset(NOADDR);
            }
            PFAIL => {
                self.unset(FAIL);
                self.unset(CONNECTED);
            }
            FAIL => {
                self.unset(PFAIL);
                self.unset(CONNECTED);
                self.unset(HANDSHAKE);
            }
            MASTER => self.unset(SLAVE),
            SLAVE => self.unset(MASTER),
            HANDSHAKE => self.unset(NOADDR),
            NOADDR => self.unset(HANDSHAKE),
            _ => {}
        }
    }

    /// Desactiva una flag.
    pub fn unset(&mut self, flag: u8) {
        self.0 &= !flag;
    }

    /// Comprueba si una flag está activa.
    pub fn is_set(&self, flag: u8) -> bool {
        (self.0 & flag) != 0
    }

    /// Borra todas las get_flags.
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    /// Útil para el gossip, durante el intercambio de información,
    /// los nodos solo se enteran si el nodo en cuestión es MASTER/SLAVE,
    /// si está PFAIL/FAIL/CONNECTED.
    ///
    /// Los campos HANDSHAKE/NOADDR/ME **no se comparten**, ya que son un
    /// POV de cada nodo en sí.
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut aux = NodeFlags::new();
        if self.is_set(PFAIL) {
            aux.set(PFAIL);
        } else if self.is_set(FAIL) {
            aux.set(FAIL);
        } else if self.is_set(CONNECTED) {
            aux.set(CONNECTED);
        }
        if self.is_set(MASTER) {
            aux.set(MASTER);
        } else if self.is_set(SLAVE) {
            aux.set(SLAVE);
        }

        bytes.extend_from_slice(&aux.get_state().to_be_bytes());
        bytes
    }

    pub fn print(&self) -> String {
        let myself = if self.is_set(ME) {
            "\x1b[36mME   \x1b[0m"
        } else {
            "KNOWN"
        };
        let role = if self.is_set(MASTER) {
            "\x1b[32mMASTER\x1b[0m"
        } else {
            "\x1b[34mSLAVE \x1b[0m"
        };
        let state = if self.is_set(CONNECTED) {
            "ALIVE"
        } else if self.is_set(PFAIL) {
            "\x1b[33mPFAIL\x1b[0m"
        } else {
            "\x1b[31mFAIL \x1b[0m"
        };
        let noaddr = if self.is_set(NOADDR) {
            "NOADDR   "
        } else if self.is_set(HANDSHAKE) {
            "HANDSHAKE"
        } else {
            "CONNECTED"
        };
        format!("({},{},{},{})", myself, role, state, noaddr)
    }

    pub fn state_contains(state: u8, flag: u8) -> bool {
        (state & flag) == flag
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_flags() {
        let mut flags = NodeFlags::new();
        assert!(!flags.is_set(MASTER));

        flags.set(MASTER);
        assert!(flags.is_set(MASTER));
    }

    #[test]
    fn test_unset_flags() {
        let mut flags = NodeFlags::new();

        flags.set(FAIL);
        assert!(flags.is_set(FAIL));

        flags.unset(FAIL);
        assert!(!flags.is_set(FAIL));
    }
}
