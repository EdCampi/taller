mod forth_79;
mod operation;
mod output_error;
mod stack;

use forth_79::Forth79;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::Path;

/// Escribe los contenidos restantes del stack en stack.fth
/// en la capeta base de forth.
/// # Parámetros
/// `forth` - Instancia utilizada de Forth79 para imprimir.
fn write_stack_at_exit(forth: Forth79) {
    let stack_state = forth.get_stack_output();
    println!("{:?}", &stack_state);

    let _out_file = match File::create("./stack.fth") {
        Ok(ref mut f) => {
            match &f.write_all(stack_state.as_bytes()) {
                Ok(_) => (),
                Err(e) => println!("Error when writing the file, \"stack.fth\" {}", e),
            }
            f
        }
        Err(e) => {
            println!("Error when creating the file, \"stack.fth\" {}", e);
            return;
        }
    };
}

/// Función prinicpal que corre las instucciones en el archivo pasado
/// por parámetro al llamar al programa.
///
/// # Parámetros
/// `args: Vec<String>` - Son los argumentos con los cuales se llamó al programa
/// de la forma [ ruta_del_programa, ruta_del_archivo, capacidad_máx_en_bits_del_stack ].
/// OBS: El archivo debe tener un conjuuntos de instrucciones separadas por lineas (idealmente, ".fth").
fn run_instructions(args: &Vec<String>) {
    let mut forth = Forth79::new();
    let stack_size: usize = if args.len() > 2 {
        let size_value: Vec<&str> = args[2].split("=").collect();
        match size_value[1].parse::<usize>() {
            Ok(size) => size,
            Err(_) => {
                println!("Error when setting the size -> Using default value");
                1024
            }
        }
    } else {
        1024
    };

    forth.set_stack_size(stack_size);
    let file_path = &args[1];
    if let Ok(lines) = read_lines(file_path) {
        for line in lines.map_while(Result::ok) {
            println!("{}", &line);
            if !forth.interpret_line(line, &mut io::stdout()) {
                break;
            }
        }
    }
    print!("\n");
    write_stack_at_exit(forth);
}

/// Función auxiliar para leer línea por línea el archivo.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    run_instructions(&args);
}
