use crate::app::{index::document::Document, operation::generic::ParsableBytes}; // Adjust the path as needed

pub type Documents = Vec<Document>;

impl ParsableBytes for Documents {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        // Guardamos la cantidad de documentos como u32 (4 bytes, little endian)
        bytes.extend(&(self.len() as u32).to_le_bytes());
        for doc in self {
            let doc_bytes = doc.to_bytes();
            // Guardamos el tamaño del documento como u32 (4 bytes, little endian)
            bytes.extend(&(doc_bytes.len() as u32).to_le_bytes());
            bytes.extend(doc_bytes);
        }
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<(Self, usize)> {
        let mut offset = 0;
        if bytes.len() < 4 {
            return None;
        }
        let len = u32::from_le_bytes(bytes[offset..offset + 4].try_into().ok()?) as usize;
        offset += 4;
        let mut docs = Vec::with_capacity(len);
        for _ in 0..len {
            if bytes.len() < offset + 4 {
                return None;
            }
            let doc_size = u32::from_le_bytes(bytes[offset..offset + 4].try_into().ok()?) as usize;
            offset += 4;
            if bytes.len() < offset + doc_size {
                return None;
            }
            let (doc, used) = Document::from_bytes(&bytes[offset..offset + doc_size])?;
            if used != doc_size {
                return None; // Inconsistencia, abortar
            }
            docs.push(doc);
            offset += doc_size;
        }
        Some((docs, offset))
    }
}
