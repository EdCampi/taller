//! Esta interfaz permite interactuar con el cliente Redis,
//! incluyendo la visualización de datos, la edición de archivos
//! y la gestión de documentos.
//!
//! # Uso
//! cargo run --bin interfaz -- 1 (o el número del nodo)

use eframe::egui::{self, Visuals};
use rustidocs::app::client::client_data::Client;
use rustidocs::app::client::client_init::ClientThread;
use rustidocs::app::operation::generic::{Instruction};
use rustidocs::app::operation::text::TextOperation;
use std::fs;
use std::io::{Error, ErrorKind};
use std::net::TcpStream;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use std::{env, path::PathBuf};
// Al inicio del archivo
use rustidocs::app::client::client_index::ClientIndex;
use rustidocs::app::client::llm_client::LLMClient;
use rustidocs::app::index::document::DocType;
use rustidocs::app::operation::csv::{SpreadOperation, SpreadSheet};

use rfd::FileDialog;
use rustidocs::app::index::documents::Documents;
use rustidocs::app::utils::connect_to_cluster;

/// Detecta si Docker está corriendo y retorna la configuración apropiada
fn detect_docker_environment() -> (String, String) {
    // Verificar si hay contenedores Docker corriendo en el puerto 7001
    let host_addr = "localhost:7001";
    
    if test_connection(host_addr) {
        // Verificar si podemos hacer un comando básico al cluster para confirmar que es Docker
        if is_docker_cluster(host_addr) {
            println!("[Docker Detection] Detectado cluster Docker en: {}", host_addr);
            return ("localhost".to_string(), "7001".to_string());
        }
    }
    
    // Por defecto, usar localhost
    println!("[Docker Detection] Usando configuración por defecto: localhost:7001");
    ("localhost".to_string(), "7001".to_string())
}

/// Verifica si el cluster en la dirección dada es un cluster Docker
fn is_docker_cluster(addr: &str) -> bool {
    use std::net::TcpStream;
    use std::io::Write;
    
    if let Ok(socket_addr) = addr.parse::<std::net::SocketAddr>() {
        if let Ok(mut stream) = TcpStream::connect(socket_addr) {
            // Enviar un comando AUTH para verificar que es nuestro cluster
            let auth_cmd = b"AUTH super 1234\r\n";
            if stream.write_all(auth_cmd).is_ok() {
                return true;
            }
        }
    }
    false
}

/// Prueba una conexión TCP básica sin usar imports externos
fn test_connection(addr: &str) -> bool {
    use std::net::TcpStream;
    use std::time::Duration;
    
    // Intentar parsear la dirección de forma segura
    if let Ok(socket_addr) = addr.parse::<std::net::SocketAddr>() {
        match TcpStream::connect_timeout(&socket_addr, Duration::from_millis(100)) {
            Ok(_) => true,
            Err(_) => false,
        }
    } else {
        false
    }
}

fn main() -> Result<(), eframe::Error> {
    let args: Vec<String> = env::args().collect();

    let client_id: u64;
    if args.len() >= 2 {
        println!("Se recibe la id: {}", args[1]);
        client_id = args[1].parse::<u64>().unwrap_or(0);
        println!("Se crea cliente con id: {}", client_id);
    } else {
        client_id = 0;
    }

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Redis Cluster",
        options,
        Box::new({
            let client_id_clone = client_id.clone();
            move |_cc| Box::new(RedisApp::new(client_id_clone))
        }),
    )
}

#[derive(PartialEq)]
enum CurrentView {
    Login,
    MainApp,
    TextEditor,
    SpreadsheetEditor,
}

struct RedisApp {
    client_id: u64,
    current_view: CurrentView,
    username: String,
    password: String,
    redis_stream: Option<TcpStream>,
    login_error_message: String,
    text_editor_content: String,
    //text_editor_filename: String,
    spreadsheet_data: SpreadSheet,
    open_text_file_requestd: bool,
    open_csv_file_requested: bool,
    watched_file_path: Arc<Mutex<Option<PathBuf>>>,
    file_events_rx: Arc<Mutex<Receiver<String>>>,
    file_notifications: Arc<Mutex<Vec<String>>>,
    //last_file_content: Arc<Mutex<Option<String>>>,
    previous_spreadsheet_data: SpreadSheet,
    //show_remote_join_dialog: bool,
    remote_filename: String,
    remote_ip: String,
    remote_port: String,
    remote_address: String,
    //show_creatio_button: bool,
    text_data: Option<Client<String, TextOperation>>,
    text_remote: Option<Receiver<Instruction<TextOperation>>>,
    // Para CSV - cambiar a SpreadSheet y SpreadOperation
    csv_data: Option<Client<SpreadSheet, SpreadOperation>>,
    csv_remote: Option<Receiver<Instruction<SpreadOperation>>>,
    // Para archivos
    available_documents: Option<Documents>,
    client_index: Option<ClientIndex>,
    document_receiver: Option<Receiver<Documents>>,
    show_document_creation_dialog: bool,
    new_document_name: String,
    new_document_type: DocType,
    modo_lectura: bool,
    // Campos para AI
    llm_client: Option<LLMClient>,
    ai_prompt: String,
    show_ai_dialog: bool,
    ai_position: usize,
    ai_error_message: String,
    selected_text: String,
    show_text_selection: bool,
    ai_response: Option<String>,
    show_ai_response_dialog: bool,
    //process_ai_request_for_selected: bool,
}

