use crate::stack::Stack;

/// Struct `Operation` sirve para representar las operaciones de los tokens.
/// OBS: Else y Then (End) están para delimintar durente el parseo y en tiempo de ejecució no hacen nada.
pub enum Operation {
    Add,
    Sub,
    Mul,
    Div,
    Dup,
    Drop,
    Swap,
    Over,
    Rot,
    Dot,
    Emit,
    Cr,
    Print(String),
    Eq,
    Lt,
    Gt,
    And,
    Or,
    Not,
    BranchIf(Vec<Operation>, Vec<Operation>),
    BranchElse, // Aunque no hagan nada, los necesito
    BranchEnd,  // para que la función pueda definir bien los ifs anidados.
    N(i16),
    Unknown,
}

impl Operation {
    pub fn apply(&self, stack: &mut Stack, stack_size: usize, buffer: &mut Vec<String>) -> bool {
        match self {
            Operation::N(n) => add_to_the_stack(&n, stack, stack_size, buffer),
            Operation::Add | Operation::Sub | Operation::Mul | Operation::Div => {
                arithmetic_operation(stack, self, buffer)
            }
            Operation::Dup => duplicate_peak(stack, stack_size, buffer),
            Operation::Drop => drop_peak(stack, buffer),
            Operation::Swap => swap_first_two_items(stack, buffer),
            Operation::Over => over_operation(stack, stack_size, buffer),
            Operation::Rot => rotate_stack_by_one(stack, buffer),
            Operation::Dot => pop_and_print(stack, buffer, false),
            Operation::Emit => pop_and_print(stack, buffer, true),
            Operation::Cr => print_operation(buffer, "\n".to_string()),
            Operation::Print(str) => print_operation(buffer, str.to_string()),
            Operation::Eq | Operation::Lt | Operation::Gt => {
                comparison_operation(stack, &self, buffer)
            }
            Operation::And | Operation::Or => boolean_operation(stack, &self, buffer),
            Operation::Not => not_operation(stack, buffer),
            Operation::BranchIf(pos_branch, neg_branch) => {
                browse_if_clause(pos_branch, neg_branch, stack, stack_size, buffer)
            }
            Operation::Unknown => crate::forth_79::Error::UnknownWord.throw_error(buffer),
            Operation::BranchElse | Operation::BranchEnd => true,
        }
    }
}

fn add_to_the_stack(
    n: &i16,
    stack: &mut Stack,
    stack_size: usize,
    buffer: &mut Vec<String>,
) -> bool {
    if stack.len() >= stack_size {
        return crate::forth_79::Error::Overflow.throw_error(buffer);
    }
    stack.push(*n);
    true
}

fn arithmetic_operation(
    stack: &mut Stack,
    operation: &Operation,
    buffer: &mut Vec<String>,
) -> bool {
    let (a, b): (Option<i16>, Option<i16>) = stack.pop_peak();
    if let (Some(a), Some(b)) = (a, b) {
        match operation {
            Operation::Add => {
                stack.push(b + a);
            }
            Operation::Sub => {
                stack.push(b - a);
            }
            Operation::Mul => {
                stack.push(b * a);
            }
            Operation::Div => {
                if a == 0 {
                    return crate::forth_79::Error::DivisionByZero.throw_error(buffer);
                }
                stack.push(b / a);
            }
            _ => return false,
        }
        return true;
    }
    crate::forth_79::Error::Underflow.throw_error(buffer)
}

fn duplicate_peak(stack: &mut Stack, stack_size: usize, buffer: &mut Vec<String>) -> bool {
    if stack.len() + 1 >= stack_size {
        return crate::forth_79::Error::Overflow.throw_error(buffer);
    }
    let a: Option<i16> = stack.pop();
    if let Some(a) = a {
        stack.push(a);
        stack.push(a);
        return true;
    }
    crate::forth_79::Error::Underflow.throw_error(buffer)
}

