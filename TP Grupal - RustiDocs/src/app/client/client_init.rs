use crate::app::client::client_data::Client;
use crate::app::client::client_input::ClientInput;
use crate::app::client::client_output::ClientOutput;
use crate::app::network::header::Message;
use crate::app::network::redis_parser::content_to_message;
use crate::app::network::redis_parser::sub_to_channel;
use crate::app::operation::generic::Applicable;
use crate::app::operation::generic::Instruction;
use crate::app::operation::generic::ParsableBytes;
use crate::app::operation::generic::Transformable;
use crate::cluster::types::DEFAULT_BUFFER_SIZE;
use crate::network::resp_parser::parse_resp_line;
use std::io::Write;
use std::io::{BufReader, Read};
use std::net::TcpStream;
use std::sync::mpsc::Sender;
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::thread::JoinHandle;

pub struct ClientThread {
    _input_join: JoinHandle<()>,
    _output_join: JoinHandle<()>,
}

impl ClientThread {
    pub fn init<D, O>(
        client_id: u64,
        redis_stream: &mut TcpStream,
        channel_name: String,
    ) -> Result<(Client<D, O>, Receiver<Instruction<O>>), String>
    where
        O: Applicable<D> + Transformable + Clone + ParsableBytes + Send + 'static + std::fmt::Debug,
        D: Clone + ParsableBytes + 'static,
    {
        let _ = redis_stream.write_all(&sub_to_channel(&channel_name));
        redis_stream.flush().unwrap();
        let mut buffer = [0; DEFAULT_BUFFER_SIZE];
        match redis_stream.read(&mut buffer) {
            Ok(0) => {
                println!("[INIT] No hay datos en el socket");
                return Err("[INIT] No hay datos en el socket".to_string());
            }
            Ok(n) => {
                let mut reader = BufReader::new(&buffer[..n]);
                println!("READER SUB {:?}", reader);
                match parse_resp_line(&mut reader) {
                    Ok(contenido) => {
                        println!("CONTEIDO {:?}", contenido);
                    }
                    Err(_e) => {}
                }
            }
            Err(e) => {
                println!("[INIT] Error leyendo del socket: {}", e);
                return Err("[INIT] Error leyendo del socket".to_string());
            }
        }

        println!("[INIT] Enviando init message");
        let init_message = Message::<D, O>::Init(client_id).message_to_pub(&channel_name);
        println!("El init message es {:?}", &init_message);
        let _ = redis_stream.write_all(&init_message);
        redis_stream.flush().unwrap();
        println!("[INIT] Subscribiendo a canal");
        let (data, version) = get_state::<D, O>(client_id, redis_stream)?;
        println!("[INIT] Data");
        let (input, receiver) = init_input::<D, O>(&redis_stream, client_id);
        let (output, sender) = init_output::<D, O>(&redis_stream, channel_name, client_id);
        println!("[INIT] Output: {:?}", output);
        let client = Client::new(data, sender.clone(), version, client_id);
        println!("[INIT] Client");
        // ACA HAY QUE MANEJAR THREADS PERO BUENO
        Self {
            _input_join: input,
            _output_join: output,
        };
        println!("[INIT] Retornando Ok");
        Ok((client, receiver))
    }
}

fn init_input<D, O>(
    socket: &TcpStream,
    client_id: u64,
) -> (JoinHandle<()>, Receiver<Instruction<O>>)
where
    O: Clone + ParsableBytes + Send + 'static + std::fmt::Debug,
    D: Clone + ParsableBytes,
{
    let (sender, receiver) = channel();
    let socket_clone = socket.try_clone().unwrap();
    let join = thread::spawn(move || {
        let mut input: ClientInput<D, O> = ClientInput::new(socket_clone, sender, client_id);
        input.run();
    });

    (join, receiver)
}

fn init_output<D, O>(
    socket: &TcpStream,
    channel_name: String,
    client_id: u64,
) -> (JoinHandle<()>, Sender<Instruction<O>>)
where
    O: Clone + ParsableBytes + Send + 'static,
    D: ParsableBytes,
{
    let (sender, receiver) = channel();
    let socket_clone = socket.try_clone().unwrap();
    let join = thread::spawn(move || {
        let mut input: ClientOutput<D, O> =
            ClientOutput::new(socket_clone, receiver, channel_name, client_id);
        input.run();
    });

    (join, sender)
}

fn get_state<D, O>(client_id: u64, stream: &mut TcpStream) -> Result<(D, u64), String>
where
    O: Clone + ParsableBytes,
    D: Clone + ParsableBytes + 'static,
{
    let mut reader = BufReader::new(stream);
    loop {
        match parse_resp_line(&mut reader) {
            Err(e) => {
                eprintln!("[INIT] Error leyendo del socket: {}", e);
                println!("Client: Entró en rama Err de parse_resp_line");
                return Err(format!("[INIT] Error leyendo del socket: {}", e));
            }
            Ok(contenido) => {
                println!("Client: Entró en rama Ok de parse_resp_line");
                if let Some(Message::State(mut data, version, id)) =
                    content_to_message::<D, O>(contenido)
                {
                    if id == client_id {
                        // Inicialización robusta para SpreadSheet
                        if let Some(sheet) = any_as_mut_spreadsheet(&mut data) {
                            if sheet.data.is_empty() {
                                sheet.data.push(vec![String::new()]);
                            }
                        }
                        println!(
                            "Client: id {} coincide con client_id {}, retornando Ok",
                            id, client_id
                        );
                        return Ok((data, version));
                    } else {
                        println!(
                            "Client: id {} NO coincide con client_id {}, continuando",
                            id, client_id
                        );
                        continue;
                    }
                } else {
                    println!(
                        "Client: Mensaje no es State o no es válido, ignorando y esperando el mensaje correcto"
                    );
                    continue;
                }
            }
        }
    }
}

// Helper para inicialización robusta solo si D es SpreadSheet
#[allow(dead_code)]
fn any_as_mut_spreadsheet<D: 'static>(
    data: &mut D,
) -> Option<&mut crate::app::operation::csv::SpreadSheet> {
    use std::any::Any;
    (data as &mut dyn Any).downcast_mut::<crate::app::operation::csv::SpreadSheet>()
}
