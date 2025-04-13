use forth::forth_79::Forth79;
use std::io;

#[test]
fn test_equals_true() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 1 =".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [-1]);
}

#[test]
fn test_equals_false() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 2 =".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [0]);
}

#[test]
fn test_less_true() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 2 <".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [-1]);
}

#[test]
fn test_less_false() {
    let mut forth = Forth79::new();
    forth.interpret_line("2 1 < ".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [0]);
}

#[test]
fn test_less_equals() {
    let mut forth = Forth79::new();
    forth.interpret_line("2 2 <".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [0]);
}

#[test]
fn test_more_true() {
    let mut forth = Forth79::new();
    forth.interpret_line("2 1 >".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [-1]);
}

#[test]
fn test_more_false() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 2 >".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [0]);
}

#[test]
fn test_more_equals() {
    let mut forth = Forth79::new();
    forth.interpret_line("2 2 >".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [0]);
}

#[test]
fn test_and_none() {
    let mut forth = Forth79::new();
    forth.interpret_line("0 0 and".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [0]);
}

#[test]
fn test_and_one() {
    let mut forth = Forth79::new();
    forth.interpret_line("-1 0 and".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [0]);
}

#[test]
fn test_and_both() {
    let mut forth = Forth79::new();
    forth.interpret_line("-1 -1 and".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [-1]);
}

#[test]
fn test_or_none() {
    let mut forth = Forth79::new();
    forth.interpret_line("0 0 or".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [0]);
}

#[test]
fn test_or_one() {
    let mut forth = Forth79::new();
    forth.interpret_line("-1 0 or".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [-1]);
}

#[test]
fn test_or_both() {
    let mut forth = Forth79::new();
    forth.interpret_line("-1 -1 or".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [-1]);
}

#[test]
fn test_not_true() {
    let mut forth = Forth79::new();
    forth.interpret_line("-1 not".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [0]);
}

#[test]
fn test_not_false() {
    let mut forth = Forth79::new();
    forth.interpret_line("0 not".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [-1]);
}

#[test]
fn test_not_not() {
    let mut forth = Forth79::new();
    forth.interpret_line("10 not not".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [-1]);
}
