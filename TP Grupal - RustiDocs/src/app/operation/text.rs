
use crate::app::operation::generic::{Applicable, ParsableBytes, Transformable};

const INSERT: u8 = 0;
const DELETE: u8 = 1;
const NO_OPERATION: u8 = 2;
const DELETE_ALL: u8 = 3;
const DELETE_RANGE: u8 = 4; // Nueva operación atómica para eliminar un rango de texto
const INSERT_TEXT: u8 = 5;    // Nueva operación atómica para insertar texto

impl ParsableBytes for String {
    fn to_bytes(&self) -> Vec<u8> {
        let bytes = self.as_bytes();
        let mut result = Vec::with_capacity(8 + bytes.len());
        let len = bytes.len() as u64;
        result.extend_from_slice(&len.to_le_bytes());
        result.extend_from_slice(bytes);
        result
    }

    fn from_bytes(bytes: &[u8]) -> Option<(Self, usize)> {
        if bytes.len() < 8 {
            return None;
        }
        let len = u64::from_le_bytes(bytes[0..8].try_into().ok()?);
        let len = len as usize;
        if bytes.len() < 8 + len {
            return None;
        }
        let string = String::from_utf8(bytes[8..8 + len].to_vec()).ok()?;
        Some((string, 8 + len))
    }
}

/// Operaciones de texto que pueden ser aplicadas a un documento de texto.
/// Estas operaciones incluyen insertar un caracter en una posicion especifica
/// o eliminar un caracter de una posicion especifica.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextOperation {
    Insert { position: usize, character: char },
    Delete { position: usize },
    DeleteAll,   // Util cuando se quiere eliminar todo el texto
    DeleteRange{ start: usize, end: usize }, // Eliminar un rango de texto
    NoOperation, // Util cuando una operacion no tiene efecto
    InsertText { position: usize, text: String }, // Insertar un string completo en una posición
}

impl Default for TextOperation {
    fn default() -> Self {
        TextOperation::NoOperation
    }
}

impl Applicable<String> for TextOperation {
    fn apply(&self, data: &mut String) {
         match self {
            TextOperation::Insert {
                position,
                character,
            } => {
                let char_indices: Vec<usize> = data.char_indices().map(|(i, _)| i).collect();
                let byte_pos = if *position < char_indices.len() {
                    char_indices[*position]
                } else {
                    data.len()
                };
                data.insert(byte_pos, *character);
            }
            TextOperation::Delete { position } => {
                let char_indices: Vec<usize> = data.char_indices().map(|(i, _)| i).collect();
                if *position < char_indices.len() {
                    data.remove(char_indices[*position]);
                }
            }
            TextOperation::NoOperation => {
                // No hacemos nada, la operación no tiene efecto.
            }
            TextOperation::DeleteAll => {
                // Elimina todo el texto
                data.clear();
            }
        
            TextOperation::DeleteRange { start, end } => {
                let char_indices: Vec<usize> = data.char_indices().map(|(i, _)| i).collect();
                let start_byte = if *start < char_indices.len() {
                    char_indices[*start]
                } else {
                    data.len()
                };
                let end_byte = if *end < char_indices.len() {
                    char_indices[*end]
                } else {
                    data.len()
                };
                data.replace_range(start_byte..end_byte, "");
            }
            TextOperation::InsertText { position, text } => {
                let char_indices: Vec<usize> = data.char_indices().map(|(i, _)| i).collect();
                let byte_pos = if *position < char_indices.len() {
                    char_indices[*position]
                } else {
                    data.len()
                };
                data.insert_str(byte_pos, text);
            }
        }
    }
}

impl ParsableBytes for TextOperation {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        match self {
            TextOperation::Insert {
                position,
                character,
            } => {
                bytes.push(INSERT); // discriminante para Insert
                bytes.extend(&position.to_be_bytes());
                let c = *character as u32;
                bytes.extend(&c.to_be_bytes());
            }
            TextOperation::Delete { position } => {
                bytes.push(DELETE); // discriminante para Delete
                bytes.extend(&position.to_be_bytes());
            }
            TextOperation::NoOperation => {
                bytes.push(NO_OPERATION); // discriminante para NoOperation
            }
            TextOperation::DeleteAll => {
                bytes.push(DELETE_ALL); // discriminante para DeleteAll
            }
    
