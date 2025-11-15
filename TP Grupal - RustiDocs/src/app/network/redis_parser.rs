use std::io::BufRead;

use crate::{
    app::{network::header::Message, operation::generic::ParsableBytes},
    network::RespMessage,
};

pub fn read_resp_bulk_string<R: BufRead>(reader: &mut R) -> Result<Vec<u8>, std::io::Error> {
    let mut header = Vec::new();
    // Lee el header hasta \n (incluye \r\n)
    let bytes_leidos = reader.read_until(b'\n', &mut header)?;
    // Ejemplo de header: b"$5\r\n"
    println!(
        "Se leyo {:?} en {} bytes equivalente a \n{}",
        header,
        bytes_leidos,
        String::from_utf8_lossy(&header)
    );

    if header.len() < 4 || header[0] != b'$' {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Header RESP inválido",
        ));
    }
    // Extrae la longitud
    let len_str = std::str::from_utf8(&header[1..header.len() - 2]).unwrap();
    let len: usize = len_str.parse().unwrap();

    // Lee el contenido exacto
    let mut content = vec![0u8; len];
    reader.read_exact(&mut content)?;

    // Lee el \r\n final
    let mut crlf = [0u8; 2];
    reader.read_exact(&mut crlf)?;

    Ok(content)
}

pub fn read_resp_simple_string<R: BufRead>(reader: &mut R) -> Result<Vec<u8>, std::io::Error> {
    let mut line = Vec::new();
    let bytes_leidos = reader.read_until(b'\n', &mut line)?;
    println!(
        "Se leyo {:?} en {} bytes equivalente a \n{}",
        line,
        bytes_leidos,
        String::from_utf8_lossy(&line)
    );

    // Ejemplo de línea: b"+OK\r\n"
    if line.len() < 3 || line[0] != b'+' {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Simple String RESP inválido",
        ));
    }
    // Extrae el string sin el prefijo '+' y sin el \r\n final
    let content = &line[1..line.len() - 2];
    Ok(content.to_vec())
}

pub fn sub_to_channel(channel_name: &str) -> Vec<u8> {
    [
        b"*2\r\n$9\r\nSUBSCRIBE\r\n$",
        channel_name.len().to_string().as_bytes(),
        b"\r\n",
        channel_name.as_bytes(),
        b"\r\n",
    ]
    .concat()
}

pub fn content_to_message<D, O>(content: RespMessage) -> Option<Message<D, O>>
where
    D: Clone + ParsableBytes,
    O: Clone + ParsableBytes,
{
    match content {
        RespMessage::SimpleString(string) => Some(Message::resp_to_message(&string)?),
        RespMessage::BulkString(Some(bytes)) => {
            // Convertir los bytes a string para procesar
            let content_string = String::from_utf8_lossy(&bytes);
            println!("[REDIS_PARSER] Procesando BulkString: {}", content_string);
            Some(Message::resp_to_message(&content_string)?)
        }
        RespMessage::BulkString(None) => {
            println!("[REDIS_PARSER] Ignorando BulkString nulo");
            None
        }
        RespMessage::Integer(value) => {
            // Los mensajes Integer de Redis (como contadores de suscripción) no son mensajes de estado válidos
            // Los ignoramos silenciosamente y continuamos esperando el mensaje correcto
            println!(
                "[REDIS_PARSER] Ignorando mensaje Integer: {} (probablemente contador de suscripción)",
                value
            );
            None
        }
        _ => None,
    }
}
