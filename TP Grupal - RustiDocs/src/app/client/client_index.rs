use std::{
    io::BufReader,
    sync::mpsc::{Receiver, Sender, channel},
    thread,
};

use crate::{
    app::{
        index::{document::DocType, documents::Documents, index_instructions::IndexInstructions},
        operation::generic::ParsableBytes,
    },
    client_lib::cluster_manager::ClusterManager,
    network::{RespMessage, resp_parser::parse_resp_line},
};

const INDEX_CHANNEL: &str = "INDEX";

pub struct ClientIndex {
    cluster: ClusterManager,
}

impl ClientIndex {
    pub fn new(address: &str, user: &str, password: &str) -> (Self, Receiver<Documents>) {
        let cluster =
            ClusterManager::new(address.to_string(), user.to_string(), password.to_string())
                .unwrap();

        let (sender, receiver) = channel();

        let mut listener = IndexListener::new(sender, address, user, password).unwrap();

        // MANEJAR ESTE THREATH
        thread::spawn(move || listener.run());

        (Self { cluster }, receiver)
    }

    pub fn add_doc(&mut self, doc_name: String, doc_type: DocType) {
        println!("[ClientIndex::add_doc] Creando documento: {} tipo: {:?}", doc_name, doc_type);
        let instruction = IndexInstructions::CreateDoc(doc_name.clone(), doc_type);
        let bytes = instruction.to_bytes();
        println!("[ClientIndex::add_doc] Bytes a enviar: {:?}", bytes);
        match self.cluster.publish(INDEX_CHANNEL, &bytes) {
            Ok(_) => println!("[ClientIndex::add_doc] Mensaje publicado exitosamente en canal {}", INDEX_CHANNEL),
            Err(e) => println!("[ClientIndex::add_doc] Error publicando: {:?}", e),
        }
    }

    pub fn remove_doc(&mut self, doc_name: String) {
        let instruction = IndexInstructions::RemoveDoc(doc_name);
        self.cluster
            .publish(INDEX_CHANNEL, &instruction.to_bytes())
            .unwrap();
    }

    pub fn refresh(&mut self) {
        let instruction = IndexInstructions::Refresh;
        self.cluster
            .publish(INDEX_CHANNEL, &instruction.to_bytes())
            .unwrap();
    }
}

struct IndexListener {
    sender: Sender<Documents>,
    cluster: ClusterManager,
}

impl IndexListener {
    pub fn new(
        sender: Sender<Documents>,
        address: &str,
        user: &str,
        password: &str,
    ) -> Result<Self, ()> {
        let cluster =
            ClusterManager::new(address.to_string(), user.to_string(), password.to_string())
                .unwrap();

        Ok(Self { sender, cluster })
    }

    pub fn run(&mut self) {
        let active_stream = self.cluster.subscribe(INDEX_CHANNEL).unwrap();
        let mut reader = BufReader::new(active_stream);
        loop {
            match parse_resp_line(&mut reader) {
                Err(e) => {
                    eprintln!("Error leyendo del socket: {}", e);
                    break;
                }
                Ok(contenido) => match contenido {
                    RespMessage::BulkString(Some(bytes)) => {
                        if let Some((IndexInstructions::Docs(docs), _)) =
                            IndexInstructions::from_bytes(&bytes)
                        {
                            let _ = self.sender.send(docs);
                        }
                    }
                    RespMessage::SimpleString(bytes) => {
                        if let Some((IndexInstructions::Docs(docs), _)) =
                            IndexInstructions::from_bytes(bytes.as_bytes())
                        {
                            let _ = self.sender.send(docs);
                        }
                    }
                    _ => continue,
                },
            }
        }
    }
}
