use crate::app::operation::{
    generic::{Applicable, ParsableBytes, Transformable},
    text::TextOperation,
};

#[derive(Debug, Clone)]
pub struct SpreadSheet {
    pub data: Vec<Vec<String>>,
}

impl Default for SpreadSheet {
    fn default() -> Self {
        // Crear una matriz de 5 filas x 10 columnas con celdas vacías
        let data = vec![vec![String::new(); 10]; 5];
        Self { data }
    }
}

impl SpreadSheet {
    pub fn insert_char_cell(
        &mut self,
        row: usize,
        column: usize,
        pos: usize,
        char: char,
    ) -> Result<(), String> {
        // Asegurar que existan suficientes filas
        while self.data.len() <= row {
            self.data.push(Vec::new());
        }
        // Asegurar que existan suficientes columnas en la fila
        while self.data[row].len() <= column {
            self.data[row].push(String::new());
        }
        // Asegurar que la posición sea válida
        if pos > self.data[row][column].len() {
            return Err(format!(
                "Posición {} fuera de rango en celda [{},{}]",
                pos, row, column
            ));
        }
        self.data[row][column].insert(pos, char);
        Ok(())
    }

    pub fn delete_char_cell(
        &mut self,
        row: usize,
        column: usize,
        pos: usize,
    ) -> Result<(), String> {
        if row >= self.data.len() || column >= self.data[row].len() {
            return Err(format!("Índice fuera de rango: [{},{}]", row, column));
        }
        if pos >= self.data[row][column].len() {
            return Err(format!(
                "Posición {} fuera de rango en celda [{},{}]",
                pos, row, column
            ));
        }
        self.data[row][column].remove(pos);
        Ok(())
    }
}

#[derive(Default, Debug, Clone)]
pub struct SpreadOperation {
    pub row: usize,
    pub column: usize,
    pub operation: TextOperation,
}

impl Applicable<SpreadSheet> for SpreadOperation {
    fn apply(&self, data: &mut SpreadSheet) {
        match self.operation {
            TextOperation::Insert {
                position,
                character,
            } => {
                let _ = data.insert_char_cell(self.row, self.column, position, character);
            }
            TextOperation::Delete { position } => {
                let _ = data.delete_char_cell(self.row, self.column, position);
            }
            _ => {}
        }
    }
}

impl Transformable for SpreadOperation {
    fn transform(&self, other: &Self) -> Self {
        let operation = self.operation.transform(&other.operation);
        Self {
            row: self.row,
            column: self.column,
            operation: operation,
        }
    }
}

impl ParsableBytes for SpreadOperation {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(&(self.row as u64).to_le_bytes());
        bytes.extend(&(self.column as u64).to_le_bytes());
        bytes.extend(self.operation.to_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<(SpreadOperation, usize)> {
        if bytes.len() < 16 {
            return None;
        }
        let row = u64::from_le_bytes(bytes[0..8].try_into().ok()?) as usize;
        let column = u64::from_le_bytes(bytes[8..16].try_into().ok()?) as usize;
        let (operation, op_offset) = TextOperation::from_bytes(&bytes[16..])?;
        Some((
            SpreadOperation {
                row,
                column,
                operation,
            },
            16 + op_offset,
        ))
    }
}

impl ParsableBytes for SpreadSheet {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let row_count = self.data.len() as u64;
        bytes.extend(&row_count.to_le_bytes());
        for row in &self.data {
            let col_count = row.len() as u64;
            bytes.extend(&col_count.to_le_bytes());
            for cell in row {
                let cell_bytes = cell.as_bytes();
                let cell_len = cell_bytes.len() as u64;
                bytes.extend(&cell_len.to_le_bytes());
                bytes.extend(cell_bytes);
            }
        }
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<(SpreadSheet, usize)> {
        let mut offset = 0;
        let row_count = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap()) as usize;
        offset += 8;
        let mut data = Vec::with_capacity(row_count);
        for _ in 0..row_count {
            let col_count =
                u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap()) as usize;
            offset += 8;
            let mut row = Vec::with_capacity(col_count);
            for _ in 0..col_count {
                let cell_len =
                    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap()) as usize;
                offset += 8;
                let cell = String::from_utf8(bytes[offset..offset + cell_len].to_vec()).unwrap();
                offset += cell_len;
                row.push(cell);
            }
            data.push(row);
        }
        Some((Self { data }, offset))
    }
}
