use forth::forth_79::Forth79;
use std::io;

#[test]
fn test_unit_computation_1() {
    let mut forth = Forth79::new();

    forth.interpret_line(": meter 100 * ;".to_string(), &mut io::stdout());
    forth.interpret_line(": decimeter 10 * ;".to_string(), &mut io::stdout());
    forth.interpret_line(": centimeter 1 * ;".to_string(), &mut io::stdout());
    forth.interpret_line(
        "1 meter 5 decimeter 2 centimeter + +".to_string(),
        &mut io::stdout(),
    );

    assert_eq!(forth.get_stack_state(), [152]);
}

#[test]
fn test_unit_computation_2() {
    let mut forth = Forth79::new();

    forth.interpret_line(": seconds 1 * ;".to_string(), &mut io::stdout());
    forth.interpret_line(": minutes 60 * seconds ;".to_string(), &mut io::stdout());
    forth.interpret_line(": hours 60 * minutes ;".to_string(), &mut io::stdout());
    forth.interpret_line(
        "2 hours 13 minutes 5 seconds + +".to_string(),
        &mut io::stdout(),
    );

    assert_eq!(forth.get_stack_state(), [7985]);
}

#[test]
fn test_constant_summation() {
    let mut forth = Forth79::new();

    forth.interpret_line(": one1 1 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": one2  one1 one1 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": one4  one2 one2 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": one8  one4 one4 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": one16 one8 one8 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": add1 + ;".to_string(), &mut io::stdout());
    forth.interpret_line(": add2  add1 add1 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": add4  add2 add2 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": add8  add4 add4 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": add16 add8 add8 ;".to_string(), &mut io::stdout());
    forth.interpret_line("0".to_string(), &mut io::stdout());
    forth.interpret_line("one16".to_string(), &mut io::stdout());
    forth.interpret_line("add16".to_string(), &mut io::stdout());

    assert_eq!(forth.get_stack_state(), [16]);
}

#[test]
fn test_linear_summation() {
    let mut forth = Forth79::new();

    forth.interpret_line(": next1 dup 1 + ;".to_string(), &mut io::stdout());
    forth.interpret_line(": next2  next1 next1 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": next4  next2 next2 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": next8  next4 next4 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": next16 next8 next8 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": add1 + ;".to_string(), &mut io::stdout());
    forth.interpret_line(": add2  add1 add1 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": add4  add2 add2 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": add8  add4 add4 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": add16 add8 add8 ;".to_string(), &mut io::stdout());
    forth.interpret_line("0".to_string(), &mut io::stdout());
    forth.interpret_line("next16".to_string(), &mut io::stdout());
    forth.interpret_line("add16".to_string(), &mut io::stdout());

    assert_eq!(forth.get_stack_state(), [136]);
}

#[test]
fn test_geometric_summation() {
    let mut forth = Forth79::new();

    forth.interpret_line(": next1 dup 2 * ;".to_string(), &mut io::stdout());
    forth.interpret_line(": next2  next1 next1 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": next4  next2 next2 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": next8  next4 next4 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": add1 + ;".to_string(), &mut io::stdout());
    forth.interpret_line(": add2  add1 add1 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": add4  add2 add2 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": add8  add4 add4 ;".to_string(), &mut io::stdout());
    forth.interpret_line("1".to_string(), &mut io::stdout());
    forth.interpret_line("next8".to_string(), &mut io::stdout());
    forth.interpret_line("add8".to_string(), &mut io::stdout());

    assert_eq!(forth.get_stack_state(), [511]);
}

#[test]
fn test_power_of_2() {
    let mut forth = Forth79::new();

    forth.interpret_line(": next1 dup 2 * ;".to_string(), &mut io::stdout());
    forth.interpret_line(": next2  next1 next1 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": next4  next2 next2 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": mul1 * ;".to_string(), &mut io::stdout());
    forth.interpret_line(": mul2  mul1 mul1 ;".to_string(), &mut io::stdout());
    forth.interpret_line(": mul4  mul2 mul2 ;".to_string(), &mut io::stdout());
    forth.interpret_line("1".to_string(), &mut io::stdout());
    forth.interpret_line("next4".to_string(), &mut io::stdout());
    forth.interpret_line("mul4".to_string(), &mut io::stdout());

    assert_eq!(forth.get_stack_state(), [1024]);
}

#[test]
fn test_digit_to_string() {
    let mut forth = Forth79::new();
    let mut buffer = Vec::new();

    forth.interpret_line(": f".to_string(), &mut buffer);
    forth.interpret_line("  dup 0 = if".to_string(), &mut buffer);
    forth.interpret_line("    drop .\" zero\"".to_string(), &mut buffer);
    forth.interpret_line("  else dup 1 = if".to_string(), &mut buffer);
    forth.interpret_line("    drop .\" one\"".to_string(), &mut buffer);
    forth.interpret_line("  else dup 2 = if".to_string(), &mut buffer);
    forth.interpret_line("    drop .\" two\"".to_string(), &mut buffer);
    forth.interpret_line("  then then then ;".to_string(), &mut buffer);
    forth.interpret_line("0 f cr".to_string(), &mut buffer);
    forth.interpret_line("1 f cr".to_string(), &mut buffer);
    forth.interpret_line("2 f cr".to_string(), &mut buffer);

    assert_eq!(String::from_utf8(buffer).unwrap(), "zero\none\ntwo\n");
    assert_eq!(forth.get_stack_state(), []);
}