impl RedisApp {
    fn new(client_id: u64) -> Self {
        let (_, rx) = mpsc::channel();
        let watched_file_path = Arc::new(Mutex::new(None));

        // Detectar si Docker está corriendo para usar la IP apropiada
        let (remote_ip, remote_port) = detect_docker_environment();
        println!("🐳 Configurando conexión: {}:{}", remote_ip, remote_port);
        let remote_address = format!("{}:{}", remote_ip, remote_port);

        Self {
            client_id,
            current_view: CurrentView::Login,
            username: String::new(),
            password: String::new(),
            redis_stream: None,
            login_error_message: String::new(),
            text_editor_content: String::new(),
            //text_editor_filename: "untitled.txt".to_string(),
            open_text_file_requestd: false,
            open_csv_file_requested: false,
            watched_file_path,
            file_events_rx: Arc::new(Mutex::new(rx)),
            file_notifications: Arc::new(Mutex::new(Vec::new())),
            //last_file_content,
            spreadsheet_data: SpreadSheet::default(),
            previous_spreadsheet_data: SpreadSheet::default(),
            //show_remote_join_dialog: false,
            remote_filename: String::new(),
            remote_ip,
            remote_port,
            remote_address,
            //show_creatio_button: false,
            text_data: None,
            text_remote: None,
            csv_data: None,
            csv_remote: None,
            available_documents: None,
            client_index: None,
            document_receiver: None,
            show_document_creation_dialog: false,
            new_document_name: String::new(),
            new_document_type: DocType::Text,
            modo_lectura: false,
            // Campos para AI
            llm_client: None,
            ai_prompt: String::new(),
            show_ai_dialog: false,
            ai_position: 0,
            ai_error_message: String::new(),
            selected_text: String::new(),
            show_text_selection: false,
            ai_response: None,
            show_ai_response_dialog: false,
            //process_ai_request_for_selected: false,
        }
    }

    // MODIFICADO: Lógica de detección de cambios reemplazada por una versión robusta
    // que utiliza el algoritmo de "Longest Common Subsequence" (LCS) internamente.
    // Este enfoque es el estándar para sistemas de edición colaborativa.
    fn apply_new_changes_on_file(&mut self, _ctx: &egui::Context) {
        if let Some(text_data) = &mut self.text_data {
            let current_content = self.text_editor_content.clone();
            let stored_content = text_data.local_data.clone();

            if current_content != stored_content {
                // Convertimos las cadenas a vectores de caracteres para trabajar con índices de caracteres
                let current_chars: Vec<char> = current_content.chars().collect();
                let stored_chars: Vec<char> = stored_content.chars().collect();

                // Creamos un mapeo de índices de caracteres a índices de bytes en la cadena original
                let mut byte_indices = Vec::new();
                let mut char_pos = 0;
                for (byte_pos, _) in stored_content.char_indices() {
                    while char_pos < byte_pos {
                        byte_indices.push(byte_pos);
                        char_pos += 1;
                    }
                    byte_indices.push(byte_pos);
                    char_pos += 1;
                }
                byte_indices.push(stored_content.len()); // Añadir el final

                // 1. Encontrar el prefijo común
                let mut prefix_len = 0;
                while prefix_len < current_chars.len().min(stored_chars.len())
                    && current_chars[prefix_len] == stored_chars[prefix_len]
                {
                    prefix_len += 1;
                }

                // 2. Encontrar el sufijo común (trabajando desde el final)
                let mut suffix_len = 0;
                while suffix_len
                    < (current_chars.len() - prefix_len).min(stored_chars.len() - prefix_len)
                    && current_chars[current_chars.len() - 1 - suffix_len]
                        == stored_chars[stored_chars.len() - 1 - suffix_len]
                {
                    suffix_len += 1;
                }

                // 3. Determinar qué ha cambiado
                let old_mid_start = prefix_len;
                let old_mid_end = stored_chars.len() - suffix_len;
                let new_mid_start = prefix_len;
                let new_mid_end = current_chars.len() - suffix_len;

                // 4. Primero eliminar caracteres viejos (de atrás hacia adelante)
                // para evitar que los índices se invaliden
                for i in (old_mid_start..old_mid_end).rev() {
                    // Usamos el mismo índice i, que es un índice de carácter, no de bytes
                    let delete_op = TextOperation::Delete { position: i };
                    text_data.apply_local_operation(delete_op);
                    self.file_notifications
                        .lock()
                        .unwrap()
                        .push(format!("Eliminación en posición {}", i));
                }

                // 5. Luego insertar los nuevos caracteres (de principio a fin)
                for (j, &ch) in current_chars[new_mid_start..new_mid_end].iter().enumerate() {
                    let pos = old_mid_start + j; // índice de carácter, no de bytes
                    let insert_op = TextOperation::Insert {
                        position: pos,
                        character: ch,
                    };
                    text_data.apply_local_operation(insert_op);
                    self.file_notifications
                        .lock()
                        .unwrap()
                        .push(format!("Inserción de '{}' en posición {}", ch, pos));
                }

                // Finalmente, actualizar el contenido del editor
                self.text_editor_content = text_data.local_data.clone();
            }

            // Procesar operaciones remotas
            if let Some(remote) = &self.text_remote {
                for instruction in remote.try_iter() {
                    text_data.receive_remote_instruction(instruction.clone());
                    self.text_editor_content = text_data.local_data.clone();
                    self.file_notifications.lock().unwrap().push(format!(
                        "Operación remota del cliente {} (op: {})",
                        instruction.operation_id.client_id, instruction.operation_id.local_seq
                    ));
                }
            }
        }
    }

    fn create_text_client_data(&mut self, mut stream: TcpStream) {
        if let Ok((client_data, remote_receiver)) = ClientThread::init::<String, TextOperation>(
            self.client_id,
            &mut stream,
            self.remote_filename.to_string(),
        ) {
            println!("ok!");
            self.text_editor_content = client_data.local_data.clone();
            self.text_data = Some(client_data);
            self.text_remote = Some(remote_receiver);
        }
    }

    fn create_csv_client_data(&mut self, mut stream: TcpStream) {
        if let Ok((client_data, remote_receiver)) = ClientThread::init::<SpreadSheet, SpreadOperation>(
            self.client_id,
            &mut stream,
            self.remote_filename.to_string(),
        ) {
            println!("ok!");
            self.spreadsheet_data = client_data.local_data.clone();
            self.csv_data = Some(client_data);
            self.csv_remote = Some(remote_receiver);
        }
    }

    fn send_ai_request(&mut self) {
        if self.ai_prompt.is_empty() {
            self.ai_error_message = "El prompt no puede estar vacío".to_string();
            return;
        }

        // Inicializar cliente LLM si no existe
        if self.llm_client.is_none() {
            match LLMClient::new(&self.remote_address, "super", "1234") {
                Ok(client) => self.llm_client = Some(client),
                Err(e) => {
                    self.ai_error_message = format!("Error conectando al servicio LLM: {}", e);
                    return;
                }
            }
        }

        if let Some(client) = &mut self.llm_client {
            let result = if self.ai_position == 0 {
                // Reemplazar todo el documento
                client.request_ai_replace(
                    self.remote_filename.clone(),
                    self.ai_prompt.clone(),
                    self.client_id,
                )
            } else {
                // Insertar en posición específica
                client.request_ai_insert(
                    self.remote_filename.clone(),
                    self.ai_prompt.clone(),
                    self.ai_position,
                    self.client_id,
                )
            };

            match result {
                Ok(response) => {
                    self.ai_response = Some(response);
                    self.show_ai_response_dialog = true;
                    self.ai_error_message.clear();
                }
                Err(e) => {
                    self.ai_error_message = format!("Error de AI: {}", e);
                }
            }
        }
    }

