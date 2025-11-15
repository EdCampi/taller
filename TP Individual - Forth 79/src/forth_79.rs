use crate::operation::Operation;
pub use crate::output_error::Error;
use crate::stack::Stack;

use std::collections::HashMap;
use std::io::Write;

/// Estructura que representa el interpretador de código Forth-79.
/// # Atributos
/// `stack: Stack` - Stack asociado a la instancia.
/// `stack_size: usize` - Tamaño en bits de la "memoria" máxima del stack.
/// `words: HashMap<String, Vec<String>>` - Diccionario interno para la implentaciópn de las
/// palabras predefinidas.
/// `buffer_aux: Vec<String>` - Buffer intermedio que guarda los outputs antes de la salida.
/// `if_buffer: String` - Buffer que permite el uso de re/definiciones multilínea de words.
pub struct Forth79 {
    stack: Stack, // stack.rs Stack
    stack_size: usize,
    words: HashMap<String, Vec<String>>, // Dictionario para guardar las palabras mapeadas.
    buffer_aux: Vec<String>,
    if_buffer: String,
}

impl Forth79 {
    pub fn new() -> Forth79 {
        Forth79 {
            stack: Stack::new(),
            words: HashMap::new(),    // Tengo las definiciones de palabras.
            stack_size: usize::MAX,   // Valor default
            buffer_aux: Vec::new(),   // Tengo todo lo que voy a imprimir
            if_buffer: String::new(), // Tengo las definiciones multilínea
        }
    }

    /// Setter del tamaño de la memoria del stack.
    /// `size: usize` - Tamaño a utilizar.
    pub fn set_stack_size(&mut self, size: usize) {
        self.stack_size = size / 2;
    }

    /// Función wrapper para la itnerpretación de la línea.
    /// # Parámetros
    /// `line: String` - Línea a interpretar.
    /// `buffer: &mut W` - Buffer a usar para el output.
    /// # Retorna
    /// `true` - Si se completo con éxito la operación.
    pub fn interpret_line<W: Write>(&mut self, line: String, buffer: &mut W) -> bool {
        if self.update_buffer(&line) {
            if self.if_buffer.ends_with(";") {
                return self.tokenize_and_print(&line, true, buffer);
            }
            return true;
        }
        self.tokenize_and_print(&line, false, buffer)
    }

    /// Se encarga de tokenizar la línea pasada y de imprimir las salidas,
    /// así como de limpiar el buffer en cada salida.
    /// # Paramétros
    /// `line: &String` - Línea a analizar.
    /// `flush: bool` - If true -> Se hace flush del buffer de salida.
    /// `buffer: &mut W` - Buffer de salida
    /// # Retorna
    /// `true` - Si se completo con éxito la operación.
    fn tokenize_and_print<W: Write>(&mut self, line: &String, flush: bool, buffer: &mut W) -> bool {
        let input: &String = if flush { &self.if_buffer } else { line };
        let mut tokens: Vec<String> = tokenize(input);
        if flush {
            self.if_buffer.clear();
        }
        let ins_state: bool = self.run_instructions(&mut tokens);
        print_buffer(buffer, &mut self.buffer_aux);
        ins_state
    }

    /// Hecho para el manejo de definiciones multilínea
    /// Pushea al buffer interno y notifica la funcion superior para que no siga con el
    /// análisis de la línea en sí.
    /// # Retorna
    /// `true` - Si se agrego contenido al buffer.
    fn update_buffer(&mut self, line: &String) -> bool {
        if line.starts_with(":") {
            self.if_buffer.push_str(&line);
            return true;
        }
        if self.if_buffer.len() > 0 {
            self.if_buffer.push_str(" ");
            self.if_buffer.push_str(&line);
            return true;
        }
        false
    }