fn drop_peak(stack: &mut Stack, buffer: &mut Vec<String>) -> bool {
    if let Some(_) = stack.pop() {
        return true;
    }
    crate::forth_79::Error::Underflow.throw_error(buffer)
}

fn swap_first_two_items(stack: &mut Stack, buffer: &mut Vec<String>) -> bool {
    let (a, b): (Option<i16>, Option<i16>) = stack.pop_peak();
    if let (Some(a), Some(b)) = (a, b) {
        stack.push(a);
        stack.push(b);
        return true;
    }
    crate::forth_79::Error::Underflow.throw_error(buffer)
}

fn over_operation(stack: &mut Stack, stack_size: usize, buffer: &mut Vec<String>) -> bool {
    if stack.len() + 1 >= stack_size {
        return crate::forth_79::Error::Overflow.throw_error(buffer);
    }
    let (a, b): (Option<i16>, Option<i16>) = stack.pop_peak();
    if let (Some(a), Some(b)) = (a, b) {
        stack.push(b);
        stack.push(a);
        stack.push(b);
        return true;
    }
    crate::forth_79::Error::Underflow.throw_error(buffer)
}

fn rotate_stack_by_one(stack: &mut Stack, buffer: &mut Vec<String>) -> bool {
    let a: Option<i16> = stack.remove(0);
    if let Some(a) = a {
        stack.push(a);
        return true;
    }
    crate::forth_79::Error::Underflow.throw_error(buffer)
}

fn pop_and_print(stack: &mut Stack, buffer: &mut Vec<String>, is_char: bool) -> bool {
    let a: Option<i16> = stack.pop();
    if let Some(a) = a {
        let res: String = match is_char {
            true => (a as u8 as char).to_string(),
            false => a.to_string(),
        };
        buffer.push(res);
        return true;
    }
    crate::forth_79::Error::Underflow.throw_error(buffer)
}

fn print_operation(buffer: &mut Vec<String>, str: String) -> bool {
    buffer.push(str);
    true
}

fn comparison_operation(
    stack: &mut Stack,
    operation: &Operation,
    buffer: &mut Vec<String>,
) -> bool {
    let (a, b): (Option<i16>, Option<i16>) = stack.pop_peak();
    if let (Some(a), Some(b)) = (a, b) {
        let result: i16 = match operation {
            Operation::Eq => {
                if a == b {
                    -1
                } else {
                    0
                }
            }
            Operation::Lt => {
                if a > b {
                    -1
                } else {
                    0
                }
            }
            Operation::Gt => {
                if a < b {
                    -1
                } else {
                    0
                }
            }
            _ => -1,
        };
        stack.push(result);
        return true;
    }
    crate::forth_79::Error::Underflow.throw_error(buffer)
}

fn boolean_operation(stack: &mut Stack, operation: &Operation, buffer: &mut Vec<String>) -> bool {
    let (a, b): (Option<i16>, Option<i16>) = stack.pop_peak();
    if let (Some(a), Some(b)) = (a, b) {
        match operation {
            Operation::And => {
                stack.push(if a == b && a == -1 { -1 } else { 0 });
            }
            Operation::Or => {
                stack.push(if a == -1 || b == -1 { -1 } else { 0 });
            }
            _ => return false,
        }
        return true;
    }
    crate::forth_79::Error::Underflow.throw_error(buffer)
}

fn not_operation(stack: &mut Stack, buffer: &mut Vec<String>) -> bool {
    let a: Option<i16> = stack.pop();
    if let Some(a) = a {
        let result: i16 = if a == 0 { -1 } else { 0 };
        stack.push(result);
        return true;
    }
    crate::forth_79::Error::Underflow.throw_error(buffer)
}