    fn send_ai_request_for_selected_text(&mut self) {
        if self.ai_prompt.is_empty() {
            self.ai_error_message = "El prompt no puede estar vacío".to_string();
            return;
        }

        if self.selected_text.is_empty() {
            self.ai_error_message = "Debes seleccionar algún texto".to_string();
            return;
        }

        // Inicializar cliente LLM si no existe
        if self.llm_client.is_none() {
            match LLMClient::new(&self.remote_address, "super", "1234") {
                Ok(client) => self.llm_client = Some(client),
                Err(e) => {
                    self.ai_error_message = format!("Error conectando al servicio LLM: {}", e);
                    return;
                }
            }
        }

        if let Some(client) = &mut self.llm_client {
            match client.request_ai_replace_selected(
                self.remote_filename.clone(),
                self.ai_prompt.clone(),
                self.selected_text.clone(),
                self.client_id,
            ) {
                Ok(response) => {
                    self.ai_response = Some(response);
                    self.show_ai_response_dialog = true;
                    self.ai_error_message.clear();
                }
                Err(e) => {
                    self.ai_error_message = format!("Error de AI: {}", e);
                }
            }
        }
    }

    fn apply_ai_response(&mut self) {
        if let Some(response) = &self.ai_response {
            if let Some(text_data) = &mut self.text_data {
                // Primero, asegurarnos de que todos los cambios pendientes se han aplicado
                let current_content = self.text_editor_content.clone();
                let stored_content = text_data.local_data.clone();
                
                // Si hay diferencias, aplicar cambios pendientes primero
                if current_content != stored_content {
                    let current_chars: Vec<char> = current_content.chars().collect();
                    let stored_chars: Vec<char> = stored_content.chars().collect();

                    // Si el usuario borró todo el texto de golpe
                    if current_chars.is_empty() && !stored_chars.is_empty() {
                        text_data.apply_local_operation(TextOperation::DeleteAll);
                    } else {
                        // Aplicar algoritmo de diferencias (similar a apply_new_changes_on_file)
                        let mut prefix_len = 0;
                        while prefix_len < current_chars.len().min(stored_chars.len())
                            && current_chars[prefix_len] == stored_chars[prefix_len]
                        {
                            prefix_len += 1;
                        }

                        let mut suffix_len = 0;
                        while suffix_len
                            < (current_chars.len() - prefix_len).min(stored_chars.len() - prefix_len)
                            && current_chars[current_chars.len() - 1 - suffix_len]
                                == stored_chars[stored_chars.len() - 1 - suffix_len]
                        {
                            suffix_len += 1;
                        }

                        let old_mid_start = prefix_len;
                        let old_mid_end = stored_chars.len() - suffix_len;
                        let new_mid_start = prefix_len;
                        let new_mid_end = current_chars.len() - suffix_len;

                        // Eliminar caracteres viejos (de atrás hacia adelante)
                        for i in (old_mid_start..old_mid_end).rev() {
                            if i < text_data.local_data.len() {
                                let delete_op = TextOperation::Delete { position: i };
                                text_data.apply_local_operation(delete_op);
                            }
                        }

                        // Insertar los nuevos caracteres
                        for (j, &ch) in current_chars[new_mid_start..new_mid_end].iter().enumerate() {
                            let pos = old_mid_start + j;
                            let insert_op = TextOperation::Insert {
                                position: pos,
                                character: ch,
                            };
                            text_data.apply_local_operation(insert_op);
                        }
                    }
                }

                if !self.selected_text.is_empty() {
                    // CASO 1: Reemplazar texto seleccionado - USAR OPERACIÓN ATÓMICA
                    let current_content = &text_data.local_data;
                    
                    if let Some(start_pos) = current_content.find(&self.selected_text) {
                        let end_pos = start_pos + self.selected_text.chars().count();
                        
                        // Verificar que el rango es válido
                        if end_pos <= current_content.chars().count() {
                            let found_text: String = current_content.chars().skip(start_pos).take(self.selected_text.chars().count()).collect();
                            
                            // Solo proceder si el texto encontrado coincide exactamente
                            if found_text == self.selected_text {
                                // USAR OPERACIÓN ATÓMICA: ReplaceRange
                                let delete = TextOperation::DeleteRange {
                                    start: start_pos,
                                    end: end_pos,
                                };
                                text_data.apply_local_operation(delete);
                                text_data.apply_local_operation(TextOperation::InsertText {
                                    position: start_pos,
                                    text: response.clone(),
                                });
                                
                                self.file_notifications.lock().unwrap().push(format!(
                                    "🤖 AI: Reemplazado texto seleccionado '{}' con '{}'",
                                    self.selected_text,
                                    response
                                ));
                            } else {
                                self.ai_error_message = "Error: El texto seleccionado no se encontró en la posición esperada".to_string();
                            }
                        } else {
                            self.ai_error_message = "Error: El texto seleccionado excede el tamaño del documento".to_string();
                        }
                    } else {
                        self.ai_error_message = "Error: No se pudo encontrar el texto seleccionado en el documento".to_string();
                    }
                    self.text_editor_content = text_data.local_data.clone();
                } else if self.ai_position == 0 {
                    text_data.apply_local_operation(TextOperation::DeleteAll);
                    text_data.apply_local_operation(TextOperation::InsertText { position: 0, text: response.clone() });
                    
                    self.file_notifications.lock().unwrap().push(format!(
                        "🤖 AI: Reemplazado todo el documento con {} caracteres",
                        response.chars().count()
                    ));
                    
                    self.text_editor_content = text_data.local_data.clone();
                } else {
                    // CASO 3: Insertar en posición específica - USAR OPERACIÓN ATÓMICA
                    let insert_text_op = TextOperation::InsertText {
                        position: self.ai_position,
                        text: response.clone(),
                    };
                    text_data.apply_local_operation(insert_text_op);
                    
                    self.file_notifications.lock().unwrap().push(format!(
                        "🤖 AI: Insertado texto en posición {} ({} caracteres)",
                        self.ai_position,
                        response.chars().count()
                    ));
                    
                    self.text_editor_content = text_data.local_data.clone();
                }

                // Limpiar campos solo si no hubo errores
                if self.ai_error_message.is_empty() {
                    self.ai_prompt.clear();
                    self.selected_text.clear();
                    self.ai_response = None;
                    self.show_ai_response_dialog = false;
                }
            }
        }
    }
    fn update_spreadsheet_from_string(&mut self, content: &str) {
        let rows: Vec<Vec<String>> = content
            .lines()
            .map(|line| {
                line.split(';')
                    .map(|s| s.trim().to_string())
                    .collect::<Vec<String>>()
            })
            .collect();

        let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);

