use crate::cluster::state::flags::{FAIL, NodeFlags, SLAVE};
use crate::cluster::state::node_data::NodeData;
use crate::cluster::types::{
    Epoch, KnownNode, NodeId, NodeMessage, PROMOTION_TYPE, SlotRange, TimeStamp,
};
use crate::cluster::utils::{read_string_from_buffer, read_u16_from_buffer, read_u64_from_buffer};
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

/// Constantes para la promoción de réplicas
const PROMOTION_DELAY: u64 = 2000; // 1 segundo de delay antes de iniciar promoción
static DEFINITIVE_FAILURE: TimeStamp = i64::MAX;

/// Inicia el proceso de promoción de réplica
pub fn start_promotion(
    failed_master_id: NodeId,
    sender_data_lock: Arc<RwLock<NodeData>>,
    known_nodes_lock: Arc<RwLock<HashMap<NodeId, KnownNode>>>,
    broadcast_sender: Sender<Vec<u8>>,
) {
    thread::spawn(move || {
        // Esperar un poco para que se propague el estado FAIL
        thread::sleep(Duration::from_millis(PROMOTION_DELAY));

        let known_nodes = known_nodes_lock.read().unwrap();

        // Buscar réplicas del master fallido
        let replicas: Vec<&KnownNode> = known_nodes
            .values()
            .filter(|node| {
                node.is_slave()
                    && node
                        .get_master_id()
                        .map(|master_id| master_id == &failed_master_id)
                        .unwrap_or(false)
                    && !node.get_flags().is_set(FAIL)
            })
            .collect();

        if replicas.is_empty() {
            println!(
                "\x1b[31m[PROMOTION] No hay réplicas disponibles para el master fallido {}\x1b[0m",
                failed_master_id
            );
            return;
        }

        // Elegir la mejor réplica (con mayor offset de replicación)
        let best_replica = replicas
            .iter()
            .max_by_key(|replica| replica.get_last_update_time())
            .unwrap();

        println!(
            "[PROMOTION] Iniciando promoción de réplica {} para reemplazar master {}",
            best_replica.get_id(),
            failed_master_id
        );

        // Obtener slots del master fallido
        let failed_master = known_nodes.get(&failed_master_id).unwrap();
        let slots_to_assume = failed_master.get_slots();

        let best_replica_id = best_replica.get_id().clone();
        let current_epoch = sender_data_lock.read().unwrap().get_cepoch() + 1;
        drop(known_nodes);

        // Crear mensaje de promoción
        let promotion_msg = PromotionMessage::new(
            best_replica_id.clone(),
            failed_master_id.clone(),
            slots_to_assume,
            current_epoch,
        );

        // Enviar mensaje de promoción
        let bytes = promotion_msg.serialize();
        let sender_data = sender_data_lock.read().unwrap();
        let broadcast_message = NodeMessage::new(
            sender_data.get_id(),
            sender_data.get_ip(),
            sender_data.get_port(),
            PROMOTION_TYPE,
            bytes.len() as u16,
            bytes,
        );
        drop(sender_data);

        if let Err(_) = broadcast_sender.send(broadcast_message.serialize()) {
            println!("[PROMOTION] Error al enviar mensaje de promoción");
        } else {
            let _ = process_promotion_msg(broadcast_message, &sender_data_lock, &known_nodes_lock); // A mi no me va a llegar, entonces lo proceso...
        }
    });
}

