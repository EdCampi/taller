use forth::forth_79::Forth79;

#[test]
fn test_dot_without_leftover() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("1 2".to_string(), &mut buffer);
    forth.interpret_line(". .".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "2 1");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_dot_with_leftover() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("1 2 3 4 5".to_string(), &mut buffer);
    forth.interpret_line(". . .".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "5 4 3");
    assert_eq!(forth.get_stack_state(), [1, 2]);
}

#[test]
fn test_cr_1() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("cr".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "\n");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_cr_2() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("cr cr".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "\n\n");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_dot_and_cr() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("1 .".to_string(), &mut buffer);
    forth.interpret_line("cr cr".to_string(), &mut buffer);
    forth.interpret_line("2 .".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "1\n\n2");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_emit_uppercase() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("65 emit".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "A");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_emit_lowercase() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("97 emit".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "a");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_emit_multiple() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("68 67 66 65".to_string(), &mut buffer);
    forth.interpret_line("emit emit emit emit".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "A B C D");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_dot_quote_hello_world() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line(".\" hello world\"".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "hello world");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_dot_quote_multiple_whitespace() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line(".\" hello      world!\"".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "hello      world!");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_dot_quote_mutiples() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line(".\" hello\"".to_string(), &mut buffer);
    forth.interpret_line(".\" world\"".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "hello world");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_dot_quote_and_cr() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line(".\" hello\"".to_string(), &mut buffer);
    forth.interpret_line("cr".to_string(), &mut buffer);
    forth.interpret_line(".\" world\"".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "hello\nworld");
    assert_eq!(forth.get_stack_state(), []);
}
