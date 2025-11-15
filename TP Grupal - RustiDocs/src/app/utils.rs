use crate::cluster::types::DEFAULT_BUFFER_SIZE;
use crate::network::RespMessage;
use crate::network::resp_parser::parse_resp_line;
use crate::parser::response_parser::format_resp_message;
use std::io::{BufReader, Error, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

static QUEUE_LIMIT: usize = 20;

/// Conecta al usuario al nodo como cliente, retorna el stream y un booleano
/// indicando, `true` si el usuario es escritura o `false` si es de solo lectura.
pub fn connect_to_cluster(
    address: String,
    username: String,
    password: String,
) -> Result<(TcpStream, bool), Error> {
    let stream = TcpStream::connect(address);
    thread::sleep(Duration::from_millis(150)); // Espero que la conexión se inicie

    if let Ok(mut stream) = stream {
        // Autentico
        let auth_cmd = format!("AUTH {} {}", username, password);
        let cmd = format_resp_message(auth_cmd.as_str()).unwrap();
        stream.write_all(cmd.as_bytes())?;
        stream.flush()?;

        // Reviso
        let mut buffer = [0; DEFAULT_BUFFER_SIZE];
        match stream.read(&mut buffer) {
            Ok(n) => {
                let mut reader = BufReader::new(&buffer[..n]);
                let res = parse_resp_line(&mut reader).unwrap();
                match res {
                    RespMessage::SimpleString(msg) => {
                        println!("\x1b[32m[AUTH] Autenticado\x1b[0m");
                        if msg == "Usuario logeado correctamente - WRITE" {
                            Ok((stream, true))
                        } else {
                            Ok((stream, false))
                        }
                    }
                    _ => {
                        println!("\x1b[31m[AUTH] Usuario y/o contraseña incorrectos\x1b[0m");
                        Err(Error::new(ErrorKind::Other, "Error al autenticar"))
                    }
                }
            }
            Err(_) => {
                println!("[AUTH] Error al recibir respuesta");
                Err(Error::new(
                    ErrorKind::Other,
                    "Error al recibir respuesta de logueo",
                ))
            }
        }
    } else {
        println!("Error al conectar");
        Err(Error::new(ErrorKind::Other, "Error al conectar"))
    }
}

// Son un conjunto de threads que corren lo que les pase
// no se crean ni destruyen para justamente ahorrar ese tiempo extra
pub struct ThreadPool {
    pool_size: usize,
    pool: Vec<JoinHandle<()>>,
    thread_queue: Arc<(Mutex<Vec<Box<dyn FnOnce() + Send + 'static>>>, Condvar)>,
    receiver: Arc<Mutex<Receiver<Box<dyn FnOnce() + Send + 'static>>>>,
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.finish();
    }
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        let (tx, rx) = channel::<Box<dyn FnOnce() + Send + 'static>>();
        let receiver = Arc::new(Mutex::new(rx));
        let pool = Self::create_worker_threads(size, Arc::clone(&receiver));
        let thread_queue = Arc::new((Mutex::new(Vec::new()), Condvar::new()));

        let thread_queue_clone = thread_queue.clone();

        let res = ThreadPool {
            pool_size: size,
            pool,
            thread_queue,
            receiver,
        };

        thread::spawn(move || {
            Self::start_queueing(thread_queue_clone, tx);
        });

        res
    }

    pub fn start_queueing(thread_queue_lock: Arc<(Mutex<Vec<Box<dyn FnOnce() + Send + 'static>>>, Condvar)>, sender: Sender<Box<dyn FnOnce() + Send + 'static>>) {
        let (queue_lock, condvar) = &*thread_queue_lock;
        loop {
            let mut queue = queue_lock.lock().unwrap();
            while queue.is_empty() {
                queue = condvar.wait(queue).unwrap(); // Libera el mutex, no hay deadlock
            }
            let task = queue.pop();
            if let Err(_) = sender.send(task.unwrap()) {
                println!("Error al enviar tarea al pool");
            }
            drop(queue);
        }
    }

    pub(crate) fn spawn<F>(&mut self, f: F) -> Result<(), Error>
    where
        F: FnOnce() + Send + 'static,
    {
        self.maintain_pool();
        let (queue_lock, condvar) = &*self.thread_queue;
        let mut queue = queue_lock.lock().unwrap();
        if queue.len() >= QUEUE_LIMIT {
            return Err(Error::new(
                ErrorKind::Other,
                "No hay suficientes threads disponibles",
            ));
        }

        queue.push(Box::new(f));
        drop(queue);
        condvar.notify_one();
        Ok(())
    }

    fn maintain_pool(&mut self) {
        self.pool.retain(|thread| !thread.is_finished());

        let needed = self.pool_size - self.pool.len();
        if needed > 0 {
            let new_threads = Self::create_worker_threads(needed, Arc::clone(&self.receiver));
            self.pool.extend(new_threads);
        }
    }

    fn create_worker_threads(
        count: usize,
        receiver: Arc<Mutex<Receiver<Box<dyn FnOnce() + Send + 'static>>>>,
    ) -> Vec<JoinHandle<()>> {
        (0..count)
            .map(|_| Self::spawn_worker(Arc::clone(&receiver)))
            .collect()
    }

    fn spawn_worker(
        receiver: Arc<Mutex<Receiver<Box<dyn FnOnce() + Send + 'static>>>>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            loop {
                let job = receiver.lock().unwrap().recv();
                match job {
                    Ok(task) => task(),
                    Err(_) => break,
                }
            }
        })
    }

    fn finish(&mut self) {
        for t in self.pool.drain(..) {
            let _ = t.join();
        }
    }
}