fn browse_if_clause(
    pos_branch: &Vec<Operation>,
    neg_branch: &Vec<Operation>,
    stack: &mut Stack,
    stack_size: usize,
    buffer: &mut Vec<String>,
) -> bool {
    let condition = stack.pop();
    if let Some(condition) = condition {
        if condition == 0 {
            for op in neg_branch {
                if !op.apply(stack, stack_size, buffer) {
                    return false;
                }
            }
            return true;
        }
        for op in pos_branch {
            if !op.apply(stack, stack_size, buffer) {
                return false;
            }
        }
        return true;
    }
    crate::forth_79::Error::Underflow.throw_error(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_up_full_stack() -> Stack {
        let mut stack = Stack::new();
        stack.push(1);
        stack.push(2);
        stack
    }

    fn set_up_full_stack_w_neg_items() -> Stack {
        let mut stack = Stack::new();
        stack.push(-1);
        stack.push(-2);
        stack
    }

    fn set_up_full_stack_w_mixed_items() -> Stack {
        let mut stack = Stack::new();
        stack.push(-1);
        stack.push(2);
        stack
    }

    fn set_up_one_item_stack() -> Stack {
        let mut stack = Stack::new();
        stack.push(1);
        stack
    }

    fn set_up_empty_stack() -> Stack {
        Stack::new()
    }

    #[test]
    fn test_add_sums_items_in_a_stack() {
        let mut stack = set_up_full_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Add;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0); // Reviso que no se haya pusheado nada al buffer.
        assert_eq!(stack.len(), 1); // Reviso que haya modificado bien la longitud de la pila.
        assert_eq!(stack.pop().unwrap(), 3); // Reviso que haya pusheado el resultado correcto.
    }

    #[test]
    fn test_add_sums_neg_items_in_a_stack() {
        let mut stack = set_up_full_stack_w_neg_items();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Add;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), -3);
    }

    #[test]
    fn test_add_sums_mixed_items_a_stack() {
        let mut stack = set_up_full_stack_w_mixed_items();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Add;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 1);
    }

    #[test]
    fn test_add_cant_sum_1_item_in_a_stack() {
        let mut stack = set_up_one_item_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Add;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0); // Las operaciones consumen los datos que tocan, no hay undo.
    }

    #[test]
    fn test_add_cant_sum_0_items_in_a_stack() {
        let mut stack = set_up_empty_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Add;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    /* TEST RESTA */

    #[test]
    fn test_sub_subs_items_in_a_stack() {
        let mut stack = set_up_full_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Sub;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), -1);
    }

    #[test]
    fn test_sub_subs_neg_items_in_a_stack() {
        let mut stack = set_up_full_stack_w_neg_items();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Sub;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 1);
    }

    #[test]
    fn test_sub_subs_mixed_items_a_stack() {
        let mut stack = set_up_full_stack_w_mixed_items();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Sub;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), -3);
    }

    #[test]
    fn test_sub_cant_sub_1_item_in_a_stack() {
        let mut stack = set_up_one_item_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Sub;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0); // Las operaciones consumen los datos que tocan, no hay undo.
    }

    #[test]
    fn test_sub_cant_sub_0_items_in_a_stack() {
        let mut stack = set_up_empty_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Sub;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    /* TEST MULTIPLICACIÓN */

    #[test]
    fn test_mul_multiplies_items_in_a_stack() {
        let mut stack = set_up_full_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Mul;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 2);
    }

    #[test]
    fn test_mul_multiplies_neg_items_in_a_stack() {
        let mut stack = set_up_full_stack_w_neg_items();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Mul;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 2);
    }

    #[test]
    fn test_mul_multiplies_mixed_items_a_stack() {
        let mut stack = set_up_full_stack_w_mixed_items();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Mul;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), -2);
    }

    #[test]
    fn test_mul_multiplies_with_a_0_left() {
        let mut stack = Stack::new();
        stack.push(0);
        stack.push(1);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Mul;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 0);
    }

    #[test]
    fn test_mul_multiplies_with_a_0_right() {
        let mut stack = Stack::new();
        stack.push(1);
        stack.push(0);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Mul;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 0);
    }

    #[test]
    fn test_mul_cant_multiply_1_item_in_a_stack() {
        let mut stack = set_up_one_item_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Mul;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0); // Las operaciones consumen los datos que tocan, no hay undo.
    }

    #[test]
    fn test_mul_cant_multiply_0_items_in_a_stack() {
        let mut stack = set_up_empty_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Mul;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    /* TEST DIVISIÓN */

    #[test]
    fn test_div_divides_items_in_a_stack() {
        let mut stack = set_up_full_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Div;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 0);
    }

    #[test]
    fn test_div_divides_neg_items_in_a_stack() {
        let mut stack = set_up_full_stack_w_neg_items();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Div;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 0);
    }

    #[test]
    fn test_div_divides_mixed_items_a_stack() {
        let mut stack = set_up_full_stack_w_mixed_items();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Div;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 0);
    }

    #[test]
    fn test_div_divides_with_a_0_left() {
        let mut stack = Stack::new();
        stack.push(0);
        stack.push(10);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Div;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 0);
    }

    #[test]
    fn test_div_divides_with_a_0_right() {
        let mut stack = Stack::new();
        stack.push(10);
        stack.push(0);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Div;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_div_cant_div_1_item_in_a_stack() {
        let mut stack = set_up_one_item_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Div;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0); // Las operaciones consumen los datos que tocan, no hay undo.
    }

    #[test]
    fn test_div_cant_div_0_items_in_a_stack() {
        let mut stack = set_up_empty_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Div;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    /* TESTS DROP */

    #[test]
    fn test_dup_on_a_full_stack() {
        let mut stack = set_up_full_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Dup;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 3);
        assert_eq!(stack.pop().unwrap(), 2);
        assert_eq!(stack.pop().unwrap(), 2);
        assert_eq!(stack.pop().unwrap(), 1);
    }

    #[test]
    fn test_dup_overflow() {
        let mut stack = set_up_full_stack();
        let stack_size: usize = 2;
        let mut buffer = Vec::new();
        let operation = Operation::Dup;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 2);
    }

    #[test]
    fn test_dup_underflow() {
        let mut stack = set_up_empty_stack();
        let stack_size: usize = 2;
        let mut buffer = Vec::new();
        let operation = Operation::Dup;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    /* TESTS DROP */

    #[test]
    fn test_drop_success() {
        let mut stack = set_up_full_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Drop;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 1);
    }

    #[test]
    fn test_drop_underflow() {
        let mut stack = set_up_empty_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Drop;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    /* TESTS SWAP */
    #[test]
    fn test_swap_success() {
        let mut stack = set_up_full_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Swap;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.pop().unwrap(), 1);
        assert_eq!(stack.pop().unwrap(), 2);
    }

    #[test]
    fn test_swap_underflow_w_1_item_in_a_stack() {
        let mut stack = set_up_one_item_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Swap;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_swap_underflow_w_0_items_in_a_stack() {
        let mut stack = set_up_empty_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Swap;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    /* TESTS OVER */
    #[test]
    fn test_over_success() {
        let mut stack = set_up_full_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Over;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 3);
        assert_eq!(stack.pop().unwrap(), 1);
        assert_eq!(stack.pop().unwrap(), 2);
        assert_eq!(stack.pop().unwrap(), 1);
    }

    #[test]
    fn test_over_overflow() {
        let mut stack = set_up_full_stack();
        let stack_size: usize = 2;
        let mut buffer = Vec::new();
        let operation = Operation::Over;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 2); // Se llenó y se hizo push una vez más.
    }

    #[test]
    fn test_over_underflow_w_1_item_in_a_stack() {
        let mut stack = set_up_one_item_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Over;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_over_underflow_w_0_items_in_a_stack() {
        let mut stack = set_up_empty_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Over;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    /* TESTS ROT */
    #[test]
    fn test_rot_success() {
        let mut stack = set_up_full_stack();
        stack.push(3);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Rot;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 3);
        assert_eq!(stack.pop().unwrap(), 1);
        assert_eq!(stack.pop().unwrap(), 3);
        assert_eq!(stack.pop().unwrap(), 2);
    }

    #[test]
    fn test_rot_w_1_item_in_a_stack() {
        let mut stack = set_up_one_item_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Rot;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 1);
    }

    #[test]
    fn test_rot_underflow_w_0_items_in_a_stack() {
        let mut stack = set_up_empty_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Rot;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    /* TESTS DOT */
    #[test]
    fn test_dot_success() {
        let mut stack = set_up_full_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Dot;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn test_dot_underflow_w_0_items_in_a_stack() {
        let mut stack = set_up_empty_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Dot;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    /* TESTS EMIT */

    #[test]
    fn test_emit_success() {
        let mut stack = set_up_full_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Emit;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn test_emit_underflow_w_0_items_in_a_stack() {
        let mut stack = set_up_empty_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Emit;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    /* TESTS CR */

    #[test]
    fn test_cr_success_w_full_stack() {
        let mut stack = set_up_full_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Cr;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0], "\n");
        assert_eq!(stack.len(), 2);
    }

    #[test]
    fn test_cr_success_w_empty_stack() {
        let mut stack = set_up_empty_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Cr;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0], "\n");
        assert_eq!(stack.len(), 0);
    }

    /* TESTS PRINT */

    #[test]
    fn test_print_success_w_full_stack() {
        let mut stack = set_up_full_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Print("Hola".to_string());

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0], "Hola");
        assert_eq!(stack.len(), 2);
    }

    #[test]
    fn test_print_success_w_empty_stack() {
        let mut stack = set_up_empty_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Print("Mundo".to_string());

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0], "Mundo");
        assert_eq!(stack.len(), 0);
    }

    /* TEST == */

    #[test]
    fn test_equals_both_equals() {
        let mut stack = Stack::new();
        stack.push(-1);
        stack.push(-1);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Eq;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), -1);
    }

    #[test]
    fn test_equals_differents() {
        let mut stack = Stack::new();
        stack.push(1);
        stack.push(0);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Eq;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 0);
    }

    #[test]
    fn test_equals_1_item_stack() {
        let mut stack = set_up_one_item_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Eq;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_equals_empty_stack() {
        let mut stack = set_up_empty_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Eq;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    /* TEST < */

    #[test]
    fn test_less_than_a_gt_b() {
        let mut stack = Stack::new();
        stack.push(10);
        stack.push(-1);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Lt;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 0);
    }

    #[test]
    fn test_less_than_a_lt_b() {
        let mut stack = Stack::new();
        stack.push(-1);
        stack.push(10);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Lt;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), -1);
    }

    #[test]
    fn test_less_than_underflow_1_item_stack() {
        let mut stack = set_up_one_item_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Lt;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_less_than_underflow_empty_stack() {
        let mut stack = set_up_empty_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Lt;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    /* TESTS > */

    #[test]
    fn test_greater_than_a_gt_b() {
        let mut stack = Stack::new();
        stack.push(10);
        stack.push(-1);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Gt;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), -1);
    }

    #[test]
    fn test_greater_than_a_lt_b() {
        let mut stack = Stack::new();
        stack.push(-1);
        stack.push(10);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Gt;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 0);
    }

    #[test]
    fn test_greater_than_underflow_1_item_stack() {
        let mut stack = set_up_one_item_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Gt;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_greater_than_underflow_empty_stack() {
        let mut stack = set_up_empty_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Gt;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    /* TESTS AND */

    #[test]
    fn test_and_both_true() {
        let mut stack = Stack::new();
        stack.push(-1);
        stack.push(-1);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::And;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), -1);
    }

    #[test]
    fn test_and_both_false() {
        let mut stack = Stack::new();
        stack.push(0);
        stack.push(0);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::And;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 0);
    }

    #[test]
    fn test_and_true_false() {
        let mut stack = Stack::new();
        stack.push(-1);
        stack.push(0);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::And;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 0);
    }

    #[test]
    fn test_and_false_true() {
        let mut stack = Stack::new();
        stack.push(0);
        stack.push(-1);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::And;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 0);
    }

    #[test]
    fn test_and_1_item_stack() {
        let mut stack = set_up_one_item_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::And;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_and_empty_stack() {
        let mut stack = set_up_empty_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::And;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    /* TEST OR */

    #[test]
    fn test_or_both_true() {
        let mut stack = Stack::new();
        stack.push(-1);
        stack.push(-1);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Or;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), -1);
    }

    #[test]
    fn test_or_both_false() {
        let mut stack = Stack::new();
        stack.push(0);
        stack.push(0);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Or;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 0);
    }

    #[test]
    fn test_or_true_false() {
        let mut stack = Stack::new();
        stack.push(-1);
        stack.push(0);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Or;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), -1);
    }

    #[test]
    fn test_or_false_true() {
        let mut stack = Stack::new();
        stack.push(0);
        stack.push(-1);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Or;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), -1);
    }

    #[test]
    fn test_or_1_item_stack() {
        let mut stack = set_up_one_item_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Or;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_or_empty_stack() {
        let mut stack = set_up_empty_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Or;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    /* TESTS NOT */
    #[test]
    fn test_not_true() {
        let mut stack = Stack::new();
        stack.push(-1);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Not;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 0);
    }

    #[test]
    fn test_not_false() {
        let mut stack = Stack::new();
        stack.push(0);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Not;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), -1);
    }

    #[test]
    fn test_not_underflow() {
        let mut stack = Stack::new();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Not;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    /* TESTS IF */
    #[test]
    fn test_if_underflow() {
        let mut stack = set_up_empty_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::BranchIf(vec![], vec![]);

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_if_true() {
        let mut stack = Stack::new();
        stack.push(-1);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::BranchIf(
            vec![Operation::Print("IZQ".to_string())],
            vec![Operation::Print("IZQ".to_string())],
        );

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0], "IZQ");
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_if_false() {
        let mut stack = Stack::new();
        stack.push(0);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::BranchIf(
            vec![Operation::Print("IZQ".to_string())],
            vec![Operation::Print("DER".to_string())],
        );

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0], "DER");
        assert_eq!(stack.len(), 0);
    }

    /* TESTS ELSE Y THEN */

    #[test]
    fn test_else_does_nothing() {
        let mut stack = Stack::new();
        stack.push(-1);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::BranchElse;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn test_then_does_nothing() {
        let mut stack = Stack::new();
        stack.push(-1);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::BranchEnd;

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
    }

    /* TEST NUMBER */
    #[test]
    fn test_number_pushes_correctly() {
        let mut stack = set_up_empty_stack();
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::N(10);

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 0);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 10);
    }

    #[test]
    fn test_number_overflow() {
        let mut stack = Stack::new();
        let stack_size: usize = 1;
        let mut buffer = Vec::new();
        let operation = Operation::N(2);

        assert!(operation.apply(&mut stack, stack_size, &mut buffer));
        assert!(!operation.apply(&mut stack, stack_size, &mut buffer)); // 2da vez no pasa.
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop().unwrap(), 2);
    }

    /* TEST UNKNOWN */

    #[test]
    fn test_unknown_prints_on_buffer() {
        let mut stack = Stack::new();
        stack.push(-1);
        let stack_size: usize = 10;
        let mut buffer = Vec::new();
        let operation = Operation::Unknown;

        assert!(!operation.apply(&mut stack, stack_size, &mut buffer));
        assert_eq!(buffer.len(), 1);
        assert_eq!(stack.len(), 1);
    }
}