            TextOperation::InsertText { position, text } => {
                bytes.push(INSERT_TEXT);
                bytes.extend(&position.to_be_bytes());
                let text_bytes = text.to_bytes();
                bytes.extend(text_bytes);
            }
            TextOperation::DeleteRange { start, end } => {
                bytes.push(DELETE_RANGE);
                bytes.extend(&start.to_be_bytes());
                bytes.extend(&end.to_be_bytes());
            }
        }
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<(Self, usize)> {
        if bytes.is_empty() {
            return None;
        }
        match bytes[0] {
            INSERT => {
                // Insert
                if bytes.len() < 1 + 8 + 4 {
                    return None;
                }
                let position = usize::from_be_bytes(bytes[1..9].try_into().ok()?);
                let c = u32::from_be_bytes(bytes[9..13].try_into().ok()?);
                let character = std::char::from_u32(c)?;
                Some((
                    TextOperation::Insert {
                        position,
                        character,
                    },
                    13,
                ))
            }
            DELETE => {
                // Delete
                if bytes.len() < 1 + 8 {
                    return None;
                }
                let position = usize::from_be_bytes(bytes[1..9].try_into().ok()?);
                Some((TextOperation::Delete { position }, 9))
            }
            NO_OPERATION => {
                // NoOperation
                Some((TextOperation::NoOperation, 1))
            }
            DELETE_RANGE => {
                // DeleteRange
                if bytes.len() < 1 + 8 + 8 {
                    return None;
                }
                let start = usize::from_be_bytes(bytes[1..9].try_into().ok()?);
                let end = usize::from_be_bytes(bytes[9..17].try_into().ok()?);
                Some((TextOperation::DeleteRange { start, end }, 17))
            }
            DELETE_ALL => {
                // DeleteAll
                Some((TextOperation::DeleteAll, 1))
            }
            INSERT_TEXT => {
                // InsertText
                if bytes.len() < 1 + 8 {
                    return None;
                }
                let position = usize::from_be_bytes(bytes[1..9].try_into().ok()?);
                let (text, text_size) = String::from_bytes(&bytes[9..])?;
                Some((
                    TextOperation::InsertText { position, text },
                    9 + text_size,
                ))
            }
            _ => None,
        }
    }
}

impl TextOperation {
    /// Transformación cuando ambas operaciones son inserciones concurrentes.
    /// Si la otra inserción está antes o en la misma posición, desplazamos la posición actual.
    fn transform_insert_insert(&self, other: &Self) -> Self {
        match (self, other) {
            (
                TextOperation::Insert {
                    position: p1,
                    character: c1,
                },
                TextOperation::Insert { position: p2, .. },
            ) => {
                if *p1 < *p2 {
                    self.clone()
                } else if *p1 > *p2 {
                    TextOperation::Insert {
                        position: p1 + 1,
                        character: *c1,
                    }
                } else {
                    // Mismo índice: definimos que la operación "self" se desplaza hacia adelante.
                    TextOperation::Insert {
                        position: p1 + 1,
                        character: *c1,
                    }
                }
            }
            _ => self.clone(), // No debería pasar
        }
    }

