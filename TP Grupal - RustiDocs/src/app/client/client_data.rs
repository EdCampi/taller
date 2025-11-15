use crate::app::operation::generic::Applicable;
use crate::app::operation::generic::Instruction;
use crate::app::operation::generic::InstructionId;
use crate::app::operation::generic::ParsableBytes;
use crate::app::operation::generic::Transformable;
use std::sync::mpsc::Sender;

#[derive(Clone)]
pub struct Client<D, O>
where
    O: Applicable<D> + Transformable + Clone + ParsableBytes,
{
    pub client_id: u64,
    pub local_operation_id: u64, // Contador de operaciones locales
    pub local_data: D,
    pub local_version: u64, // Representa la version local del cliente pero confirmada por el sv
    pub pending_operations: Vec<Instruction<O>>,
    output: Option<Sender<Instruction<O>>>,
}

impl<D, O> Client<D, O>
where
    O: Applicable<D> + Transformable + Clone + ParsableBytes,
{
    pub fn new(
        data: D,
        output: Sender<Instruction<O>>,
        local_version: u64,
        client_id: u64,
    ) -> Self {
        Client {
            client_id,
            local_data: data,
            local_version,
            output: Some(output),
            local_operation_id: 0, // Comienza en 0
            pending_operations: Vec::new(),
        }
    }

    pub fn new_no_output(data: D, local_version: u64, client_id: u64) -> Self {
        Client {
            client_id,
            local_data: data,
            local_version,
            output: None,
            local_operation_id: 0, // Comienza en 0
            pending_operations: Vec::new(),
        }
    }

    pub fn apply_local_operation(&mut self, operation: O) -> Instruction<O> {
        println!("Cliente id {} recibe operacion local", self.client_id);
        operation.apply(&mut self.local_data);
        // No se incrementa la version hasta que el servidor confirme la operacion.

        let instruction = Instruction {
            operation_id: InstructionId {
                client_id: self.client_id,
                local_seq: self.local_operation_id,
            },
            base_version: self.local_version,
            operation,
        };

        self.local_operation_id += 1;

        self.pending_operations.push(instruction.clone());

        if let Some(socket) = &self.output {
            let _ = socket.send(instruction.clone());
        }

        instruction
    }

    pub fn receive_remote_instruction(&mut self, mut remote_instruction: Instruction<O>) {
        println!("Cliente id {} recibe operacion remota", self.client_id);

        // Si la instrucción es del mismo cliente quitamos del pendig operation.
        if remote_instruction.operation_id.client_id == self.client_id {
            self.remove_pending_operation(remote_instruction.operation_id);
            self.local_version += 1;
        } else {
            // Transformar contra las pending operations
            for pending in &self.pending_operations {
                remote_instruction.operation =
                    remote_instruction.operation.transform(&pending.operation);
            }

            // Aplicar al estado local
            remote_instruction.operation.apply(&mut self.local_data);

            self.local_version += 1;

            // Actualizar el pending buffer: transformar las pending contra la nueva operación remota
            for pending in &mut self.pending_operations {
                pending.operation = pending.operation.transform(&remote_instruction.operation);
            }
        }
    }

    pub fn remove_pending_operation(&mut self, instruction_id: InstructionId) {
        for i in 0..self.pending_operations.len() {
            if self.pending_operations[i].operation_id.local_seq == instruction_id.local_seq {
                self.pending_operations.remove(i);
                break;
            }
        }
    }
}
