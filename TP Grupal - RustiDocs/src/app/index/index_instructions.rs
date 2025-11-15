use crate::app::index::document::DocType;
use crate::app::index::documents::Documents;
use crate::app::operation::generic::ParsableBytes;

#[derive(Debug)]
pub enum IndexInstructions {
    Docs(Documents),
    CreateDoc(String, DocType),
    RemoveDoc(String),
    Refresh,
}
impl ParsableBytes for IndexInstructions {
    fn from_bytes(bytes: &[u8]) -> Option<(IndexInstructions, usize)> {
        match bytes.first()? {
            0 => {
                // Docs
                Documents::from_bytes(&bytes[1..]).map(|(docs_vec, used)| {
                    (IndexInstructions::Docs(Documents::from(docs_vec)), used + 1)
                })
            }
            1 => {
                // CreateDoc
                let (name, used1) = String::from_bytes(&bytes[1..])?;
                let (doc_type, used2) = DocType::from_bytes(&bytes[1 + used1..])?;
                Some((
                    IndexInstructions::CreateDoc(name, doc_type),
                    1 + used1 + used2,
                ))
            }
            2 => {
                // RemoveDoc
                let (name, used) = String::from_bytes(&bytes[1..])?;
                Some((IndexInstructions::RemoveDoc(name), 1 + used))
            }
            3 => {
                // Refresh
                Some((IndexInstructions::Refresh, 1))
            }
            _ => None,
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        match self {
            IndexInstructions::Docs(docs) => {
                let mut v = vec![0];
                v.extend(docs.to_bytes());
                v
            }
            IndexInstructions::CreateDoc(name, doc_type) => {
                let mut v = vec![1];
                v.extend(name.to_bytes());
                v.extend(doc_type.to_bytes());
                v
            }
            IndexInstructions::RemoveDoc(name) => {
                let mut v = vec![2];
                v.extend(name.to_bytes());
                v
            }
            IndexInstructions::Refresh => vec![3],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::index::document::{DocType, Document};

    fn make_documents() -> Documents {
        // Assuming Documents can be created from a Vec<Document>
        let doc1 = Document::new("doc1".to_string(), DocType::SpreadSheet);
        let doc2 = Document::new("doc2".to_string(), DocType::Text);
        Documents::from(vec![doc1, doc2])
    }

    #[test]
    fn test_to_bytes_and_from_bytes_docs() {
        let docs = make_documents();
        let docs2 = make_documents();

        let instr = IndexInstructions::Docs(docs);
        let bytes = instr.to_bytes();
        let (parsed, used) = IndexInstructions::from_bytes(&bytes).unwrap();
        match parsed {
            IndexInstructions::Docs(parsed_docs) => {
                assert_eq!(parsed_docs[0], docs2[0]);
                assert_eq!(parsed_docs[1], docs2[1]);
            }
            _ => panic!("Expected Docs variant"),
        }
        assert_eq!(used, bytes.len());
    }

    #[test]
    fn test_to_bytes_and_from_bytes_refresh() {
        let instr = IndexInstructions::Refresh;
        let bytes = instr.to_bytes();
        let (parsed, used) = IndexInstructions::from_bytes(&bytes).unwrap();
        match parsed {
            IndexInstructions::Refresh => {}
            _ => panic!("Expected Refresh variant"),
        }
        assert_eq!(used, 1);
    }

    #[test]
    fn test_from_bytes_invalid_instruction() {
        let bytes = vec![42, 0, 1, 2];
        assert!(IndexInstructions::from_bytes(&bytes).is_none());
    }

    #[test]
    fn test_from_bytes_empty() {
        let bytes = vec![];
        assert!(IndexInstructions::from_bytes(&bytes).is_none());
    }
}