    /// Se encarga de correr las operaciones, frenar en:
    /// 1. La línea era una defnición.
    /// 2. La línea intentó ser de definición, pero estaba mal.
    /// 3. Alguna operación fracasó.
    /// Si todo sale bien, retorna true.
    fn run_instructions(&mut self, line: &mut Vec<String>) -> bool {
        let updated_word_code: i16 = self.update_word(line);
        if updated_word_code == 0 {
            return Error::InvalidWord.throw_error(&mut self.buffer_aux);
        }
        if updated_word_code == -1 {
            return true;
        }
        let tokens = self.parse_line(line);
        for token in tokens.iter() {
            if !token.apply(&mut self.stack, self.stack_size, &mut self.buffer_aux) {
                return false;
            }
        }
        true
    }

    /// Función que permite llevar la cuenta de las definiciones y redefinicions de palabras en el diccionario.
    fn update_word(&mut self, line: &mut Vec<String>) -> i16 {
        if line[0] != ":" {
            return 1;
        }
        if is_numerical(&line[1]) {
            return 0;
        }
        if self.words.contains_key(&line[1]) {
            // Así no pierdo el valor con el que se definieron originalmente algunas words.
            self.words = replace_occurrences(&self.words);
        }

        let mut aux: Vec<String> = Vec::new();
        for token in &line[2..line.len() - 1] {
            // Reviso las words de la definición.
            if token == &line[1] {
                // Evito bucles infinitos si me redefino a mi mismo conmigo mismo.
                if let Some(v) = self.words.get_mut(token) {
                    for value in v {
                        aux.push(value.to_string());
                    }
                }
                continue;
            }
            aux.push(token.to_string());
        }
        self.words.insert(line[1].to_string(), aux);
        -1
    }

    /// Ddada una línea de tokens, parseo la misma a un vector de operaciones.
    /// # Retorna
    /// `Vec<Operation>`- Vector de operaciones listo para aplicar sobre la pila.
    fn parse_line(&mut self, tokens: &mut Vec<String>) -> Vec<Operation> {
        let mut res: Vec<Operation> = Vec::new();
        let mut i = 0;
        while i < tokens.len() {
            if self.words.contains_key(&tokens[i]) {
                self.expand_token(tokens, &mut i);
                continue;
            }
            if &tokens[i] == "IF" {
                self.push_if_token(tokens, &mut i, &mut res);
            } else {
                res.push(self.token_to_op(&tokens[i]));
                i += 1;
            }
        }
        res
    }

    fn expand_token(&mut self, tokens: &mut Vec<String>, i: &mut usize) {
        let mut tokens_added: usize = 0;
        for token in self.words.get(&tokens[*i].to_string()).unwrap() {
            tokens.insert(*i + tokens_added, token.to_string());
            tokens_added += 1;
        }
        tokens.remove(*i + tokens_added);
    }

    /// Inicia el mapeo del bloque if. delega las branches en la función `push_branch`.
    fn push_if_token(&mut self, tokens: &mut Vec<String>, i: &mut usize, res: &mut Vec<Operation>) {
        let mut if_operator: Operation = self.token_to_op(&tokens[*i]);
        if let Operation::BranchIf(ref mut pos, ref mut neg) = if_operator {
            *i += 1;
            self.push_branch(tokens, i, pos, vec!["THEN", "ELSE"]); // Pusheo la primera rama, IF
            self.push_branch(tokens, i, neg, vec!["THEN"]); // Pusheo la segunda rama, ELSE
        }
        res.push(if_operator);
    }

    /// Analiza un solo lado de la rama if. Util para condicionales anidados.
    fn push_branch(
        &mut self,
        tokens: &mut Vec<String>,
        i: &mut usize,
        operations: &mut Vec<Operation>,
        delimiters: Vec<&str>,
    ) {
        let mut tokens_aux: Vec<String> = Vec::new();
        let mut j = 1;
        while j != 0 && *i < tokens.len() {
            tokens_aux.push(tokens[*i].to_string());
            j += if tokens[*i] == "IF" { 1 } else { 0 };
            j -= if delimiters.contains(&&tokens[*i].as_str()) {
                1
            } else {
                0
            };
            *i += 1;
        }
        let branch_operations = self.parse_line(&mut tokens_aux);
        for operation in branch_operations {
            operations.push(operation);
        }
    }