        self.spreadsheet_data = SpreadSheet {
            data: rows
                .iter()
                .map(|row| {
                    let mut new_row = row.clone();
                    new_row.resize(max_cols, String::new());
                    new_row
                })
                .collect(),
        };

        self.previous_spreadsheet_data = self.spreadsheet_data.clone();
    }

    fn connect_to_redis(&mut self) -> Result<(), Error> {
        match &self.redis_stream {
            None => {
                let address = format!("{}:{}", self.remote_ip, self.remote_port);
                match connect_to_cluster(address, self.username.clone(), self.password.clone()) {
                    Ok((stream, mode)) => {
                        // TODO: Queda ver cuando llega acá!!!!!!
                        self.redis_stream = Some(stream);
                        self.modo_lectura = !mode;
                        Ok(())
                    }
                    Err(_) => Err(Error::new(ErrorKind::Other, "Error al conectar")),
                }
            }
            Some(_) => {
                println!("Ya tenia conexión");
                Ok(())
            }
        }
    }
    fn handle_login(&mut self) {
        match self.redis_stream {
            None => match self.connect_to_redis() {
                Ok(()) => {
                    self.current_view = CurrentView::MainApp;
                    self.login_error_message.clear();
                }
                Err(_) => {
                    self.login_error_message = "Usuario o contraseña incorrectos.".to_string();
                }
            },
            Some(_) => {}
        }
    }

    fn load_spreadsheet_from_csv_dialog(&mut self) {
        if let Some(path) = FileDialog::new().add_filter("CSV", &["csv"]).pick_file() {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    let rows: Vec<Vec<String>> = content
                        .lines()
                        .map(|line| {
                            line.split(';')
                                .map(|s| s.trim().to_string())
                                .collect::<Vec<String>>()
                        })
                        .collect();

                    let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);

                    self.spreadsheet_data = SpreadSheet {
                        data: rows
                            .iter()
                            .map(|row| {
                                let mut new_row = row.clone();
                                new_row.resize(max_cols, String::new());
                                new_row
                            })
                            .collect(),
                    };

                    self.previous_spreadsheet_data = self.spreadsheet_data.clone();
                    *self.watched_file_path.lock().unwrap() = Some(path);
                }
                Err(e) => {
                    eprintln!("Error al abrir CSV: {}", e);
                    self.file_notifications
                        .lock()
                        .unwrap()
                        .push("❌ Error al abrir archivo CSV.".to_string());
                }
            }
        }
    }

    fn render_login_screen(&mut self, ctx: &egui::Context) {
        let screen_rect = ctx.screen_rect();
        let painter = ctx.layer_painter(egui::LayerId::background());
        painter.rect_filled(screen_rect, 0.0, egui::Color32::from_rgb(30, 30, 30));

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(
                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                |ui| {
                    ui.group(|ui| {
                        ui.set_min_width(350.0);
                        ui.vertical_centered(|ui| {
                            ui.heading(
                                egui::RichText::new("🔐 Redis Login")
                                    .size(28.0)
                                    .strong()
                                    .color(egui::Color32::from_rgb(220, 220, 220)),
                            );
                            ui.add_space(25.0);

                            ui.label(egui::RichText::new("Usuario:").size(18.0));
                            ui.add(
                                egui::TextEdit::singleline(&mut self.username)
                                    .hint_text("Ingrese usuario")
                                    .desired_width(f32::INFINITY)
                                    .font(egui::TextStyle::Heading),
                            );
                            ui.add_space(15.0);

                            ui.label(egui::RichText::new("Contraseña:").size(18.0));
                            ui.add(
                                egui::TextEdit::singleline(&mut self.password)
                                    .password(true)
                                    .hint_text("Ingrese contraseña")
                                    .desired_width(f32::INFINITY)
                                    .font(egui::TextStyle::Heading),
                            );
                            ui.add_space(15.0);

                            // Nuevo campo para la dirección IP
                            ui.label(egui::RichText::new("Dirección IP:").size(18.0));
                            ui.add(
                                egui::TextEdit::singleline(&mut self.remote_ip)
                                    .hint_text("0.0.0.0")
                                    .desired_width(f32::INFINITY)
                                    .font(egui::TextStyle::Heading),
                            );
                            ui.add_space(15.0);

                            // Nuevo campo para el puerto
                            ui.label(egui::RichText::new("Puerto:").size(18.0));
                            ui.add(
                                egui::TextEdit::singleline(&mut self.remote_port)
                                    .hint_text("7001")
                                    .desired_width(f32::INFINITY)
                                    .font(egui::TextStyle::Heading),
                            );
                            ui.add_space(25.0);

                            // Actualizar la dirección remota cuando cambien IP o puerto
                            self.remote_address =
                                format!("{}:{}", self.remote_ip, self.remote_port);

                            if ui
                                .add_sized(
                                    [ui.available_width(), 40.0],
                                    egui::Button::new(
                                        egui::RichText::new("Iniciar sesión")
                                            .size(20.0)
                                            .color(egui::Color32::WHITE),
                                    ),
                                )
                                .clicked()
                            {
                                self.handle_login();
                            }

                            ui.add_space(10.0);

                            if !self.login_error_message.is_empty() {
                                ui.label(
                                    egui::RichText::new(&self.login_error_message)
                                        .color(egui::Color32::RED)
                                        .strong(),
                                );
                            }
                        });
                    });
                },
            );
        });
    }

    fn apply_new_changes_on_csv(&mut self, ctx: &egui::Context) {
        // Variable para detectar si necesitamos actualizar la UI
        let mut ui_needs_update = false;
        let mut canal_cerrado = false;

        // Procesar cambios remotos
        if let Some(csv_data) = &mut self.csv_data {
            // Procesar operaciones remotas primero
            if let Some(remote) = &self.csv_remote {
                for instruction in remote.try_iter() {
                    // Aplicar la operación remota
                    csv_data.receive_remote_instruction(instruction.clone());
                    ui_needs_update = true;

                    // Registrar la operación para depuración
                    println!("CSV: Recibida operación remota: {:?}", instruction);

                    self.file_notifications.lock().unwrap().push(format!(
                        "CSV: Operación remota del cliente {} en celda [{},{}]",
                        instruction.operation_id.client_id,
                        instruction.operation.row + 1,
                        instruction.operation.column + 1
                    ));
                }
                // Verificar si el canal está cerrado
                if let Err(std::sync::mpsc::TryRecvError::Disconnected) = remote.try_recv() {
                    canal_cerrado = true;
                }

                // Si hubo cambios remotos, actualizar la UI inmediatamente
                if ui_needs_update {
                    println!("CSV: Actualizando UI con datos del SpreadSheet:");
                    for (i, row) in csv_data.local_data.data.iter().enumerate() {
                        println!("  Fila {}: {:?}", i, row);
                    }

                    // Actualizar la UI con los datos del SpreadSheet
                    self.spreadsheet_data = rustidocs::app::operation::csv::SpreadSheet {
                        data: csv_data.local_data.data.clone(),
                    };
                    self.previous_spreadsheet_data = self.spreadsheet_data.clone();

                    // CRÍTICO: Forzar repintado para que los cambios sean visibles de inmediato
                    ctx.request_repaint();
                }
            }
        }
        if canal_cerrado {
            self.file_notifications.lock().unwrap().push("Error: El canal de comunicación con el servidor se cerró. Puede que el backend haya fallado. Intente recargar la planilla o reiniciar la conexión.".to_string());
        }
    }

    // Cambiar esta función de mét-odo a función independiente
    fn apply_cell_change(
        row: usize,
        col: usize,
        old_value: &str,
        new_value: &str,
        csv_data: &mut Client<SpreadSheet, SpreadOperation>,
        file_notifications: &Arc<Mutex<Vec<String>>>,
    ) {
        if old_value == new_value {
            return;
        }

        // Determinar cambios a nivel de caracteres
        let old_chars: Vec<char> = old_value.chars().collect();
        let new_chars: Vec<char> = new_value.chars().collect();

        // 1. Encontrar el prefijo común
        let mut prefix_len = 0;
        while prefix_len < old_chars.len().min(new_chars.len())
            && old_chars[prefix_len] == new_chars[prefix_len]
        {
            prefix_len += 1;
        }

        // 2. Encontrar el sufijo común
        let mut suffix_len = 0;
        while suffix_len < (old_chars.len() - prefix_len).min(new_chars.len() - prefix_len)
            && old_chars[old_chars.len() - 1 - suffix_len]
                == new_chars[new_chars.len() - 1 - suffix_len]
        {
            suffix_len += 1;
        }

        // 3. Determinar qué ha cambiado
        let old_mid_start = prefix_len;
        let old_mid_end = old_chars.len() - suffix_len;
        let new_mid_start = prefix_len;
        let new_mid_end = new_chars.len() - suffix_len;

        // 4. Primero eliminar caracteres viejos (de atrás hacia adelante)
        for i in (old_mid_start..old_mid_end).rev() {
            // Crear TextOperation para eliminar caracteres
            let text_op = TextOperation::Delete { position: i };

            // Envolver en SpreadOperation con la información de la celda
            let spread_op = SpreadOperation {
                row,
                column: col,
                operation: text_op,
            };

            // Aplicar la operación localmente
            csv_data.apply_local_operation(spread_op);

            file_notifications.lock().unwrap().push(format!(
                "CSV: Eliminación en celda [{},{}] posición {}",
                row + 1,
                col + 1,
                i
            ));
        }

        // 5. Luego insertar los nuevos caracteres
        for (j, &ch) in new_chars[new_mid_start..new_mid_end].iter().enumerate() {
            let pos = old_mid_start + j;

            // Crear TextOperation para insertar caracteres
            let text_op = TextOperation::Insert {
                position: pos,
                character: ch,
            };

            // Envolver en SpreadOperation con la información de la celda
            let spread_op = SpreadOperation {
                row,
                column: col,
                operation: text_op,
            };

            // Aplicar la operación localmente
            csv_data.apply_local_operation(spread_op);

            file_notifications.lock().unwrap().push(format!(
                "CSV: Inserción de '{}' en celda [{},{}] posición {}",
                ch,
                row + 1,
                col + 1,
                pos
            ));
        }
    }

    fn render_main_app(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                let title_text = if self.modo_lectura {
                    "Redis Cluster Client (Modo Solo Lectura)"
                } else {
                    "Redis Cluster Client"
                };
                ui.heading(title_text);
            });

            ui.add_space(10.0);
            ui.heading("📚 Documentos");

            // Área scrollable para mostrar los documentos
            ui.group(|ui| {
                ui.set_height(150.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if let Some(documents) = &self.available_documents {
                        if documents.is_empty() {
                            ui.label("No hay documentos disponibles.");
                        } else {
                            // Collect document info to avoid borrowing self mutably and immutably
                            let docs_info: Vec<(String, DocType)> = documents
                                .iter()
                                .map(|doc| (doc.get_name(), doc.get_type()))
                                .collect();
                            for (doc_name, doc_type) in docs_info {
                                let doc_type_icon = match doc_type {
                                    DocType::Text => "📝",
                                    DocType::SpreadSheet => "📊",
                                };

                                ui.horizontal(|ui| {
                                    ui.label(format!("{} {}", doc_type_icon, doc_name));

                                    // CAMBIO AQUÍ: Permitir que usuarios en modo lectura se unan a documentos
                                    // Eliminamos el add_enabled para que el botón siempre esté activo
                                    if ui.button("Unirse").clicked() {
                                        self.remote_filename = doc_name.clone();

                                        // Intentar conectar directamente sin mostrar diálogos adicionales
                                        if let Ok((stream, _)) = connect_to_cluster(
                                            self.remote_address.clone(),
                                            self.username.clone(),
                                            self.password.clone(),
                                        ) {
                                            match doc_type {
                                                DocType::Text => {
                                                    println!(
                                                        "Uniendo a texto: {}",
                                                        self.remote_filename
                                                    );
                                                    self.create_text_client_data(stream);
                                                    self.current_view = CurrentView::TextEditor;
                                                }
                                                DocType::SpreadSheet => {
                                                    println!(
                                                        "Uniendo a CSV: {}",
                                                        self.remote_filename
                                                    );
                                                    self.create_csv_client_data(stream);
                                                    self.current_view =
                                                        CurrentView::SpreadsheetEditor;
                                                }
                                            }
                                        } else {
                                            eprintln!("Error al conectar a Redis");
                                            self.file_notifications.lock().unwrap().push(
                                                "❌ Error al conectarse al servidor Redis"
                                                    .to_string(),
                                            );
                                        }
                                    }

                                    // Botón para borrar el documento - sigue deshabilitado en modo solo lectura
                                    if ui
                                        .add_enabled(
                                            !self.modo_lectura,
                                            egui::Button::new("🗑️ Borrar"),
                                        )
                                        .clicked()
                                    {
                                        if let Some(client_index) = &mut self.client_index {
                                            println!("Eliminando documento: {}", doc_name);
                                            client_index.remove_doc(doc_name.clone());
                                            self.file_notifications.lock().unwrap().push(format!(
                                                "🗑️ Documento '{}' eliminado",
                                                doc_name
                                            ));
                                        }
                                    }
                                });
                            }
                        }
                    } else {
                        ui.label("Cargando documentos...");
                    }
                });
            });

            ui.add_space(20.0);

            ui.horizontal(|ui| {
                if ui.button("🔄 Actualizar").clicked() {
                    // Llamar al método refresh del client_index
                    if let Some(client_index) = &mut self.client_index {
                        client_index.refresh();
                    }
                }

                // Botón para crear documento - deshabilitado en modo solo lectura
                if ui
                    .add_enabled(!self.modo_lectura, egui::Button::new("📝 Crear Documento"))
                    .clicked()
                {
                    self.show_document_creation_dialog = true;
                }
            });

            // Mostrar indicador de modo solo lectura
            if self.modo_lectura {
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(5.0);
                ui.colored_label(
                    egui::Color32::from_rgb(255, 200, 0),
                    "⚠️ MODO SOLO LECTURA: No puedes modificar ni crear documentos",
                );
            }
        });

        // Diálogo para crear un nuevo documento - no debería mostrarse en modo solo lectura
        if self.show_document_creation_dialog && !self.modo_lectura {
            egui::Window::new("Crear Nuevo Documento")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    // El código del diálogo se mantiene igual
                    ui.horizontal(|ui| {
                        ui.label("Nombre del documento:");
                        ui.text_edit_singleline(&mut self.new_document_name);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Tipo:");
                        ui.radio_value(&mut self.new_document_type, DocType::Text, "Texto");
                        ui.radio_value(
                            &mut self.new_document_type,
                            DocType::SpreadSheet,
                            "Planilla",
                        );
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Cancelar").clicked() {
                            self.show_document_creation_dialog = false;
                        }

                        if ui.button("Crear").clicked() {
                            if !self.new_document_name.is_empty() {
                                // Primero registrar el documento en el índice
                                if let Some(client_index) = &mut self.client_index {
                                    client_index.add_doc(
                                        self.new_document_name.clone(),
                                        self.new_document_type.clone(),
                                    );

                                    self.new_document_name.clear();
                                    self.show_document_creation_dialog = false;
                                }
                            }
                        }
                    });
                });
        }
    }

    fn render_text_editor(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let title = if self.modo_lectura {
                "📝 Editor de Texto (Solo Lectura)"
            } else {
                "📝 Editor de Texto"
            };
            ui.heading(title);

            ui.horizontal(|ui| {
                if ui.button("⬅️ Volver").clicked() {
                    self.current_view = CurrentView::MainApp;
                }

                // Botones de AI - solo mostrar si no está en modo lectura
                if !self.modo_lectura {
                    ui.separator();
                    ui.label("🤖 AI:");

                    if ui.button("🤖 Insertar Texto").clicked() {
                        self.ai_position = self.text_editor_content.len(); // Posición al final
                        self.show_ai_dialog = true;
                    }

                    if ui.button("🤖 Reemplazar Todo").clicked() {
                        self.ai_position = 0; // Posición 0 para reemplazar todo
                        self.show_ai_dialog = true;
                    }

                    if ui.button("🤖 Mejorar Selección").clicked() {
                        self.ai_position = usize::MAX; // Marca especial para texto seleccionado
                        self.show_text_selection = true;
                    }
                }
            });

            let filename_display = &self.remote_filename;
            ui.label(filename_display);
            ui.add_space(10.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                // Campo de texto deshabilitado en modo solo lectura
                let editor = egui::TextEdit::multiline(&mut self.text_editor_content)
                    .desired_width(f32::INFINITY)
                    .desired_rows(20)
                    .interactive(!self.modo_lectura);
                ui.add(editor);
            });

            if self.modo_lectura {
                ui.add_space(5.0);
                ui.colored_label(
                    egui::Color32::from_rgb(255, 200, 0),
                    "⚠️ MODO SOLO LECTURA: No puedes editar este documento",
                );
            }

            // Mostrar errores de AI si los hay
            if !self.ai_error_message.is_empty() {
                ui.add_space(5.0);
                ui.colored_label(
                    egui::Color32::from_rgb(255, 100, 100),
                    format!("❌ Error de AI: {}", self.ai_error_message),
                );
            }

            ui.separator();
            ui.collapsing("🔔 Notificaciones de Archivo", |ui| {
                egui::ScrollArea::vertical()
                    .max_height(100.0)
                    .show(ui, |ui| {
                        ui.vertical_centered_justified(|ui| {
                            let notifications = self.file_notifications.lock().unwrap();
                            if notifications.is_empty() {
                                ui.label("No hay notificaciones.");
                            } else {
                                for notif in notifications.iter().rev() {
                                    ui.label(notif);
                                }
                            }
                        });
                    });
            });
        });

        // Diálogo de selección de texto para AI
        if self.show_text_selection {
            let mut selected_text = self.selected_text.clone();
            let mut ai_prompt = self.ai_prompt.clone();
            let mut should_send_request = false;
            let mut should_cancel = false;

            egui::Window::new("🤖 Seleccionar Texto para Mejorar")
                .open(&mut self.show_text_selection)
                .show(ctx, |ui| {
                    ui.label("Ingresa el texto que quieres mejorar:");
                    ui.text_edit_multiline(&mut selected_text);
                    ui.add_space(10.0);

                    ui.label("Prompt para la AI:");
                    ui.text_edit_singleline(&mut ai_prompt);
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("🚀 Enviar a AI").clicked() {
                            should_send_request = true;
                        }
                        if ui.button("❌ Cancelar").clicked() {
                            should_cancel = true;
                        }
                    });
                });

            self.selected_text = selected_text;
            self.ai_prompt = ai_prompt;

            if should_send_request {
                self.show_text_selection = false;
                self.send_ai_request_for_selected_text();
            } else if should_cancel {
                self.show_text_selection = false;
            }
        }

        // Diálogo de prompt para AI
        if self.show_ai_dialog {
            let mut ai_prompt = self.ai_prompt.clone();
            let ai_position = self.ai_position;
            let mut should_send_request = false;
            let mut should_cancel = false;

            egui::Window::new("🤖 Solicitud de AI")
                .open(&mut self.show_ai_dialog)
                .show(ctx, |ui| {
                    if ai_position == 0 {
                        ui.label("Ingresa el prompt para reemplazar todo el documento:");
                    } else {
                        ui.label("Ingresa el prompt para insertar texto:");
                    }
                    ui.text_edit_multiline(&mut ai_prompt);
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("🚀 Enviar").clicked() {
                            should_send_request = true;
                        }
                        if ui.button("❌ Cancelar").clicked() {
                            should_cancel = true;
                        }
                    });
                });

            self.ai_prompt = ai_prompt;

            if should_send_request {
                self.show_ai_dialog = false;
                self.send_ai_request();
            } else if should_cancel {
                self.show_ai_dialog = false;
            }
        }

        // Diálogo de respuesta de AI
        if self.show_ai_response_dialog {
            let ai_response = self.ai_response.clone();
            let mut should_accept = false;
            let mut should_reject = false;

            egui::Window::new("🤖 Respuesta de AI")
                .open(&mut self.show_ai_response_dialog)
                .show(ctx, |ui| {
                    ui.label("Texto generado por la AI:");
                    if let Some(response) = &ai_response {
                        let mut response_text = response.clone();
                        ui.text_edit_multiline(&mut response_text);
                    }
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("✅ Aceptar").clicked() {
                            should_accept = true;
                        }
                        if ui.button("❌ Rechazar").clicked() {
                            should_reject = true;
                        }
                    });
                });

            if should_accept {
                self.show_ai_response_dialog = false;
                self.apply_ai_response();
            } else if should_reject {
                self.show_ai_response_dialog = false;
            }
        }
    }

    fn render_spreadsheet_editor(&mut self, ctx: &egui::Context) {
        // Declarar changed_cells fuera del bloque UI para que sea visible más adelante
        let mut changed_cells: Vec<(usize, usize, String, String)> = Vec::new();

        egui::CentralPanel::default().show(ctx, |ui| {
            let title = if self.modo_lectura {
                "📊 Editor de Planilla (Solo Lectura)"
            } else {
                "📊 Editor de Planilla"
            };
            ui.heading(title);

            ui.horizontal(|ui| {
                if ui.button("⬅️ Volver").clicked() {
                    self.current_view = CurrentView::MainApp;
                }
            });

            if self.modo_lectura {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 200, 0),
                    "⚠️ MODO SOLO LECTURA: No puedes editar esta planilla",
                );
            }

            let mut cell_changed = false;

            egui::ScrollArea::both().show(ui, |ui| {
                egui::Grid::new("spreadsheet_grid")
                    .spacing([4.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        // Código existente para mostrar la tabla
                        let max_rows = self
                            .spreadsheet_data
                            .data
                            .len()
                            .max(self.previous_spreadsheet_data.data.len());

                        // Expandir las grillas si es necesario
                        while self.spreadsheet_data.data.len() < max_rows {
                            self.spreadsheet_data.data.push(Vec::new());
                        }
                        while self.previous_spreadsheet_data.data.len() < max_rows {
                            self.previous_spreadsheet_data.data.push(Vec::new());
                        }

                        for row_idx in 0..max_rows {
                            let max_cols = if row_idx < self.spreadsheet_data.data.len()
                                && row_idx < self.previous_spreadsheet_data.data.len()
                            {
                                self.spreadsheet_data.data[row_idx]
                                    .len()
                                    .max(self.previous_spreadsheet_data.data[row_idx].len())
                            } else {
                                5 // Número mínimo de columnas
                            };

                            // Expandir las filas si es necesario
                            if row_idx < self.spreadsheet_data.data.len() {
                                while self.spreadsheet_data.data[row_idx].len() < max_cols {
                                    self.spreadsheet_data.data[row_idx].push(String::new());
                                }
                            }
                            if row_idx < self.previous_spreadsheet_data.data.len() {
                                while self.previous_spreadsheet_data.data[row_idx].len() < max_cols
                                {
                                    self.previous_spreadsheet_data.data[row_idx]
                                        .push(String::new());
                                }
                            }

                            for col_idx in 0..max_cols {
                                let mut cell_value = if row_idx < self.spreadsheet_data.data.len()
                                    && col_idx < self.spreadsheet_data.data[row_idx].len()
                                {
                                    self.spreadsheet_data.data[row_idx][col_idx].clone()
                                } else {
                                    String::new()
                                };

                                let prev_cell_value = if row_idx
                                    < self.previous_spreadsheet_data.data.len()
                                    && col_idx < self.previous_spreadsheet_data.data[row_idx].len()
                                {
                                    self.previous_spreadsheet_data.data[row_idx][col_idx].clone()
                                } else {
                                    String::new()
                                };

                                // Hacer el TextEdit deshabilitado en modo solo lectura
                                let response = egui::TextEdit::singleline(&mut cell_value)
                                    .desired_width(80.0)
                                    .interactive(!self.modo_lectura)
                                    .show(ui);

                                if response.response.changed()
                                    && cell_value != prev_cell_value
                                    && !self.modo_lectura
                                {
                                    // Registrar el cambio para procesarlo después
                                    changed_cells.push((
                                        row_idx,
                                        col_idx,
                                        prev_cell_value.clone(),
                                        cell_value.clone(),
                                    ));

                                    // Resto del código para procesar cambios...
                                    let msg = format!(
                                        "📝 Celda modificada en [Fila {}, Columna {}]: '{}' → '{}'",
                                        row_idx + 1,
                                        col_idx + 1,
                                        prev_cell_value,
                                        cell_value
                                    );

                                    // Actualizar el valor en ambas estructuras
                                    if row_idx >= self.spreadsheet_data.data.len() {
                                        self.spreadsheet_data.data.resize(row_idx + 1, Vec::new());
                                    }
                                    if col_idx >= self.spreadsheet_data.data[row_idx].len() {
                                        self.spreadsheet_data.data[row_idx]
                                            .resize(col_idx + 1, String::new());
                                    }
                                    self.spreadsheet_data.data[row_idx][col_idx] =
                                        cell_value.clone();

                                    if row_idx >= self.previous_spreadsheet_data.data.len() {
                                        self.previous_spreadsheet_data
                                            .data
                                            .resize(row_idx + 1, Vec::new());
                                    }
                                    if col_idx >= self.previous_spreadsheet_data.data[row_idx].len()
                                    {
                                        self.previous_spreadsheet_data.data[row_idx]
                                            .resize(col_idx + 1, String::new());
                                    }
                                    self.previous_spreadsheet_data.data[row_idx][col_idx] =
                                        cell_value;

                                    self.file_notifications.lock().unwrap().push(msg);
                                    cell_changed = true;
                                }
                            }
                            ui.end_row();
                        }
                    });
            });

            ui.separator();
            ui.heading("🔔 Cambios recientes:");
            for msg in self.file_notifications.lock().unwrap().iter().rev().take(5) {
                ui.label(msg);
            }
        });

        // Procesar todos los cambios de celdas detectados - no hacer en modo solo lectura
        if let Some(csv_data) = &mut self.csv_data {
            // Si hay cambios locales para procesar y NO estamos en modo solo lectura
            if !changed_cells.is_empty() && !self.modo_lectura {
                println!(
                    "CSV: Procesando {} cambios locales en celdas",
                    changed_cells.len()
                );

                // Procesar los cambios celda por celda
                let notifications = self.file_notifications.clone();

                for (row_idx, col_idx, old_value, new_value) in &changed_cells {
                    println!(
                        "CSV: Cambio local - celda [{},{}]: '{}' → '{}'",
                        row_idx + 1,
                        col_idx + 1,
                        old_value,
                        new_value
                    );

                    // Aplicar operaciones CRDT celda por celda
                    Self::apply_cell_change(
                        *row_idx,
                        *col_idx,
                        old_value,
                        new_value,
                        csv_data,
                        &notifications,
                    );
                }

                // Sincronizar los datos locales actualizados
                csv_data.local_data.data = self.spreadsheet_data.data.clone();

                // Forzar repintado para reflejar los cambios locales
                ctx.request_repaint();
            }
        }
    }
}