    /// Transformación cuando esta operación es insert y la otra es delete.
    /// Si la otra delete está antes que esta inserción, desplazamos hacia atrás la posición.
    fn transform_insert_delete(&self, other: &Self) -> Self {
        match (self, other) {
            (
                TextOperation::Insert {
                    position: p1,
                    character: c1,
                },
                TextOperation::Delete { position: p2 },
            ) => {
                if *p1 <= *p2 {
                    self.clone()
                } else {
                    TextOperation::Insert {
                        position: p1 - 1,
                        character: *c1,
                    }
                }
            }
            _ => self.clone(),
        }
    }
    /// Transformación cuando esta operación es delete y la otra es insert.
    /// Si la otra insert está antes o igual que esta delete, desplazamos hacia adelante la posición.
    fn transform_delete_insert(&self, other: &Self) -> Self {
        match (self, other) {
            (
                TextOperation::Delete { position: p1 },
                TextOperation::Insert { position: p2, .. },
            ) => {
                if *p1 < *p2 {
                    self.clone()
                } else {
                    TextOperation::Delete { position: p1 + 1 }
                }
            }
            _ => self.clone(),
        }
    }
    // Transformación cuando ambas operaciones son deletes.
    // Si ambas deletes son en la misma posición, se convierte en NoOperation.
    fn transform_delete_delete(&self, other: &Self) -> Self {
        match (self, other) {
            (TextOperation::Delete { position: p1 }, TextOperation::Delete { position: p2 }) => {
                if p1 == p2 {
                    // Ambas deletes borran el mismo carácter -> no-op
                    TextOperation::NoOperation
                } else if p1 < p2 {
                    self.clone()
                } else {
                    TextOperation::Delete { position: p1 - 1 }
                }
            }
            _ => self.clone(),
        }
    }
}

