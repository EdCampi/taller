/*use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;
use std::collections::HashMap;


type ConectionThread = (InputThread,OutputThread);

type InputThread = JoinHandle<()>;
type OutputThread = JoinHandle<()>;


#[derive(Debug)]
pub enum SupervisorInstruction {
    Add(String, ConectionThread),
    Terminate(String),
    TerminateItself(),
}



pub struct Supervisor{
    receiver: Receiver<SupervisorInstruction>, // Es el id
    conexiones_activas: HashMap<String,(InputThread,OutputThread)>
}


impl Supervisor {
    pub fn new(receiver: Receiver<SupervisorInstruction>) ->Self {
        Self{
             receiver,
            conexiones_activas: HashMap::new()
        }
    }


    pub fn init(&mut self){
        for instruction in &self.receiver{
            match instruction{
                SupervisorInstruction::Add(id, tuple_threads ) =>{
                    self.conexiones_activas.insert(id, tuple_threads);
                }
                SupervisorInstruction::Terminate(id) =>{
                    if let Some(conections) = self.conexiones_activas.remove(&id){
                        let (input_thread,output_thread) = conections;
                        let _ = input_thread.join();
                        let _ = output_thread.join();
                    }
                }
                SupervisorInstruction::TerminateItself() => {
                    break;
                }
            }
        }
    }
}*/
