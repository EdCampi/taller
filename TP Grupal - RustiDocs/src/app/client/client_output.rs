use crate::app::network::header::Message;
use crate::app::operation::generic::{Instruction, ParsableBytes};
use std::io::Write;
use std::net::TcpStream;
use std::sync::mpsc::Receiver;

use std::marker::PhantomData;

pub struct ClientOutput<D, O>
where
    O: Clone + ParsableBytes,
    D: ParsableBytes,
{
    pub socket: TcpStream,
    pub receiver: Receiver<Instruction<O>>,
    pub channel_name: String,
    _client_id: u64,
    _marker: PhantomData<D>,
}

impl<D, O> ClientOutput<D, O>
where
    O: Clone + ParsableBytes,
    D: ParsableBytes,
{
    pub fn new(
        socket: TcpStream,
        receiver: Receiver<Instruction<O>>,
        channel_name: String,
        client_id: u64,
    ) -> Self {
        ClientOutput {
            socket,
            receiver,
            channel_name,
            _client_id: client_id,
            _marker: PhantomData,
        }
    }

    pub fn run(&mut self) {
        for instruction in self.receiver.iter() {
            let message: Message<D, O> = Message::create_request(instruction);
            let pub_message = message.message_to_pub(&self.channel_name);
            self.socket.write(&pub_message).unwrap();
        }
    }
}
