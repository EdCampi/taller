use forth::forth_79::Forth79;
use std::io;

#[test]
fn test_do_not_clone() {
    let mut forth2 = Forth79::new();

    forth2.interpret_line(": word1 1 ;".to_string(), &mut io::stdout());
    forth2.interpret_line(": word2 word1 word1 ;".to_string(), &mut io::stdout());
    forth2.interpret_line(": word4 word2 word2 ;".to_string(), &mut io::stdout());
    forth2.interpret_line(": word8 word4 word4 ;".to_string(), &mut io::stdout());
    forth2.interpret_line(": word16 word8 word8 ;".to_string(), &mut io::stdout());
    forth2.interpret_line(": word32 word16 word16 ;".to_string(), &mut io::stdout());
    forth2.interpret_line(": word64 word32 word32 ;".to_string(), &mut io::stdout());
    forth2.interpret_line(": word128 word64 word64 ;".to_string(), &mut io::stdout());
    forth2.interpret_line(": word256 word128 word128 ;".to_string(), &mut io::stdout());
    forth2.interpret_line(": word512 word256 word256 ;".to_string(), &mut io::stdout());
    forth2.interpret_line(
        ": word1024 word512 word512 ;".to_string(),
        &mut io::stdout(),
    );
    forth2.interpret_line(
        ": word2048 word1024 word1024 ;".to_string(),
        &mut io::stdout(),
    );
    forth2.interpret_line(
        ": word4096 word2048 word2048 ;".to_string(),
        &mut io::stdout(),
    );
    forth2.interpret_line(
        ": word8192 word4096 word4096 ;".to_string(),
        &mut io::stdout(),
    );
    forth2.interpret_line(
        ": word16384 word8192 word8192 ;".to_string(),
        &mut io::stdout(),
    );
    forth2.interpret_line(
        ": word32768 word16384 word16384 ;".to_string(),
        &mut io::stdout(),
    );
    forth2.interpret_line(
        ": word65536 word32768 word32768 ;".to_string(),
        &mut io::stdout(),
    );
    forth2.interpret_line(
        ": word131072 word65536 word65536 ;".to_string(),
        &mut io::stdout(),
    );
    forth2.interpret_line(
        ": word262144 word131072 word131072 ;".to_string(),
        &mut io::stdout(),
    );
    forth2.interpret_line(
        ": word524288 word262144 word262144 ;".to_string(),
        &mut io::stdout(),
    );
    forth2.interpret_line(
        ": word1048576 word524288 word524288 ;".to_string(),
        &mut io::stdout(),
    );
    forth2.interpret_line(
        ": word2097152 word1048576 word1048576 ;".to_string(),
        &mut io::stdout(),
    );
    forth2.interpret_line(
        ": word4194304 word2097152 word2097152 ;".to_string(),
        &mut io::stdout(),
    );
    forth2.interpret_line(
        ": word8388608 word4194304 word4194304 ;".to_string(),
        &mut io::stdout(),
    );
    forth2.interpret_line(
        ": word16777216 word8388608 word8388608 ;".to_string(),
        &mut io::stdout(),
    );
    forth2.interpret_line(
        ": word33554432 word16777216 word16777216 ;".to_string(),
        &mut io::stdout(),
    );
    forth2.interpret_line(
        ": word67108864 word33554432 word33554432 ;".to_string(),
        &mut io::stdout(),
    );
    forth2.interpret_line(
        ": word134217728 word67108864 word67108864 ;".to_string(),
        &mut io::stdout(),
    );

    assert_eq!(forth2.get_stack_state(), []);
}
