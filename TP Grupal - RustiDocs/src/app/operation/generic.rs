/// Instruccion generica que guarda metadatos, siendo el id de la operacion,
/// la version base y una operacion generica que idealmente deberia implementar los traits
/// de Transformable y Applicable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction<T>
where
    T: ParsableBytes,
{
    pub operation_id: InstructionId,
    pub base_version: u64,
    pub operation: T,
}

/// Identificador de una instruction.
/// El identificador unico se corresponde al id del cliente emisor
/// y el id local de la operacion dentro del cliente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionId {
    pub client_id: u64,
    pub local_seq: u64,
}

impl ParsableBytes for InstructionId {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(self.client_id.to_le_bytes());
        bytes.extend(self.local_seq.to_le_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<(Self, usize)> {
        if bytes.len() < 16 {
            return None; // Necesitamos al menos 16 bytes (8 para cada campo)
        }
        let client_id = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let local_seq = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
        Some((
            InstructionId {
                client_id,
                local_seq,
            },
            16,
        ))
    }
}

impl<T> ParsableBytes for Instruction<T>
where
    T: ParsableBytes,
{
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(self.operation_id.to_bytes());
        bytes.extend(self.base_version.to_le_bytes());
        bytes.extend(self.operation.to_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<(Self, usize)> {
        let (operation_id, offset) = InstructionId::from_bytes(bytes)?;
        if bytes.len() < offset + 8 {
            return None; // Necesitamos al menos 8 bytes para la base_version
        }
        let base_version = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        let (operation, op_size) = T::from_bytes(&bytes[offset + 8..])?;
        Some((
            Instruction {
                operation_id,
                base_version,
                operation,
            },
            offset + 8 + op_size,
        ))
    }
}

/// Trait que transforma una operacion teniendo en cuenta otra operacion
/// y devuelve una nueva operacion.
pub trait Transformable {
    fn transform(&self, other: &Self) -> Self;
}

/// Trait que aplica una operacion a un dato.
/// Este trait es generico y puede ser implementado para cualquier tipo de dato.
/// Por ejemplo, una operacion que inserte texto en un documento de texto
/// o modifique una cuadricula en una planilla de calculo.
pub trait Applicable<D> {
    fn apply(&self, data: &mut D);
}

// ESTE TRAIT HAY QUE MOVERLO A UNA JERARQUIA MAS GENERAL PORQUE ES USADA POR CLIENT_LIB
pub trait ParsableBytes: Sized {
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Option<(Self, usize)>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct DummyOperation(u32);

    impl ParsableBytes for DummyOperation {
        fn to_bytes(&self) -> Vec<u8> {
            self.0.to_le_bytes().to_vec()
        }

        fn from_bytes(bytes: &[u8]) -> Option<(Self, usize)> {
            if bytes.len() < 4 {
                return None;
            }
            let val = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
            Some((DummyOperation(val), 4))
        }
    }

    #[test]
    fn test_instruction_id_to_bytes_and_from_bytes() {
        let id = InstructionId {
            client_id: 123,
            local_seq: 456,
        };
        let bytes = id.to_bytes();
        let (parsed, size) = InstructionId::from_bytes(&bytes).unwrap();
        assert_eq!(parsed, id);
        assert_eq!(size, 16);
    }

    #[test]
    fn test_instruction_id_from_bytes_too_short() {
        let bytes = vec![0u8; 10];
        assert!(InstructionId::from_bytes(&bytes).is_none());
    }

    #[test]
    fn test_instruction_to_bytes_and_from_bytes() {
        let id = InstructionId {
            client_id: 1,
            local_seq: 2,
        };
        let op = DummyOperation(42);
        let instr = Instruction {
            operation_id: id.clone(),
            base_version: 99,
            operation: op.clone(),
        };
        let bytes = instr.to_bytes();
        let (parsed, size) = Instruction::<DummyOperation>::from_bytes(&bytes).unwrap();
        assert_eq!(parsed, instr);
        // 16 bytes para InstructionId, 8 para base_version, 4 para DummyOperation
        assert_eq!(size, 28);
    }

    #[test]
    fn test_instruction_from_bytes_too_short_for_base_version() {
        let id = InstructionId {
            client_id: 1,
            local_seq: 2,
        };
        let mut bytes = id.to_bytes();
        // Faltan bytes para base_version (necesita 8)
        bytes.extend([0u8; 4]);
        assert!(Instruction::<DummyOperation>::from_bytes(&bytes).is_none());
    }

    #[test]
    fn test_instruction_from_bytes_too_short_for_operation() {
        let id = InstructionId {
            client_id: 1,
            local_seq: 2,
        };
        let mut bytes = id.to_bytes();
        bytes.extend(99u64.to_le_bytes());
        // Faltan bytes para DummyOperation (necesita 4)
        bytes.extend([0u8; 2]);
        assert!(Instruction::<DummyOperation>::from_bytes(&bytes).is_none());
    }

    #[test]
    fn test_instruction_with_zero_values() {
        let id = InstructionId {
            client_id: 0,
            local_seq: 0,
        };
        let op = DummyOperation(0);
        let instr = Instruction {
            operation_id: id,
            base_version: 0,
            operation: op,
        };
        let bytes = instr.to_bytes();
        let (parsed, size) = Instruction::<DummyOperation>::from_bytes(&bytes).unwrap();
        assert_eq!(parsed, instr);
        assert_eq!(size, 28);
    }
}
