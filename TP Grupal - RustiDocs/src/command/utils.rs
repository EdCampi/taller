use crate::cluster::types::SlotRange;
use std::collections::HashMap;

/// Parsea el resultado de CLUSTER SLOTS a un HashMap `{ Slots: [ [Node_1_data], ... ] }`
#[allow(unused)]
pub fn parse_flat_cluster_slots(flat: &[String]) -> HashMap<SlotRange, Vec<Vec<String>>> {
    let mut map = HashMap::new();
    let mut i = 0;
    while i + 1 < flat.len() {
        let start = flat[i].parse::<u16>().unwrap_or(0);
        let end = flat[i + 1].parse::<u16>().unwrap_or(0);
        i += 2;

        let mut nodes = Vec::new();
        while i + 3 < flat.len() {
            if flat[i].parse::<u16>().is_ok() && flat[i + 1].parse::<u16>().is_ok() {
                break;
            }
            let node = vec![
                flat[i].clone(),
                flat[i + 1].clone(),
                flat[i + 2].clone(),
                flat[i + 3].clone(),
            ];
            nodes.push(node);
            i += 4;
        }
        map.insert((start, end), nodes);
    }
    map
}
