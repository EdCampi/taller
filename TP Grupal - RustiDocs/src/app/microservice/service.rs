use crate::{
    app::{
        microservice::{
            control::control_service::ControlService, control_instructions::ControlInstruction,
        },
        network::{
            header::{InstructionType, Message},
            redis_parser::content_to_message,
        },
        operation::generic::{Applicable, ParsableBytes, Transformable},
    },
    client_lib::cluster_manager::ClusterManager,
    network::resp_parser::parse_resp_line,
};
use std::{
    io::{BufReader, Write},
    net::TcpStream,
    sync::mpsc::Sender,
};

const VERSION_TO_SAVE: u64 = 1;

#[derive(Debug)]
pub struct Service<D, O>
where
    O: Applicable<D> + Transformable + Clone + ParsableBytes + std::fmt::Debug,
    D: ParsableBytes + Clone + Default + std::fmt::Debug,
{
    pub doc_name: String,
    pub doc_hash: String,
    pub doc_channel: String,
    cluster_data: ClusterManager,
    pub redis_stream: TcpStream,
    pub control_service: ControlService<D, O>,
    // TODO: state_sender: Sender<ControlInstruction>,
    pub delta_version: u64,
}

impl<D, O> Service<D, O>
where
    O: Applicable<D> + Transformable + Clone + ParsableBytes + std::fmt::Debug,
    D: ParsableBytes + Clone + Default + std::fmt::Debug,
{
    pub fn new(
        doc_name: String,
        doc_hash: String,
        doc_channel: String,
        redis_address: String,
        _: Sender<ControlInstruction>,
    ) -> Result<Self, std::io::Error> {
        let mut cluster_data =
            ClusterManager::new(redis_address, "super".to_string(), "1234".to_string()).unwrap(); // TODO: HARDCODEADO

        let mut data = D::default();

        if let Ok(data_get) = cluster_data.get(&doc_hash) {
            (data, _) = D::from_bytes(&data_get).unwrap_or((data, 0));
        }

        let sub_channel = cluster_data.subscribe(&doc_channel).unwrap();

        Ok(Service {
            doc_name,
            doc_hash,
            doc_channel,
            redis_stream: sub_channel,
            cluster_data,
            control_service: ControlService::new(data),
            delta_version: 0,
            //state_sender,
        })
        /*
        // Conexion inicial al address
        let mut redis_stream = TcpStream::connect(redis_address)?;

        // Modularizar de aca la parte de la data
        let data: Option<D> = get_data(&mut redis_stream, doc_name.to_string());
        let data = if data.is_none() {
            println!("No se encontro hash, se inicializa por default");
            D::default()
        } else {
            println!("Se encontro hash");
            data.unwrap()
        };
        // Aca termina la parte de la da ta

        //Aca se trata de subscribir al canal
        let mensaje = &sub_to_channel(&doc_channel);
        println!("Voy a enviar {:?} a redis",mensaje);
        redis_stream.write_all(&mensaje)?;

        // Suponemos que ya estamos subscritos y creamos la instancia
        */
    }

    pub fn run(&mut self) {
            let mut reader = BufReader::new(self.redis_stream.try_clone().unwrap());
        loop {
                    match parse_resp_line(&mut reader) {
                        Err(e) => {
                            eprintln!("Error leyendo del socket: {}", e);
                            let _error_msg = format!("[SERVICE] Error: {}", e);
                            let pub_message =
                                Message::<D, O>::Resync.message_to_pub(&self.doc_channel);
                            let _ = self.redis_stream.write_all(&pub_message);
                            break;
                        }
                        Ok(contenido) => {
                            if let Some(_message) = content_to_message::<D, O>(contenido) {
                                println!("[SERVICE] Message parseado correctamente");
                                match _message {
                                    Message::Instruction(instruction_type, instruction) => {
                                        match instruction_type {
                                            InstructionType::Response => {
                                                println!(
                                                    "Entró en InstructionType::Response, ignorando response propia {:?}",
                                                    instruction
                                                );
                                                continue;
                                            }
                                            InstructionType::Request => {
                                                println!(
                                                    "Entró en InstructionType::Request, aplicando instrucción recibida {:?}",
                                                    instruction
                                                );
                                                let instruction = self
                                                    .control_service
                                                    .apply_operation(instruction)
                                                    .unwrap();
                                                let response: Message<D, O> =
                                                    Message::create_response(instruction);
                                                println!("Creo la instruccion y trato de enviarla");
                                                let pub_message =
                                                    response.message_to_pub(&self.doc_channel);
                                               self.redis_stream.write_all(&pub_message).unwrap();
                                                if self.delta_version >= VERSION_TO_SAVE {
                                                    println!("Trato de guardar");
                                                    self.delta_version = 0;
                                                    self.save_data();
                                                    println!("Ya guarde");
                                                } else {
                                                    self.delta_version += 1;
                                                    println!(
                                                        "Sumo al delta y queda {}",
                                                        self.delta_version
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    Message::Init(client_id) => {
                                        println!(
                                            "[SERVICE] Recibido Init de cliente {}",
                                            client_id
                                        );
                                        let data = self.control_service.data.clone();
                                        let version = self.control_service.version;
                                        let state: Message<D, O> =
                                            Message::State(data, version, client_id);
                                        let pub_message = state.message_to_pub(&self.doc_channel);
                                        let _ = self.redis_stream.write_all(&pub_message);
                                        println!("[SERVICE] Enviado State a cliente {}", client_id);
                                    }
                                    _ => {
                                        println!("[SERVICE] Mensaje no reconocido o no relevante");
                                        continue;
                                    }
                                }
                            } else {
                                println!("[SERVICE] No se pudo parsear el mensaje a Message<D, O>");
                            }
                        }
                    }
                }
                
            }
        
    

    fn save_data(&mut self) {
        let bytes = self.control_service.data.to_bytes();
        let _ = self.cluster_data.set(&self.doc_name, &bytes);
    }
}

impl<D, O> Drop for Service<D, O>
where
    O: Applicable<D> + Transformable + Clone + ParsableBytes + std::fmt::Debug,
    D: ParsableBytes + Clone + Default + std::fmt::Debug,
{
    fn drop(&mut self) {
        self.save_data();
    }
}
