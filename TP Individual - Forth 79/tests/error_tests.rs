use forth::forth_79::Forth79;

#[test]
fn test_underflow_1() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("+".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "stack-underflow\n");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_underflow_2() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("1 +".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "stack-underflow\n");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_underflow_3() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("-".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "stack-underflow\n");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_underflow_4() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("1 -".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "stack-underflow\n");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_underflow_5() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("*".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "stack-underflow\n");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_underflow_6() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("1 *".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "stack-underflow\n");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_underflow_7() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("/".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "stack-underflow\n");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_underflow_8() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("1 /".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "stack-underflow\n");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_underflow_9() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("dup".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "stack-underflow\n");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_underflow_10() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("drop".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "stack-underflow\n");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_underflow_11() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("swap".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "stack-underflow\n");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_underflow_12() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("1 swap".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "stack-underflow\n");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_underflow_13() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("over".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "stack-underflow\n");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_underflow_14() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("1 over".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "stack-underflow\n");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_division_by_zero() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("4 0 /".to_string(), &mut buffer);
    assert_eq!(String::from_utf8(buffer).unwrap(), "division-by-zero\n");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_invalid_word_1() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line(": 1 2 ;".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "invalid-word\n");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_invalid_word_2() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line(": -1 2 ;".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "invalid-word\n");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_unknown_word() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line("foo".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "?\n");
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_limited_stack() {
    let mut forth = Forth79::new();
    forth.set_stack_size(10);
    let mut buffer = Vec::new();

    forth.interpret_line("1 2 3 4 5".to_string(), &mut buffer);
    forth.interpret_line(". cr 5 6".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "5\nstack-overflow\n");
    assert_eq!(forth.get_stack_state(), [1, 2, 3, 4, 5]);
}
