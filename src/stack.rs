use std::fmt;

/// Estructura LIFO básica para asociar a Forth-79.
pub struct Stack {
    data: Vec<i16>,
}

impl Stack {
    pub fn new() -> Stack {
        Stack { data: Vec::new() }
    }

    pub fn push(&mut self, value: i16) {
        self.data.push(value);
    }

    pub fn pop(&mut self) -> Option<i16> {
        self.data.pop()
    }

    pub fn pop_peak(&mut self) -> (Option<i16>, Option<i16>) {
        (self.data.pop(), self.data.pop())
    }

    pub fn remove(&mut self, n: usize) -> Option<i16> {
        if n < self.data.len() {
            return Some(self.data.remove(n));
        }
        None
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn get_items(&self) -> Vec<i16> {
        self.data.clone()
    }
}

impl fmt::Display for Stack {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut aux: Vec<String> = Vec::new();
        for i in self.data.iter() {
            aux.push(i.to_string());
        }
        write!(f, "[{}]", aux.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adding_elements_and_popping_them() {
        let mut stack = Stack::new();
        stack.push(1);
        stack.push(2);
        stack.push(3);

        assert_eq!(stack.pop(), Some(3));
        assert_eq!(stack.pop(), Some(2));
        assert_eq!(stack.pop(), Some(1));
        assert_eq!(stack.pop(), None);
    }

    #[test]
    fn test_trying_to_pop_from_empty_stack() {
        let mut stack = Stack::new();
        assert_eq!(stack.pop(), None);
    }

    #[test]
    fn test_adding_elements_after_popping() {
        let mut stack = Stack::new();
        stack.push(1);
        stack.push(2);

        assert_eq!(stack.pop(), Some(2));

        stack.push(4);
        stack.push(5);

        assert_eq!(stack.pop(), Some(5));
        assert_eq!(stack.pop(), Some(4));
        assert_eq!(stack.pop(), Some(1));
        assert_eq!(stack.pop(), None);
    }

    #[test]
    fn test_len_empty_stack() {
        let stack = Stack::new();
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_len_after_pushing_multiple_elements() {
        let mut stack = Stack::new();
        stack.push(1);
        stack.push(2);
        stack.push(3);
        assert_eq!(stack.len(), 3);
    }

    #[test]
    fn test_len_after_popping_multiple_elements() {
        let mut stack = Stack::new();
        stack.push(1);
        stack.push(2);
        stack.push(3);

        stack.pop();
        assert_eq!(stack.len(), 2);

        stack.pop();
        assert_eq!(stack.len(), 1);

        stack.pop();
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_display_empty_stack() {
        let stack = Stack::new();
        assert_eq!(format!("{}", stack), "[]");
    }

    #[test]
    fn test_display_stack_with_one_element() {
        let mut stack = Stack::new();
        stack.push(1);
        assert_eq!(format!("{}", stack), "[1]");
    }

    #[test]
    fn test_display_stack_with_multiple_elements() {
        let mut stack = Stack::new();
        stack.push(1);
        stack.push(2);
        stack.push(3);
        assert_eq!(format!("{}", stack), "[1, 2, 3]");
    }

    #[test]
    fn test_display_stack_after_popping() {
        let mut stack = Stack::new();
        stack.push(-1);
        stack.push(-2);
        stack.push(-3);
        stack.pop();
        assert_eq!(format!("{}", stack), "[-1, -2]");
    }
}
