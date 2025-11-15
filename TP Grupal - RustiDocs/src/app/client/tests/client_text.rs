#[cfg(test)]
mod tests {
    use crate::app::client::client_data::Client;
    use crate::app::operation::generic::{Instruction, InstructionId};
    use crate::app::operation::text::TextOperation;

    fn make_op_id(client_id: u64, seq: u64) -> InstructionId {
        InstructionId {
            client_id,
            local_seq: seq,
        }
    }

    #[test]
    fn test_simple_local_insert() {
        let mut client = Client::new_no_output(String::new(), 0, 1);

        client.apply_local_operation(TextOperation::Insert {
            position: 0,
            character: 'H',
        });

        assert_eq!(client.local_data, "H");
        assert_eq!(client.pending_operations.len(), 1);
        assert_eq!(client.local_version, 0);
    }

    #[test]
    fn test_local_insert_with_ack() {
        let mut client = Client::new_no_output(String::new(), 0, 1);

        client.apply_local_operation(TextOperation::Insert {
            position: 0,
            character: 'H',
        });

        // Simula recibir el ack del servidor
        client.receive_remote_instruction(Instruction {
            base_version: 0,
            operation: TextOperation::Insert {
                position: 0,
                character: 'H',
            },
            operation_id: make_op_id(1, 0),
        });

        assert_eq!(client.local_data, "H");
        assert_eq!(client.pending_operations.len(), 0);
        assert_eq!(client.local_version, 1);
    }

    #[test]
    fn test_concurrent_insert_remote() {
        let mut client = Client::new_no_output(String::new(), 0, 1);

        client.apply_local_operation(TextOperation::Insert {
            position: 0,
            character: 'H',
        });

        // Otro cliente hace insert en posición 0
        client.receive_remote_instruction(Instruction {
            base_version: 0,
            operation: TextOperation::Insert {
                position: 0,
                character: 'A',
            },
            operation_id: make_op_id(2, 0),
        });

        // El estado final debería ser: "HA"
        assert_eq!(client.local_data, "HA");
        assert_eq!(client.pending_operations.len(), 1);
    }

    #[test]
    fn test_concurrent_delete_remote() {
        let mut client = Client::new_no_output(String::new(), 0, 1);

        client.apply_local_operation(TextOperation::Insert {
            position: 0,
            character: 'H',
        });

        // Otro cliente borra la posición 0 antes que llegue nuestro insert
        client.receive_remote_instruction(Instruction {
            base_version: 0,
            operation: TextOperation::Delete { position: 0 },
            operation_id: make_op_id(2, 0),
        });

        // El estado local debería seguir siendo "H" porque insertamos después
        assert_eq!(client.local_data, "H");
        assert_eq!(client.pending_operations.len(), 1);
    }

    #[test]
    fn test_remote_insert_before_local() {
        let mut client = Client::new_no_output(String::new(), 0, 1);

        client.apply_local_operation(TextOperation::Insert {
            position: 1,
            character: 'B',
        });

        // Otro cliente inserta antes de nosotros
        client.receive_remote_instruction(Instruction {
            base_version: 0,
            operation: TextOperation::Insert {
                position: 0,
                character: 'A',
            },
            operation_id: make_op_id(2, 0),
        });

        // Debería ajustar nuestra inserción de 'B' a la posición 2
        assert_eq!(client.local_data, "AB");
        assert_eq!(client.pending_operations.len(), 1);
    }
}