    /// Mapea cada token (`&String`) a una `Operation`.
    fn token_to_op(&mut self, token: &String) -> Operation {
        match token.as_str() {
            "+" => Operation::Add,
            "-" => Operation::Sub,
            "*" => Operation::Mul,
            "/" => Operation::Div,
            "DUP" => Operation::Dup,
            "DROP" => Operation::Drop,
            "SWAP" => Operation::Swap,
            "OVER" => Operation::Over,
            "ROT" => Operation::Rot,
            "." => Operation::Dot,
            "EMIT" => Operation::Emit,
            "CR" => Operation::Cr,
            "=" => Operation::Eq,
            "<" => Operation::Lt,
            ">" => Operation::Gt,
            "AND" => Operation::And,
            "OR" => Operation::Or,
            "NOT" => Operation::Not,
            "IF" => Operation::BranchIf(Vec::new(), Vec::new()),
            "ELSE" => Operation::BranchElse,
            "THEN" => Operation::BranchEnd,
            _ => {
                if let Ok(n) = token.parse::<i16>() {
                    Operation::N(n)
                } else if token.starts_with(".\"") {
                    Operation::Print(token[3..token.len() - 1].trim().to_string())
                } else {
                    Operation::Unknown
                }
            }
        }
    }

    /// Permite reivsar el estado actual del stack sin modificaciones hechas.
    pub fn get_stack_state(&self) -> Vec<i16> {
        self.stack.get_items()
    }

    /// Devuelve un string con el contenido actual del stack.
    pub fn get_stack_output(&self) -> String {
        let state = self
            .get_stack_state()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        state.join(" ")
    }
}

/// Printea y limpia el buffer utilizado.
/// Util al final de la ejecución de la línea.
fn print_buffer<W: Write>(buffer: &mut W, aux_buffer: &mut Vec<String>) {
    let mut whitespace: bool = false;
    let mut output: String = String::new();
    for str in aux_buffer.iter() {
        if whitespace && str != "\n" {
            output.push_str(" ");
        }
        output.push_str(str);
        match write!(buffer, "{}", output) {
            Ok(_) => {}
            Err(_) => {
                println!("Error while printing");
                return;
            }
        }
        whitespace = str != "\n";
        output.clear();
    }
    if aux_buffer.len() > 0 {
        let newline = aux_buffer[aux_buffer.len() - 1] == "\n";
        aux_buffer.clear();
        if !newline {
            aux_buffer.push("".to_string()); // Pusheo un str vacío así en la próxima corrida, se imprime con el espacio. (una manera de saber que ya se imprimió)
        }
    }
}

