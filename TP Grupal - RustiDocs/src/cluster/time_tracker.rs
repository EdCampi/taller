use crate::cluster::types::NodeId;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct TimeTracker {
    entries: HashMap<u64, (NodeId, Instant)>,
    order: VecDeque<u64>,
    timeout_ms: Duration,
}

impl TimeTracker {
    pub fn new(timeout_ms: u64) -> TimeTracker {
        let entries: HashMap<u64, (NodeId, Instant)> = HashMap::new();
        let order: VecDeque<u64> = VecDeque::new();
        let timeout_ms = Duration::from_millis(timeout_ms);

        TimeTracker {
            entries,
            order,
            timeout_ms,
        }
    }

    pub fn add_entry(&mut self, id: NodeId, ping_id: u64) {
        self.entries.insert(ping_id.clone(), (id, Instant::now()));
        self.order.push_back(ping_id);
    }

    pub fn verify_timeout(&mut self) -> Option<NodeId> {
        while !self.order.is_empty() {
            let id_peek = self.order.front().unwrap();
            if let Some(entry) = self.entries.get(id_peek) {
                if entry.1.elapsed() > self.timeout_ms {
                    let res = self.order.pop_front().unwrap();
                    let aux = self.entries.remove(&res).unwrap();
                    return Some(aux.0);
                }
                return None;
            } else {
                self.order.pop_front(); // No encontré nada -> Ya saqué antes el tracker.
            }
        }
        None
    }

    pub fn remove_entry(&mut self, pong_id: u64) {
        if pong_id == 0 {
            return;
        }
        self.entries.remove(&pong_id);
    }
}
