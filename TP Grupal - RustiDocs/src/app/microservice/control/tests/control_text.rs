#[cfg(test)]
mod tests {

    use crate::app::microservice::control::control_service::ControlService;
    use crate::app::operation::generic::Instruction;
    use crate::app::operation::generic::InstructionId;
    use crate::app::operation::text::TextOperation;

    fn new_instruction<O: crate::app::operation::generic::ParsableBytes>(
        client_id: u64,
        local_seq: u64,
        base_version: u64,
        operation: O,
    ) -> Instruction<O> {
        Instruction {
            operation_id: InstructionId {
                client_id,
                local_seq,
            },
            base_version,
            operation,
        }
    }

    #[test]
    fn test_simple_insert() {
        let mut engine = ControlService::<String, TextOperation>::new(String::new());

        let instr = new_instruction(
            1,
            1,
            0,
            TextOperation::Insert {
                position: 0,
                character: 'H',
            },
        );

        let result = engine.apply_operation(instr).unwrap();

        assert_eq!(engine.data, "H");
        assert_eq!(engine.version, 1);
        assert_eq!(engine.operations_log.len(), 1);
        assert_eq!(result.base_version, 1);
    }

    #[test]
    fn test_simple_sequence() {
        let mut engine = ControlService::<String, TextOperation>::new(String::new());

        let instr1 = new_instruction(
            1,
            1,
            0,
            TextOperation::Insert {
                position: 0,
                character: 'H',
            },
        );
        let instr2 = new_instruction(
            1,
            2,
            1,
            TextOperation::Insert {
                position: 1,
                character: 'i',
            },
        );

        engine.apply_operation(instr1).unwrap();
        engine.apply_operation(instr2).unwrap();

        assert_eq!(engine.data, "Hi");
        assert_eq!(engine.version, 2);
    }

    #[test]
    fn test_concurrent_insert_insert() {
        let mut engine = ControlService::<String, TextOperation>::new(String::new());

        // Cliente 1 inserta H
        let instr1 = new_instruction(
            1,
            1,
            0,
            TextOperation::Insert {
                position: 0,
                character: 'H',
            },
        );
        engine.apply_operation(instr1).unwrap(); // Ahora el engine tiene "H"

        // Cliente 2 envía operación concurrente generada desde base_version 0
        let instr2 = new_instruction(
            2,
            1,
            0,
            TextOperation::Insert {
                position: 0,
                character: 'A',
            },
        );

        let result2 = engine.apply_operation(instr2).unwrap();

        assert_eq!(engine.data, "HA");
        assert_eq!(engine.version, 2);
        assert_eq!(result2.base_version, 2);

        // Cliente 3 envía operación concurrente generada desde base_version 0
        let instr3 = new_instruction(
            3,
            1,
            0,
            TextOperation::Insert {
                position: 0,
                character: 'L',
            },
        );

        let result3 = engine.apply_operation(instr3).unwrap();

        assert_eq!(engine.data, "HAL");
        assert_eq!(engine.version, 3);
        assert_eq!(result3.base_version, 3);
    }

    #[test]
    fn test_concurrent_insert_delete() {
        let mut engine = ControlService::<String, TextOperation>::new("Hi".to_string());

        // Cliente 1 elimina la 'i' en posición 1
        let instr1 = new_instruction(1, 1, 0, TextOperation::Delete { position: 1 });
        engine.apply_operation(instr1).unwrap();

        // Cliente 2, que estaba desactualizado (base_version 0), inserta en posición 1
        let instr2 = new_instruction(
            2,
            1,
            0,
            TextOperation::Insert {
                position: 1,
                character: '!',
            },
        );

        let result2 = engine.apply_operation(instr2).unwrap();

        assert_eq!(engine.data, "H!");
        assert_eq!(engine.version, 2);
        assert_eq!(result2.base_version, 2);
    }

    #[test]
    fn test_invalid_base_version() {
        let mut engine = ControlService::<String, TextOperation>::new("Hello".to_string());

        let instr = new_instruction(
            1,
            1,
            10, // base_version futura
            TextOperation::Insert {
                position: 0,
                character: 'X',
            },
        );

        let result = engine.apply_operation(instr);
        assert!(result.is_err());
    }
}
