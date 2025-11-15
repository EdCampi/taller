use crate::app::operation::generic::Applicable;
use crate::app::operation::generic::Instruction;
use crate::app::operation::generic::ParsableBytes;
use crate::app::operation::generic::Transformable;

/// Modulo de control generico, recibe la estructura de datos
/// y las operaciones que se van a aplicar sobre ella.
/// D es la estructura de datos que se va a modificar.
/// O es la operacion que se va a aplicar sobre D.
/// O debe poder transformarse a si misma.
/// O debe poder aplicarse a D.
///
/// Ejemplos:
/// Procesador de texto: D es string y O es una operacion de texto (insertar o eliminar un caracter).
/// Planilla de calculo: D es una matriz y O es una operacion de planilla (insertar o eliminar una celda).
///

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlService<D, O>
where
    O: Applicable<D> + Transformable + Clone + ParsableBytes,
{
    pub data: D,
    pub operations_log: Vec<Instruction<O>>,
    pub version: u64,
}

impl<D, O> ControlService<D, O>
where
    O: Applicable<D> + Transformable + Clone + ParsableBytes,
{
    pub fn new(data: D) -> Self {
        ControlService {
            data,
            operations_log: Vec::new(),
            version: 0,
        }
    }
    // Dada una instruccion en bruto, se la transforma a la ultima version
    // de la operacion, aplicando las operaciones que faltan desde la version base
    // hasta la version actual del servicio de control.
    // Si la version base es mayor a la version actual, se devuelve un error.
    // Si la operacion es transformada correctamente, se aplica a los datos
    // y se devuelve la instruccion transformada.
    pub fn apply_operation(
        &mut self,
        mut instruction: Instruction<O>,
    ) -> Result<Instruction<O>, ControlServiceError> {
        // En caso de que la version base sea mayor a la version actual,
        // se desbordaria el u64, por lo que es mayor a la version actual y se devuelve error.
        if instruction.base_version > self.version {
            return Err(ControlServiceError::VersionHigherThanCurrent);
        }

        // Si la version base es menor a la version actual, se transforma la operacion
        // teniendo en cuenta las operaciones que ya se aplicaron.
        if instruction.base_version != self.version {
            // Aplico cada operacion faltante desde la version base hasta la version actual.
            for operation_history in self
                .operations_log
                .iter()
                .skip(instruction.base_version as usize)
            {
                instruction.operation = instruction
                    .operation
                    .transform(&operation_history.operation);
            }
        }

        // Aplico la operacion transformada a los datos.
        instruction.operation.apply(&mut self.data);

        // Actualizo la version del servicio de control.
        self.version += 1;

        // Agrego la instruccion al log de operaciones.
        self.operations_log.push(instruction.clone());

        // Actualizo la version base de la instruccion a la version actual del servicio de control.
        instruction.base_version = self.version;

        // Devuelvo la instruccion transformada.
        Ok(instruction)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlServiceError {
    VersionHigherThanCurrent,
}