impl eframe::App for RedisApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Inicializar client_index cuando el usuario inicia sesión
        if self.current_view == CurrentView::MainApp && self.client_index.is_none() {
            println!("Inicializando client_index para obtener documentos disponibles");
            let (index, receiver) = ClientIndex::new(
                &format!("{}:{}", self.remote_ip, self.remote_port),
                &self.username,
                &self.password,
            );

            self.client_index = Some(index);
            self.document_receiver = Some(receiver);

            // Solicitar lista inicial de documentos
            if let Some(client_index) = &mut self.client_index {
                client_index.refresh();
            }
        }

        // Procesar documentos recibidos del servidor
        if let Some(receiver) = &self.document_receiver {
            match receiver.try_recv() {
                Ok(documents) => {
                    println!("Recibidos {} documentos del servidor", documents.len());
                    self.available_documents = Some(documents);
                    // Forzar actualización de la interfaz
                    ctx.request_repaint();
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // No hay documentos disponibles, esto es normal
                }
                Err(e) => {
                    eprintln!("Error al recibir documentos: {:?}", e);
                }
            }
        }

        // Resto del código existente
        if self.open_text_file_requestd {
            self.open_text_file_requestd = false;
        }

        if self.open_csv_file_requested {
            self.open_csv_file_requested = false;
            self.load_spreadsheet_from_csv_dialog();
        }

        // Recolectar todas las notificaciones y contenido del archivo primero
        let mut notifications_to_add = Vec::new();
        let mut content_to_update = None;
        let mut is_text_editor = false;

        {
            if let Ok(rx) = self.file_events_rx.lock() {
                for notification in rx.try_iter() {
                    if notification.starts_with("MODIFICADO") {
                        if let Some(path) = self.watched_file_path.lock().unwrap().clone() {
                            if let Ok(content) = fs::read_to_string(&path) {
                                is_text_editor = self.current_view == CurrentView::TextEditor;
                                content_to_update = Some(content);
                                notifications_to_add
                                    .push("¡Archivo recargado desde el disco!".to_string());
                            }
                        }
                    }
                    notifications_to_add.push(notification);
                }
            }
        }

        // Ahora aplicamos los cambios recolectados
        if let Some(content) = content_to_update {
            if is_text_editor {
                self.text_editor_content = content;
            } else {
                self.update_spreadsheet_from_string(&content);
            }
        }

        // Agregamos las notificaciones recolectadas
        if !notifications_to_add.is_empty() {
            let mut notifications = self.file_notifications.lock().unwrap();
            notifications.extend(notifications_to_add);
        }

        // Procesar cambios según el tipo de editor activo
        match self.current_view {
            CurrentView::TextEditor => self.apply_new_changes_on_file(ctx),
            CurrentView::SpreadsheetEditor => self.apply_new_changes_on_csv(ctx),
            _ => {}
        }

        let mut style = (*ctx.style()).clone();
        style.visuals = Visuals::dark();
        ctx.set_style(style);

        match self.current_view {
            CurrentView::Login => self.render_login_screen(ctx),
            CurrentView::MainApp => self.render_main_app(ctx),
            CurrentView::TextEditor => self.render_text_editor(ctx),
            CurrentView::SpreadsheetEditor => self.render_spreadsheet_editor(ctx),
        }

        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