/// Procesa un mensaje de promoción recibido
pub fn process_promotion_msg(
    message: NodeMessage,
    node_data_lock: &Arc<RwLock<NodeData>>,
    known_nodes_lock: &Arc<RwLock<HashMap<NodeId, KnownNode>>>,
) -> Result<(), String> {
    let promotion_msg = PromotionMessage::from_bytes(&message.get_payload())?;

    println!(
        "[PROMOTION] Procesando promoción de {} a master",
        promotion_msg.get_candidate_id()
    );

    let candidate_id = promotion_msg.get_candidate_id().clone();
    let failed_master_id = promotion_msg.get_failed_master_id().clone();
    let slots_to_assume = promotion_msg.get_slots_to_assume().clone();
    let config_epoch = promotion_msg.get_config_epoch();

    // Verificar que el master fallido realmente falló
    {
        let known_nodes = known_nodes_lock.read().unwrap();
        if let Some(failed_master) = known_nodes.get(&failed_master_id) {
            if !failed_master.get_flags().is_set(FAIL) {
                println!("El master no está marcado como fallido");
                return Ok(());
            }
            let failed_slots = failed_master.get_slots();
            if failed_slots.0 == 0 && failed_slots.1 == 0 {
                println!("El master ya fue limpiado");
                return Ok(());
            }
        }
    }

    let mut myself = node_data_lock.write().unwrap();
    let mut known_nodes = known_nodes_lock.write().unwrap();
    if NodeFlags::state_contains(myself.get_state(), SLAVE) {
        if myself.get_id() == candidate_id {
            myself.set_as_master();
            myself.set_slots((slots_to_assume.0, slots_to_assume.1));
            myself.add_cepoch();

            if let Some(failed_master) = known_nodes.get_mut(&failed_master_id) {
                failed_master.clear_slots();
                failed_master.set_last_pong_time(Some(DEFINITIVE_FAILURE));
            }
            println!(
                "\x1b[32m[PROMOTION] Réplica {} promovida exitosamente a master\x1b[0m",
                candidate_id
            );
        } else if myself.get_master_id().unwrap() == *failed_master_id {
            myself.set_as_slave(candidate_id.clone());
        }
    }
    drop(myself);

    // Procesar la promoción
    if let Some(candidate) = known_nodes.get_mut(&candidate_id) {
        if !candidate.is_slave() {
            println!("El candidato no es una réplica");
            return Ok(());
        }
        candidate.promote_to_master(slots_to_assume, config_epoch);
        if let Some(failed_master) = known_nodes.get_mut(&failed_master_id) {
            failed_master.clear_slots();
        }
        println!(
            "[PROMOTION] Réplica {} promovida exitosamente a master",
            candidate_id
        );
    }
    Ok(())
}

/// Mensaje de promoción de réplica
#[derive(Debug)]
pub struct PromotionMessage {
    candidate_id: NodeId,
    failed_master_id: NodeId,
    slots_to_assume: SlotRange,
    config_epoch: Epoch,
}

impl PromotionMessage {
    pub fn new(
        candidate_id: NodeId,
        failed_master_id: NodeId,
        slots_to_assume: SlotRange,
        config_epoch: Epoch,
    ) -> Self {
        Self {
            candidate_id,
            failed_master_id,
            slots_to_assume,
            config_epoch,
        }
    }

    pub fn get_candidate_id(&self) -> &NodeId {
        &self.candidate_id
    }

    pub fn get_failed_master_id(&self) -> &NodeId {
        &self.failed_master_id
    }

    pub fn get_slots_to_assume(&self) -> SlotRange {
        self.slots_to_assume
    }

    pub fn get_config_epoch(&self) -> Epoch {
        self.config_epoch
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = vec![];

        // Candidate ID
        let candidate_id_bytes = self.candidate_id.as_bytes();
        let candidate_id_len = candidate_id_bytes.len() as u16;
        buffer.extend_from_slice(&candidate_id_len.to_be_bytes());
        buffer.extend_from_slice(candidate_id_bytes);

        // Failed master ID
        let failed_master_id_bytes = self.failed_master_id.as_bytes();
        let failed_master_id_len = failed_master_id_bytes.len() as u16;
        buffer.extend_from_slice(&failed_master_id_len.to_be_bytes());
        buffer.extend_from_slice(failed_master_id_bytes);

        // Slots to assume
        buffer.extend_from_slice(&self.slots_to_assume.0.to_be_bytes());
        buffer.extend_from_slice(&self.slots_to_assume.1.to_be_bytes());

        // Config epoch
        buffer.extend_from_slice(&self.config_epoch.to_be_bytes());

        buffer
    }

    pub fn from_bytes(mut data: &[u8]) -> Result<Self, String> {
        // Candidate ID
        let candidate_id_len = read_u16_from_buffer(&mut data)?;
        let candidate_id = read_string_from_buffer(&mut data, candidate_id_len as usize)?;

        // Failed master ID
        let failed_master_id_len = read_u16_from_buffer(&mut data)?;
        let failed_master_id = read_string_from_buffer(&mut data, failed_master_id_len as usize)?;

        // Slots to assume
        let slots_start = read_u16_from_buffer(&mut data)?;
        let slots_end = read_u16_from_buffer(&mut data)?;
        let slots_to_assume = (slots_start, slots_end);

        // Config epoch
        let config_epoch = read_u64_from_buffer(&mut data)?;

        Ok(PromotionMessage {
            candidate_id,
            failed_master_id,
            slots_to_assume,
            config_epoch,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_promotion_message_serialization() {
        let msg = PromotionMessage::new("replica1".to_string(), "master1".to_string(), (0, 3), 42);

        let serialized = msg.serialize();
        let deserialized = PromotionMessage::from_bytes(&serialized).unwrap();

        assert_eq!(deserialized.get_candidate_id(), "replica1");
        assert_eq!(deserialized.get_failed_master_id(), "master1");
        assert_eq!(deserialized.get_slots_to_assume(), (0, 3));
        assert_eq!(deserialized.get_config_epoch(), 42);
    }
}
