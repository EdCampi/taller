use crate::app::operation::generic::ParsableBytes;

#[derive(Debug, Clone, PartialEq)]
pub enum DocType {
    Text,
    SpreadSheet,
}

impl DocType {
    #[allow(dead_code)] // TODO: Remover
    fn string_type(&self) -> String {
        match self {
            DocType::Text => "Text".to_string(),
            DocType::SpreadSheet => "Spreadsheat".to_string(),
        }
    }
}

impl ParsableBytes for DocType {
    fn to_bytes(&self) -> Vec<u8> {
        let byte = match self {
            DocType::Text => 0u8,
            DocType::SpreadSheet => 1u8,
        };
        vec![byte]
    }

    fn from_bytes(bytes: &[u8]) -> Option<(Self, usize)> {
        if bytes.is_empty() {
            return None;
        }
        let doc_type = match bytes[0] {
            0 => DocType::Text,
            1 => DocType::SpreadSheet,
            _ => return None,
        };
        Some((doc_type, 1))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    name: String,
    doc_type: DocType,
    connected_clients: u64,
    active: bool,
}

impl Document {
    pub fn new(doc_name: String, doc_type: DocType) -> Self {
        Self {
            name: doc_name,
            doc_type,
            connected_clients: 0,
            active: false,
        }
    }

    pub fn get_name(&self) -> String {
        self.name.to_string()
    }

    pub fn get_type(&self) -> DocType {
        self.doc_type.clone()
    }
}

impl ParsableBytes for Document {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        // Serialize name length and name
        let name_bytes = self.name.as_bytes();
        let name_len = name_bytes.len() as u32;
        bytes.extend(&name_len.to_le_bytes());
        bytes.extend(name_bytes);

        // Serialize doc_type
        let doc_type_byte = match self.doc_type {
            DocType::Text => 0u8,
            DocType::SpreadSheet => 1u8,
        };
        bytes.push(doc_type_byte);

        // Serialize connected_clients
        bytes.extend(&self.connected_clients.to_le_bytes());

        // Serialize active
        bytes.push(self.active as u8);

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<(Self, usize)> {
        let mut offset = 0;

        // Read name length
        if bytes.len() < offset + 4 {
            return None;
        }
        let name_len = u32::from_le_bytes(bytes[offset..offset + 4].try_into().ok()?) as usize;
        offset += 4;

        // Read name
        if bytes.len() < offset + name_len {
            return None;
        }
        let name = String::from_utf8(bytes[offset..offset + name_len].to_vec()).ok()?;
        offset += name_len;

        // Read doc_type
        if bytes.len() < offset + 1 {
            return None;
        }
        let doc_type = match bytes[offset] {
            0 => DocType::Text,
            1 => DocType::SpreadSheet,
            _ => return None,
        };
        offset += 1;

        // Read connected_clients
        if bytes.len() < offset + 8 {
            return None;
        }
        let connected_clients = u64::from_le_bytes(bytes[offset..offset + 8].try_into().ok()?);
        offset += 8;

        // Read active
        if bytes.len() < offset + 1 {
            return None;
        }
        let active = bytes[offset] != 0;
        offset += 1;

        Some((
            Document {
                name,
                doc_type,
                connected_clients,
                active,
            },
            offset,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_to_bytes_and_from_bytes_text() {
        let doc = Document {
            name: "TestDoc".to_string(),
            doc_type: DocType::Text,
            connected_clients: 42,
            active: true,
        };
        let bytes = doc.to_bytes();
        let (parsed_doc, used) = Document::from_bytes(&bytes).unwrap();
        assert_eq!(used, bytes.len());
        assert_eq!(parsed_doc.name, "TestDoc");
        match parsed_doc.doc_type {
            DocType::Text => {}
            _ => panic!("Expected DocType::Text"),
        }
        assert_eq!(parsed_doc.connected_clients, 42);
        assert!(parsed_doc.active);
    }

    #[test]
    fn test_document_to_bytes_and_from_bytes_spreadsheet() {
        let doc = Document {
            name: "Sheet1".to_string(),
            doc_type: DocType::SpreadSheet,
            connected_clients: 0,
            active: false,
        };
        let bytes = doc.to_bytes();
        let (parsed_doc, used) = Document::from_bytes(&bytes).unwrap();
        assert_eq!(used, bytes.len());
        assert_eq!(parsed_doc.name, "Sheet1");
        match parsed_doc.doc_type {
            DocType::SpreadSheet => {}
            _ => panic!("Expected DocType::SpreadSheet"),
        }
        assert_eq!(parsed_doc.connected_clients, 0);
        assert!(!parsed_doc.active);
    }

    #[test]
    fn test_document_from_bytes_invalid_doc_type() {
        let mut doc = Document {
            name: "Invalid".to_string(),
            doc_type: DocType::Text,
            connected_clients: 1,
            active: false,
        }
        .to_bytes();
        // Overwrite doc_type byte with invalid value
        let _ = 7u32.to_le_bytes();
        let offset = 4 + 7; // name_len (4 bytes) + name (7 bytes)
        doc[offset] = 99;
        assert!(Document::from_bytes(&doc).is_none());
    }

    #[test]
    fn test_document_from_bytes_truncated() {
        let doc = Document {
            name: "Short".to_string(),
            doc_type: DocType::Text,
            connected_clients: 1,
            active: false,
        }
        .to_bytes();
        // Remove last byte (active)
        let truncated = &doc[..doc.len() - 1];
        assert!(Document::from_bytes(truncated).is_none());
    }
}