fn is_numerical(string: &String) -> bool {
    match string.parse::<i16>() {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// Función que dada una línea de texto, devuelve los tokens de las mismas en función de la sintaxis
/// del lenguaje Forth-79.
fn tokenize(line: &String) -> Vec<String> {
    let pseudo_tokens: Vec<String> = line.split(' ').map(|s| s.to_string()).collect();

    let mut tokens: Vec<String> = Vec::new();
    let mut i = 0;
    while i < pseudo_tokens.len() {
        if pseudo_tokens[i] == ".\"" {
            let (aux, j) = extend_token(&pseudo_tokens, &mut i, "\"");
            i = j;
            tokens.push(aux);
        } else {
            if pseudo_tokens[i] == "" {
                i += 1;
                continue;
            }
            tokens.push(pseudo_tokens[i].to_uppercase());
        }
        i += 1;
    }
    tokens
}

/// Exitendo el token actual hasta encontrar el delimitador final.
/// Util para el caso `." palabra1 palabra2     palabra3"`
/// # Retorna
/// `String` - El nuevo token extendido
/// `usize` - La cantidad de saltos hechos, para quela función superior no pierda el rastro
/// de la modificación hecha.
fn extend_token(tokens: &Vec<String>, i: &mut usize, delimiter: &str) -> (String, usize) {
    let mut aux: String = String::new();
    aux.push_str(&tokens[*i]);
    *i += 1;
    while !tokens[*i].ends_with(delimiter) {
        aux.push_str(" ");
        aux.push_str(&tokens[*i]);
        *i += 1;
    }
    aux.push_str(" ");
    aux.push_str(&tokens[*i]);
    *i += 1;
    (aux, *i)
}

/// Cambia el diccionario por otro con las definiciones expandidas.
fn replace_occurrences(dictionary: &HashMap<String, Vec<String>>) -> HashMap<String, Vec<String>> {
    let mut res = HashMap::new();

    for (key, values) in dictionary {
        let mut replaced = Vec::new();
        for item in values {
            if let Some(values) = dictionary.get(item) {
                replaced.extend(values.clone());
            } else {
                replaced.push(item.clone());
            }
        }
        res.insert(key.clone(), replaced);
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_forth_79_creation() {
        let forth = Forth79::new();
        assert_eq!(forth.get_stack_state(), vec![]);
        assert_eq!(forth.get_stack_output(), "");
        assert_eq!(forth.words.is_empty(), true);
        assert_eq!(forth.buffer_aux.is_empty(), true);
        assert_eq!(forth.if_buffer.is_empty(), true);
    }

    #[test]
    fn test_definition_add_word() {
        let mut forth = Forth79::new();
        forth.interpret_line(": A 1 ;".to_string(), &mut io::stdout());
        assert_eq!(forth.get_stack_state(), vec![]);
        assert_eq!(forth.words.is_empty(), false);
        assert_eq!(forth.words.get("A"), Some(&vec!["1".to_string()]));
        assert_eq!(forth.buffer_aux.is_empty(), true);
    }

    #[test]
    fn test_redefine_word() {
        let mut forth = Forth79::new();
        forth.interpret_line(": A 1 ;".to_string(), &mut io::stdout());
        forth.interpret_line(": A 2 ;".to_string(), &mut io::stdout());
        assert_eq!(forth.words.get("A"), Some(&vec!["2".to_string()]));
    }

    #[test]
    fn test_setting_stack_size() {
        let mut forth = Forth79::new();
        forth.set_stack_size(1024); // Bytes -> i16 -> Se divide en 2.
        assert_eq!(forth.stack_size, 1024 / 2);
    }

    #[test]
    fn test_output_buffer_added_correctly() {
        // Error messages are inside integration tests.
        let mut forth = Forth79::new();
        let mut buffer = Vec::new();

        forth.interpret_line("1 2 3 . .".to_string(), &mut buffer);
        assert_eq!(String::from_utf8(buffer).unwrap(), String::from("3 2"));
    }

    #[test]
    fn test_if_buffer_adding_correctly() {
        let mut forth = Forth79::new();
        let mut buffer = Vec::new();

        forth.interpret_line(": a 1 2 3 ".to_string(), &mut buffer);
        assert_eq!(forth.if_buffer, ": a 1 2 3 ".to_string());
    }

    #[test]
    fn test_if_buffer_clearing_correctly_after_end_of_statement() {
        let mut forth = Forth79::new();
        let mut buffer = Vec::new();

        forth.interpret_line(": a 1 ".to_string(), &mut buffer);
        forth.interpret_line(" 2 3 4 ;".to_string(), &mut buffer);
        assert_eq!(forth.if_buffer.is_empty(), true);
        assert_eq!(
            forth.words.get("A"),
            Some(&vec![
                "1".to_string(),
                "2".to_string(),
                "3".to_string(),
                "4".to_string()
            ])
        );
    }

    #[test]
    fn test_tokenize_tokenizes_correctly() {
        let line = String::from(": A 1 2 3 ;");
        let tokens = tokenize(&line);

        assert_eq!(tokens, vec![":", "A", "1", "2", "3", ";"]);
    }

    #[test]
    fn test_tokenize_tokenizes_correctly_single_line() {
        let line = String::from("A");
        let tokens = tokenize(&line);

        assert_eq!(tokens, vec!["A"]);
    }

    #[test]
    fn test_tokenize_tokenizes_correctly_lots_of_whitespaces() {
        let line = String::from(": A   1    2 3    ;                  ");
        let tokens = tokenize(&line);

        assert_eq!(tokens, vec![":", "A", "1", "2", "3", ";"]);
    }
}
