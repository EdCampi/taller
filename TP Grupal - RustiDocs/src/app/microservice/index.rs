use crate::app::index::document::Document;
use crate::{
    app::{
        index::{document::DocType, index_instructions::IndexInstructions},
        microservice::service::Service,
        operation::{
            csv::{SpreadOperation, SpreadSheet},
            generic::ParsableBytes,
            text::TextOperation,
        },
    },
    client_lib::cluster_manager::ClusterManager,
    network::{RespMessage, resp_parser::parse_resp_line},
};
use std::io::Read;
use std::time::Duration;
use std::{
    collections::HashMap,
    io::BufReader,
    net::TcpStream,
    sync::mpsc::channel,
    thread::{self, JoinHandle},
};

use crate::app::index::documents::Documents;
use crate::cluster::types::DEFAULT_BUFFER_SIZE;

/// Key donde se almacenan los documentos creados
const DOC_KEY: &str = "INDEX";
// Nombre del canal donde opera Index
const INDEX_CHANNEL: &str = "INDEX";

enum IndexError {
    ChannelClosed,
}

pub struct Index {
    cluster: ClusterManager,
    docs: Documents,
    service_handles: HashMap<String, JoinHandle<()>>,
}

impl Index {
    pub fn new(cluster_manager: ClusterManager) -> Self {
        Self {
            cluster: cluster_manager,
            docs: Vec::new(),
            service_handles: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        // Empiezo seteando los documentos
        if let Some(documents) = self.get_docs() {
            self.docs = documents;
        }

        println!("Los docs iniciados {:?}", self.docs);

        let docs_clonados: Vec<Document> = self.docs.clone();
        for document in docs_clonados {
            self.ensure_service_running(&document);
        }

        let pub_sub_stream = self.cluster.subscribe(INDEX_CHANNEL).unwrap();

        // si esto da error hay que codear cambiar de channel proximament
        let _ = self.run_read_channel(pub_sub_stream);
    }

    fn ensure_service_running(&mut self, doc: &Document) {
        let doc_name = doc.get_name();
        let restart = match self.service_handles.get(&doc_name) {
            Some(handle) => handle.is_finished(),
            None => true,
        };
        if restart {
            let handle = self.init_service(doc);
            self.service_handles.insert(doc_name, handle);
        }
    }

    fn run_read_channel(&mut self, mut channel_stream: TcpStream) -> Result<(), IndexError> {
        // Set non-blocking mode to avoid sleep
        channel_stream
            .set_nonblocking(true)
            .map_err(|_| IndexError::ChannelClosed)?;

        let mut buffer = [0; DEFAULT_BUFFER_SIZE];

        loop {
            match channel_stream.read(&mut buffer) {
                Ok(0) => {
                    println!("[INDEX] Connection closed by peer");
                    break;
                }
                Ok(n) => {
                    let received_data = &buffer[..n];

                    let contenido = match parse_resp_line(&mut BufReader::new(received_data)) {
                        Ok(msg) => msg,
                        Err(e) => {
                            eprintln!("[INDEX] Error parsing message: {}", e);
                            continue;
                        }
                    };

                    println!("[INDEX] Mensaje recibido: {:?}", contenido);

                    if let RespMessage::SimpleString(bytes) = contenido {
                        println!("[INDEX] Bytes del mensaje: {:?}", bytes.as_bytes());
                        if let Some((instruction, _)) =
                            IndexInstructions::from_bytes(bytes.as_bytes())
                        {
                            println!("[INDEX] Instrucción parseada: {:?}", instruction);
                            match instruction {
                                IndexInstructions::CreateDoc(name, tipo) => {
                                    println!("[INDEX] Creating document: {}", name);
                                    self.add_doc(Document::new(name, tipo));
                                    self.set_docs();
                                }
                                IndexInstructions::RemoveDoc(name) => {
                                    println!("[INDEX] Removing document: {}", name);
                                    self.remove_doc(name);
                                    self.set_docs();
                                }
                                IndexInstructions::Refresh => {
                                    println!("[INDEX] Refreshing docs");
                                    let instruction = IndexInstructions::Docs(self.docs.clone());
                                    let bytes = instruction.to_bytes();

                                    if let Err(e) = self.cluster.publish(INDEX_CHANNEL, &bytes) {
                                        eprintln!("[INDEX] Error publishing refresh: {:?}", e);
                                        // Decide whether to break or continue based on your error handling strategy
                                    }
                                }
                                IndexInstructions::Docs(_) => {
                                    println!(
                                        "[INDEX] Instrucción Docs recibida (sin acción en el microservicio)"
                                    );
                                }
                            }
                        } else {
                            println!("[INDEX] Failed to parse instruction from bytes");
                        }
                    } else {
                        println!("[INDEX] Received non-SimpleString message");
                    }

                    let docs_clonados: Vec<Document> = self.docs.clone();
                    for doc in docs_clonados {
                        self.ensure_service_running(&doc);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No data available, sleep briefly and try again
                    thread::sleep(Duration::from_millis(5));
                    continue;
                }
                Err(e) => {
                    eprintln!("[INDEX] Error reading from socket: {}", e);
                    break;
                }
            }
        }

        Err(IndexError::ChannelClosed)
    }

    fn init_service(&self, doc: &Document) -> JoinHandle<()> {
        let (sx, _) = channel();
        
        /*// Usar la dirección apropiada según el entorno
        let node_address = if std::env::var("DOCKER_MODE").is_ok() {
            "node_1:7001".to_string()
        } else {
            "0.0.0.0:7001".to_string()
        };*/

        //Que tome el nodo activo del index
        let node_address = self.cluster.node_address.clone();
        
        println!("[INDEX] Iniciando servicio para documento '{}' con dirección: {}", doc.get_name(), node_address);
        
        match doc.get_type() {
            DocType::Text => {
                let mut service: Service<String, TextOperation> = Service::new(
                    doc.get_name().to_string(),
                    doc.get_name().to_string(),
                    doc.get_name().to_string(),
                    node_address,
                    sx,
                )
                .unwrap();
                thread::spawn(move || service.run())
            }
            DocType::SpreadSheet => {
                let mut service: Service<SpreadSheet, SpreadOperation> = Service::new(
                    doc.get_name().to_string(),
                    doc.get_name().to_string(),
                    doc.get_name().to_string(),
                    node_address,
                    sx,
                )
                .unwrap();
                thread::spawn(move || service.run())
            }
        }
    }

    fn add_doc(&mut self, doc: Document) {
        let doc_name = doc.get_name();
        if self.docs.iter().any(|d| d.get_name() == doc_name) {
            println!(
                "[INDEX] Ya existe un documento con el nombre '{}', no se creará otro.",
                doc_name
            );
            return;
        }
        let doc_clon = doc.clone();
        self.ensure_service_running(&doc_clon);
        self.docs.push(doc);
        self.set_docs();
        // Publicar la lista actualizada
        let instruction =
            crate::app::index::index_instructions::IndexInstructions::Docs(self.docs.clone());
        let bytes = instruction.to_bytes();
        let _ = self.cluster.publish(INDEX_CHANNEL, &bytes);
    }

    fn remove_doc(&mut self, doc_name: String) {
        for i in 0..self.docs.len() {
            if self.docs[i].get_name() == doc_name {
                self.docs.remove(i);
                break;
            }
        }
        self.set_docs();

        let instruction = IndexInstructions::Docs(self.docs.clone());
        let bytes = instruction.to_bytes();
        let _ = self.cluster.publish(INDEX_CHANNEL, &bytes);
        let _ = self.cluster.del(&doc_name);
    }

    fn set_docs(&mut self) {
        let docs_bytes = self.docs.to_bytes();
        match self.cluster.set(DOC_KEY, &docs_bytes) {
            Ok(_) => {
                println!(
                    "[INDEX] Documentos guardados correctamente. Total: {}",
                    self.docs.len()
                );
            }
            Err(e) => {
                eprintln!("[INDEX] Error guardando documentos: {:?}", e);
            }
        }
    }

    fn get_docs(&mut self) -> Option<Documents> {
        println!("[INDEX] Buscando documentos en el cluster...");
        match self.cluster.get(DOC_KEY) {
            Ok(bytes) => {
                println!("[INDEX] Bytes encontrados, intentando parsear...");
                match Documents::from_bytes(&bytes) {
                    Some((docs, _)) => {
                        println!(
                            "[INDEX] Documentos parseados correctamente. Total: {}",
                            docs.len()
                        );
                        Some(docs)
                    }
                    None => {
                        eprintln!(
                            "[INDEX] Error al parsear los documentos desde los bytes recuperados."
                        );
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "[INDEX] Error al obtener los documentos del cluster: {:?}",
                    e
                );
                None
            }
        }
    }
}

impl Drop for Index {
    fn drop(&mut self) {
        self.set_docs();
    }
}
