use crate::cluster::state::node_data::NodeData;
use crate::cluster::types::{KnownNode, NodeId, TimeStamp};
use std::collections::HashMap;
use std::io::Read;
use std::sync::RwLockReadGuard;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn system_time_to_i64(time: SystemTime) -> i64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs() as i64,
        Err(e) => -(e.duration().as_secs() as i64),
    }
}

pub fn read_u64_from_buffer<R: Read>(reader: &mut R) -> Result<u64, &'static str> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf).map_err(|_| "u64 err")?;
    Ok(u64::from_be_bytes(buf))
}

pub fn read_u32_from_buffer<R: Read>(reader: &mut R) -> Result<u32, &'static str> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf).map_err(|_| "u64 err")?;
    Ok(u32::from_be_bytes(buf))
}

pub fn read_u16_from_buffer<R: Read>(reader: &mut R) -> Result<u16, &'static str> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf).map_err(|_| "u16 err")?;
    Ok(u16::from_be_bytes(buf))
}

pub fn read_u8_from_buffer<R: Read>(reader: &mut R) -> Result<u8, &'static str> {
    let mut flag_buf = [0u8; 1];
    reader.read_exact(&mut flag_buf).map_err(|_| "role")?;
    Ok(u8::from_be_bytes(flag_buf))
}

pub fn read_string_from_buffer<R: Read>(
    reader: &mut R,
    len: usize,
) -> Result<String, &'static str> {
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).map_err(|_| "string bytes")?;
    String::from_utf8(buf).map_err(|_| "string utf8")
}

pub fn read_timestamp_from_buffer<R: Read>(reader: &mut R) -> Result<TimeStamp, &'static str> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf).map_err(|_| "timestamp err")?;
    Ok(i64::from_be_bytes(buf))
}

pub fn read_payload_from_buffer<R: Read>(
    reader: &mut R,
    large_data_len: usize,
) -> Result<Vec<u8>, String> {
    let mut data = vec![0u8; large_data_len];
    reader
        .read_exact(&mut data)
        .map_err(|e| format!("Error while reading data itself {}", e))?;
    Ok(data)
}

fn visible_width(s: &str) -> usize {
    let mut width = 0;
    let mut in_escape = false;
    for c in s.chars() {
        match c {
            '\x1b' => in_escape = true,
            'm' if in_escape => in_escape = false,
            _ if !in_escape => width += 1,
            _ => {}
        }
    }
    width
}

fn pad_ansi(s: &str, total_width: usize) -> String {
    let visible = visible_width(s);
    let padding = if total_width > visible {
        total_width - visible
    } else {
        0
    };
    format!("{}{}", s, " ".repeat(padding))
}

pub fn print_slots(
    known_nodes: &RwLockReadGuard<HashMap<NodeId, KnownNode>>,
    node_data_aux: &RwLockReadGuard<NodeData>,
) {
    let mut nodes: Vec<_> = known_nodes.values().collect();
    let myself = KnownNode::new_from_entry(&node_data_aux.get_own_gossip_entry());
    nodes.push(&myself);
    nodes.sort_by_key(|n| n.get_id()); // por si querés que esté ordenado

    println!("{}", "-".repeat(130));
    // Header
    println!(
        "{} {} {} {} {}",
        pad_ansi("ID", 12),
        pad_ansi("SLOTS", 20),
        pad_ansi("FLAGS", 35),
        pad_ansi("MASTER", 12),
        pad_ansi("PFAILS", 8)
    );

    // Separator
    println!("{}", "-".repeat(130));

    // Rows
    for node in nodes {
        let id = pad_ansi(&node.get_id(), 12);
        let slots = pad_ansi(&format!("{:?}", node.get_slots()), 20);
        let flags = pad_ansi(&node.flags_detail(), 35);
        let master = pad_ansi(&node.get_master_id().unwrap_or(&"----".to_string()), 12);
        let pfails = pad_ansi(&format!("{:?}", node.get_pfails()), 8);

        println!("{} {} {} {} {}", id, slots, flags, master, pfails);
    }
    println!("{}", "-".repeat(130));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_system_time_to_i64() {
        let now = SystemTime::now();
        let now_secs = now.duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        assert!((system_time_to_i64(now) - now_secs).abs() <= 1);

        let before_epoch = UNIX_EPOCH - Duration::from_secs(10);
        assert_eq!(system_time_to_i64(before_epoch), -10);
    }

    #[test]
    fn test_read_u64_from_buffer() {
        let data = 123456789u64.to_be_bytes();
        let mut cursor = Cursor::new(data);
        assert_eq!(read_u64_from_buffer(&mut cursor).unwrap(), 123456789u64);
    }

    #[test]
    fn test_read_u16_from_buffer() {
        let data = 54321u16.to_be_bytes();
        let mut cursor = Cursor::new(data);
        assert_eq!(read_u16_from_buffer(&mut cursor).unwrap(), 54321u16);
    }

    #[test]
    fn test_read_u8_from_buffer() {
        let data = [200u8];
        let mut cursor = Cursor::new(data);
        assert_eq!(read_u8_from_buffer(&mut cursor).unwrap(), 200u8);
    }

    #[test]
    fn test_read_string_from_buffer() {
        let s = "hello";
        let data = s.as_bytes();
        let mut cursor = Cursor::new(data);
        assert_eq!(read_string_from_buffer(&mut cursor, 5).unwrap(), "hello");
    }

    #[test]
    fn test_read_timestamp_from_buffer() {
        let ts: i64 = 1234567890;
        let data = ts.to_be_bytes();
        let mut cursor = Cursor::new(data);
        assert_eq!(read_timestamp_from_buffer(&mut cursor).unwrap(), ts);
    }

    #[test]
    fn test_read_payload_from_buffer() {
        let data = vec![1, 2, 3, 4, 5];
        let mut cursor = Cursor::new(data.clone());
        assert_eq!(read_payload_from_buffer(&mut cursor, 5).unwrap(), data);
    }
}