impl Transformable for TextOperation {
    fn transform(&self, other: &Self) -> Self {
        match (self, other) {
            (TextOperation::Insert { .. }, TextOperation::Insert { .. }) => {
                                        self.transform_insert_insert(other)
                                    }
            (TextOperation::Insert { .. }, TextOperation::Delete { .. }) => {
                                        self.transform_insert_delete(other)
                                    }
            (TextOperation::Delete { .. }, TextOperation::Insert { .. }) => {
                                        self.transform_delete_insert(other)
                                    }
            (TextOperation::Delete { .. }, TextOperation::Delete { .. }) => {
                                        self.transform_delete_delete(other)
                                    }
            (TextOperation::NoOperation, _) => self.clone(),
            (_, TextOperation::NoOperation) => other.clone(),
            (
                                        TextOperation::Insert {
                                            position: _,
                                            character,
                                        },
                                        TextOperation::DeleteAll,
                                    ) => {
                                        // Si esta operación es DeleteAll,  el insert vuelve al indice 0
                                        TextOperation::Insert {
                                            position: 0,
                                            character: *character,
                                        }
                                    }
            (TextOperation::Delete { .. }, TextOperation::DeleteAll) => {
                                        TextOperation::NoOperation // Si es DeleteAll, es un noop, porque ya se borro todo
                                    }
            (TextOperation::DeleteAll, _) => {
                                        // Si esta operación es DeleteAll, queda igual no importa que
                                        self.clone()
                                    }
            (TextOperation::InsertText { position, text },TextOperation::Insert { position: p2, character: _ }) => {
                                if *position < *p2 {
                                    // La inserción de texto ocurre antes de la inserción de un carácter
                                    TextOperation::InsertText {
                                        position: *position,
                                        text: text.clone(),
                                    }
                                } else {
                                    // La inserción de texto ocurre después, desplazamos la posición
                                    TextOperation::InsertText {
                                        position: *position + 1,
                                        text: text.clone(),
                                    }
                                }
                            }
            (TextOperation::InsertText { position, text }, TextOperation::Delete { position: p2 }) => {
                                if *position < *p2{
                                    // La inserción de texto ocurre antes o en la misma posición que la eliminación
                                    TextOperation::InsertText {
                                        position: *position,
                                        text: text.clone(),
                                    }
                                } else {
                                    // La inserción de texto ocurre después, desplazamos la posición
                                    TextOperation::InsertText {
                                        position: *position - 1,
                                        text: text.clone(),
                                    }
                                }
                            }
            (TextOperation::InsertText { position: _, text }, TextOperation::DeleteAll) => {
                                // Si la operación es DeleteAll, la inserción de texto se convierte en una inserción al inicio
                                TextOperation::InsertText {
                                    position: 0,
                                    text: text.clone(),
                                }
                            }
            (TextOperation::InsertText { position: p1, text: t1 }, TextOperation::InsertText { position: p2, text:t2 }) => {
                                if *p1 <= *p2 {
                                    // La inserción de texto ocurre antes de la otra inserción de texto
                                    TextOperation::InsertText {
                                        position: *p1,
                                        text: t1.clone(),
                                    }
                                } else if *p1 > *p2 {
                                    // La inserción de texto ocurre después, desplazamos la posición
                                    TextOperation::InsertText {
                                        position: *p1 + t2.chars().count(),
                                        text: t1.clone(),
                                    }
                                } else {
                                    // Mismo índice: definimos que la operación "self" se desplaza hacia adelante.
                                    TextOperation::InsertText {
                                        position: *p1 + t2.chars().count(),
                                        text: t1.clone(),
                                    }
                                }
                            }
            (TextOperation::Insert { position:p1, character }, TextOperation::InsertText { position:p2, text }) => {
                            if *p1 <= *p2 {
                                    // La inserción de texto ocurre antes de la otra inserción de texto
                                    self.clone()
                                } else if *p1 > *p2 {
                                    // La inserción de texto ocurre después, desplazamos la posición
                                    TextOperation::Insert {
                                        position: *p1 + text.chars().count(),
                                        character: *character,
                                    }
                                } else {
                                    // Mismo índice: definimos que la operación "self" se desplaza hacia adelante.
                                    TextOperation::Insert {
                                        position: *p1 + text.chars().count(),
                                        character: *character,
                                    }
                                }
                            }
            (TextOperation::Delete { position: p1 }, TextOperation::InsertText { position: p2, text }) => {
            
                        if *p1 < *p2 {
                            // La eliminación ocurre antes de la inserción de texto
                            self.clone()
                        } else {
                            // La eliminación ocurre después, desplazamos la posición
                            TextOperation::Delete { position: *p1 + text.chars().count() }
                        }
                    }
            (TextOperation::Insert { position, character }, TextOperation::DeleteRange { start, end }) => {
                if *position < *start {
                    // La inserción ocurre antes del rango de eliminación
                    self.clone()
                } else if *position >= *end {
                    // La inserción ocurre después del rango de eliminación, desplazamos la posición
                    TextOperation::Insert {
                        position: *position - (end - start),
                        character: *character,
                    }
                } else {
                    // La inserción ocurre dentro del rango de eliminación, Se mueve el indice al start
                    TextOperation::Insert { position: *start, character: *character }
                }
            },
            (TextOperation::Delete { position }, TextOperation::DeleteRange { start, end }) => {
                if *position < *start {
                    // La eliminación del caracter ocurre antes del rango de eliminación
                    self.clone()
                } else if *position >= *end {
                    // La eliminación del caracter ocurre después del rango de eliminación, desplazamos la posición
                    TextOperation::Delete { position: *position - (end - start) }
                } else {
                    // La eliminación ocurre dentro del rango de eliminación, se convierte en NoOperation
                    TextOperation::NoOperation
                }
            },
            (TextOperation::InsertText { position, text }, TextOperation::DeleteRange { start, end }) => {
                if *position < *start {
                    // La inserción de texto ocurre antes del rango de eliminación
                    self.clone()
                } else if *position >= *end {
                    // La inserción de texto ocurre después del rango de eliminación, desplazamos la posición
                    TextOperation::InsertText {
                        position: *position - (end - start),
                        text: text.clone(),
                    }
                } else {
                    // La inserción de texto ocurre dentro del rango de eliminación, se mueve al inicio del rango
                    TextOperation::InsertText { position: *start, text: text.clone() }
                }
            },

            (TextOperation::DeleteRange { start, end }, TextOperation::Insert { position, character: _ }) => {
                if *position < *start {
                    // La inserción ocurre antes del rango de eliminación
                    TextOperation::DeleteRange { start: *start + 1 , end: *end + 1 }
                } else if *position >= *end {
                    // La inserción ocurre después del rango de eliminación, desplazamos la posición
                    self.clone()
                } else {
                    // La inserción ocurre dentro del rango de eliminación, se mueve el final del rango + 1
                    TextOperation::DeleteRange { start: *start, end: *end + 1 }
                }
            },
            (TextOperation::DeleteRange { start, end }, TextOperation::Delete { position }) => {
                if *position < *start {
                    // La eliminación del caracter ocurre antes del rango de eliminación
                    TextOperation::DeleteRange { start: *start - 1, end: *end - 1 }
                } else if *position >= *end {
                    // La eliminación del caracter ocurre después del rango de eliminación, desplazamos la posición
                    self.clone()
                } else {
                    // La eliminación ocurre dentro del rango de eliminación, se reduce el rango
                    TextOperation::DeleteRange { start: *start, end: *end - 1 }
                }
            },
            (TextOperation::DeleteRange { start: _, end: _ }, TextOperation::DeleteAll) =>{
                TextOperation::NoOperation // Si es DeleteAll, es un noop, porque ya se borro todo
            },
            (TextOperation::DeleteRange { start: start1, end: end1 }, TextOperation::DeleteRange { start: start2, end: end2 }) => {
                if end1 < start2 {
                    // El rango de eliminación 1 ocurre antes del rango de eliminación 2
                    self.clone()
                } else if start1 >= end2 {
                    // El rango de eliminación 1 ocurre después del rango de eliminación 2, desplazamos el rango
                    TextOperation::DeleteRange { start: *start1 - (end2 - start2), end: *end1 - (end2 - start2) }
                } else if end1 < end2{
                    // el rango 1 esta contenido en el rango 2 
                    TextOperation::NoOperation
                }
                else{
                    // Los rangos se superponen parcialmente, pero pero el rango 1 termina despues
                    TextOperation::DeleteRange { start: *start2, end: *start2 + (*end1 - *end2) }
                }
            },
            (TextOperation::DeleteRange { start, end }, TextOperation::InsertText { position, text }) => {
                if *position < *start {
                    // La inserción de texto ocurre antes del rango de eliminación
                    TextOperation::DeleteRange { start: *start + text.chars().count(), end: *end + text.chars().count() }
                } else if *position >= *end {
                    // La inserción de texto ocurre después del rango de eliminación, queda igual
                    self.clone()
                } else {
                    // La inserción de texto ocurre dentro del rango de eliminación, se mueve al inicio del rango
                    TextOperation::DeleteRange { start: *start, end: *end + text.chars().count() }
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_insert_insert() {
        // Caso: inserción en posición menor → sin cambio
        let op1 = TextOperation::Insert {
            position: 2,
            character: 'a',
        };
        let op2 = TextOperation::Insert {
            position: 5,
            character: 'b',
        };
        assert_eq!(op1.transform(&op2), op1);

        // Caso: inserción en posición mayor → desplazamos posición
        let op1 = TextOperation::Insert {
            position: 5,
            character: 'a',
        };
        let op2 = TextOperation::Insert {
            position: 2,
            character: 'b',
        };
        let expected = TextOperation::Insert {
            position: 6,
            character: 'a',
        };
        assert_eq!(op1.transform(&op2), expected);

        // Caso: inserción en la misma posición → desplazamos posición
        let op1 = TextOperation::Insert {
            position: 3,
            character: 'a',
        };
        let op2 = TextOperation::Insert {
            position: 3,
            character: 'b',
        };
        let expected = TextOperation::Insert {
            position: 4,
            character: 'a',
        };
        assert_eq!(op1.transform(&op2), expected);
    }

    #[test]
    fn test_transform_insert_delete() {
        // Inserción antes de delete → sin cambio
        let op1 = TextOperation::Insert {
            position: 2,
            character: 'a',
        };
        let op2 = TextOperation::Delete { position: 5 };
        assert_eq!(op1.transform(&op2), op1);

        // Inserción después de delete → desplazamos posición
        let op1 = TextOperation::Insert {
            position: 5,
            character: 'a',
        };
        let op2 = TextOperation::Delete { position: 2 };
        let expected = TextOperation::Insert {
            position: 4,
            character: 'a',
        };
        assert_eq!(op1.transform(&op2), expected);
    }

    #[test]
    fn test_transform_delete_insert() {
        // Delete antes de insert → sin cambio
        let op1 = TextOperation::Delete { position: 2 };
        let op2 = TextOperation::Insert {
            position: 5,
            character: 'b',
        };
        assert_eq!(op1.transform(&op2), op1);

        // Delete en o después de insert → desplazamos posición
        let op1 = TextOperation::Delete { position: 5 };
        let op2 = TextOperation::Insert {
            position: 2,
            character: 'b',
        };
        let expected = TextOperation::Delete { position: 6 };
        assert_eq!(op1.transform(&op2), expected);
    }

    #[test]
    fn test_transform_delete_delete() {
        // Ambas deletes en misma posición → NoOperation
        let op1 = TextOperation::Delete { position: 3 };
        let op2 = TextOperation::Delete { position: 3 };
        assert_eq!(op1.transform(&op2), TextOperation::NoOperation);

        // Delete antes de otra delete → sin cambio
        let op1 = TextOperation::Delete { position: 2 };
        let op2 = TextOperation::Delete { position: 5 };
        assert_eq!(op1.transform(&op2), op1);

        // Delete después de otra delete → desplazamos posición hacia atrás
        let op1 = TextOperation::Delete { position: 5 };
        let op2 = TextOperation::Delete { position: 2 };
        let expected = TextOperation::Delete { position: 4 };
        assert_eq!(op1.transform(&op2), expected);
    }

    #[test]
    fn test_apply_noop() {
        let mut doc = String::from("hello");
        let op = TextOperation::NoOperation;
        op.apply(&mut doc);
        assert_eq!(doc, "hello"); // no cambia nada
    }

    // Teste de serializacion
    #[test]
    fn test_insert_serialization() {
        let op = TextOperation::Insert {
            position: 42,
            character: 'ñ',
        };
        let bytes = op.to_bytes();
        let (parsed, used) = TextOperation::from_bytes(&bytes).unwrap();
        assert_eq!(op, parsed);
        assert_eq!(used, bytes.len());
    }

    #[test]
    fn test_delete_serialization() {
        let op = TextOperation::Delete { position: 123456 };
        let bytes = op.to_bytes();
        let (parsed, used) = TextOperation::from_bytes(&bytes).unwrap();
        assert_eq!(op, parsed);
        assert_eq!(used, bytes.len());
    }

    #[test]
    fn test_noop_serialization() {
        let op = TextOperation::NoOperation;
        let bytes = op.to_bytes();
        let (parsed, used) = TextOperation::from_bytes(&bytes).unwrap();
        assert_eq!(op, parsed);
        assert_eq!(used, bytes.len());
    }

    #[test]
    fn test_invalid_bytes() {
        // Discriminante inválido
        let bytes = vec![99];
        assert!(TextOperation::from_bytes(&bytes).is_none());
        // Bytes insuficientes para Insert
        let bytes = vec![0, 0, 0];
        assert!(TextOperation::from_bytes(&bytes).is_none());
    }

    #[test]
    fn test_string_to_bytes_and_back() {
        let original = String::from("Hola mundo!");
        let bytes = original.to_bytes();
        let (parsed, used) = String::from_bytes(&bytes).unwrap();
        assert_eq!(original, parsed);
        assert_eq!(used, bytes.len());
    }

    #[test]
    fn test_empty_string() {
        let original = String::new();
        let bytes = original.to_bytes();
        let (parsed, used) = String::from_bytes(&bytes).unwrap();
        assert_eq!(original, parsed);
        assert_eq!(used, bytes.len());
    }

    #[test]
    fn test_unicode_string() {
        let original = String::from("¡Hola, 世界!");
        let bytes = original.to_bytes();
        let (parsed, used) = String::from_bytes(&bytes).unwrap();
        assert_eq!(original, parsed);
        assert_eq!(used, bytes.len());
    }

    #[test]
    fn test_string_invalid_bytes() {
        // Menos de 8 bytes (no hay longitud)
        assert!(String::from_bytes(&[1, 2, 3]).is_none());
        // Longitud mayor que los datos disponibles
        let mut bytes = 10u64.to_le_bytes().to_vec();
        bytes.extend(b"abc");
        assert!(String::from_bytes(&bytes).is_none());
    }
    
    #[test]
    fn test_delete_range(){
        let mut doc = String::from("Hello, world!");
        let op = TextOperation::DeleteRange { start: 7, end: 12 };
        op.apply(&mut doc);
        assert_eq!(doc, "Hello, !");
    }
}
