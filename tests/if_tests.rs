use forth::forth_79::Forth79;
use std::io;

#[test]
fn test_if_simple() {
    let mut forth = Forth79::new();
    forth.interpret_line(": f if 2 then ;".to_string(), &mut io::stdout());
    forth.interpret_line("-1 f".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [2]);
}

#[test]
fn test_if_else() {
    let mut forth = Forth79::new();
    forth.interpret_line(": f if 2 else 3 then ;".to_string(), &mut io::stdout());
    forth.interpret_line("-1 f".to_string(), &mut io::stdout());
    forth.interpret_line("0 f".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [2, 3]);
}

#[test]
fn test_nested_if() {
    let mut forth = Forth79::new();
    forth.interpret_line(": f".to_string(), &mut io::stdout());
    forth.interpret_line("if".to_string(), &mut io::stdout());
    forth.interpret_line("if 1 else 2 then".to_string(), &mut io::stdout());
    forth.interpret_line("else".to_string(), &mut io::stdout());
    forth.interpret_line("drop 3".to_string(), &mut io::stdout());
    forth.interpret_line("then ;".to_string(), &mut io::stdout());

    forth.interpret_line("-1 -1 f".to_string(), &mut io::stdout());
    forth.interpret_line("0 -1 f".to_string(), &mut io::stdout());
    forth.interpret_line("0 0 f".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1, 2, 3]);
}

#[test]
fn test_nested_if_else() {
    let mut forth = Forth79::new();
    forth.interpret_line(": f".to_string(), &mut io::stdout());
    forth.interpret_line("dup 0 = if".to_string(), &mut io::stdout());
    forth.interpret_line("drop 2".to_string(), &mut io::stdout());
    forth.interpret_line("else dup 1 = if".to_string(), &mut io::stdout());
    forth.interpret_line("drop 3".to_string(), &mut io::stdout());
    forth.interpret_line("else drop 4 then then ;".to_string(), &mut io::stdout());

    forth.interpret_line("0 f".to_string(), &mut io::stdout());
    forth.interpret_line("1 f".to_string(), &mut io::stdout());
    forth.interpret_line("2 f".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [2, 3, 4]);
}

#[test]
fn test_if_non_canonical() {
    let mut forth = Forth79::new();
    forth.interpret_line(": f if 10 then ;".to_string(), &mut io::stdout());
    forth.interpret_line("5 f".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [10]);
}
