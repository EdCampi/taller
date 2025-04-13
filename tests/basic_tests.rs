use forth::forth_79::Forth79;
use std::io;

#[test]
fn test_w_positive_numbers() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 2 3 4 5".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1, 2, 3, 4, 5]);
}

#[test]
fn test_w_negative_numbers() {
    let mut forth = Forth79::new();
    forth.interpret_line("-1 -2 -3 -4 -5".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [-1, -2, -3, -4, -5]);
}

#[test]
fn test_add_1() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 2 +".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [3]);
}

#[test]
fn test_add_2() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 2 3 +".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1, 5]);
}

#[test]
fn test_sub_1() {
    let mut forth = Forth79::new();
    forth.interpret_line("3 4 -".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [-1]);
}

#[test]
fn test_sub_2() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 12 3 -".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1, 9]);
}

#[test]
fn test_mul_1() {
    let mut forth = Forth79::new();
    forth.interpret_line("2 4 *".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [8]);
}

#[test]
fn test_mul_2() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 2 3 *".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1, 6]);
}

#[test]
fn test_div_1() {
    let mut forth = Forth79::new();
    forth.interpret_line("12 3 /".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [4]);
}

#[test]
fn test_div_2() {
    let mut forth = Forth79::new();
    forth.interpret_line("8 3 /".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [2]);
}

#[test]
fn test_div_3() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 12 3 /".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1, 4]);
}

#[test]
fn test_add_sub() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 2 + 4 -".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [-1]);
}

#[test]
fn test_mul_div() {
    let mut forth = Forth79::new();
    forth.interpret_line("2 4 * 3 /".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [2]);
}

#[test]
fn test_mul_add() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 3 4 * +".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [13]);
}

#[test]
fn test_add_mul() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 3 4 + *".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [7]);
}

#[test]
fn test_dup_1() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 dup".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1, 1]);
}

#[test]
fn test_dup_2() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 2 dup".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1, 2, 2]);
}

#[test]
fn test_drop_1() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 drop".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), []);
}

#[test]
fn test_drop_2() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 2 drop".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1]);
}

#[test]
fn test_swap_1() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 2 swap".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [2, 1]);
}

#[test]
fn test_swap_2() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 2 3 swap".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1, 3, 2]);
}

#[test]
fn test_over_1() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 2 over".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1, 2, 1]);
}

#[test]
fn test_over_2() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 2 3 over".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1, 2, 3, 2]);
}

#[test]
fn test_rot_1() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 2 3 rot".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [2, 3, 1]);
}

#[test]
fn test_rot_2() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 2 3 rot rot rot".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1, 2, 3]);
}

#[test]
fn test_word_definition_1() {
    let mut forth = Forth79::new();
    forth.interpret_line(": dup-twice dup dup ;".to_string(), &mut io::stdout());
    forth.interpret_line("1 dup-twice".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1, 1, 1]);
}

#[test]
fn test_word_definition_2() {
    let mut forth = Forth79::new();
    forth.interpret_line(": countup 1 2 3 ;".to_string(), &mut io::stdout());
    forth.interpret_line("countup".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1, 2, 3]);
}

#[test]
fn test_word_redefinition() {
    let mut forth = Forth79::new();
    forth.interpret_line(": foo dup ;".to_string(), &mut io::stdout());
    forth.interpret_line(": foo dup dup ;".to_string(), &mut io::stdout());
    forth.interpret_line("1 foo".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1, 1, 1]);
}

#[test]
fn test_shadowing() {
    let mut forth = Forth79::new();
    forth.interpret_line(": swap dup ;".to_string(), &mut io::stdout());
    forth.interpret_line("1 swap".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1, 1]);
}

#[test]
fn test_shadowing_symbol_1() {
    let mut forth = Forth79::new();
    forth.interpret_line(": + * ;".to_string(), &mut io::stdout());
    forth.interpret_line("3 4 +".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [12]);
}

#[test]
fn test_non_transitive() {
    let mut forth = Forth79::new();
    forth.interpret_line(": foo 5 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": bar foo ;".to_string(), &mut io::stdout());
    forth.interpret_line(": foo 6 ;".to_string(), &mut io::stdout());
    forth.interpret_line("bar foo".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [5, 6]);
}

#[test]
fn test_word_self_definition() {
    let mut forth = Forth79::new();
    forth.interpret_line(": foo 10 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": foo foo 1 + ;".to_string(), &mut io::stdout());
    forth.interpret_line("foo".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [11]);
}

#[test]
fn test_case_insensitive_1() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 Dup Dup dup".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1, 1, 1, 1]);
}

#[test]
fn test_case_insensitive_2() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 2 3 4 Drop Drop drop".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1]);
}

#[test]
fn test_case_insensitive_3() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 2 Swap 3 Swap 4 swap".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [2, 3, 4, 1]);
}

#[test]
fn test_case_insensitive_4() {
    let mut forth = Forth79::new();
    forth.interpret_line("1 2 Over Over over".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1, 2, 1, 2, 1]);
}

#[test]
fn test_case_insensitive_5() {
    let mut forth = Forth79::new();
    forth.interpret_line(": foo dup ;".to_string(), &mut io::stdout());
    forth.interpret_line("1 FOO Foo foo".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1, 1, 1, 1]);
}

#[test]
fn test_case_insensitive_6() {
    let mut forth = Forth79::new();
    forth.interpret_line(": Swap Dup Dup dup ;".to_string(), &mut io::stdout());
    forth.interpret_line("1 swap".to_string(), &mut io::stdout());
    assert_eq!(forth.get_stack_state(), [1, 1, 1, 1]);
}
