use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use crate::World;

const BOSS_INTERVAL_SECS: u64 = 60;

pub fn is_boss(npc_id: &str, world_npcs: &HashMap<String, crate::Npc>) -> bool {
    world_npcs.get(npc_id).map(|n| n.boss).unwrap_or(false)
}

pub fn start_boss_spawner(
    world: Arc<Mutex<World>>,
    registry: Arc<Mutex<HashMap<String, UnboundedSender<String>>>>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(BOSS_INTERVAL_SECS));
        interval.tick().await;

        let mut next: usize = 0;

        loop {
            interval.tick().await;

            let mut alert: Option<String> = None;

            {
                let mut w = world.lock().await;

                let boss_ids: Vec<String> = w.npcs.values()
                    .filter(|n| n.boss && n.boss_room.is_some())
                    .map(|n| n.id.clone())
                    .collect();

                if boss_ids.is_empty() { continue; }

                let any_active = boss_ids.iter().any(|bid| {
                    let room_id = w.npcs.get(bid).and_then(|n| n.boss_room.as_ref());
                    room_id.and_then(|rid| w.get_room(rid))
                        .map(|r| r.npcs.contains(bid))
                        .unwrap_or(false)
                });

                if !any_active {
                    let boss_id = &boss_ids[next % boss_ids.len()];
                    next += 1;

                    let (room_id, alert_msg, hp) = {
                        let npc = w.npcs.get(boss_id).unwrap();
                        (
                            npc.boss_room.clone().unwrap(),
                            npc.boss_alert.clone(),
                            npc.max_hp.unwrap_or(npc.hp.unwrap_or(100)),
                        )
                    };

                    if let Some(room) = w.get_mut_room(&room_id) {
                        room.npcs.push(boss_id.clone());
                    }
                    if let Some(npc) = w.get_mut_npc(boss_id) {
                        npc.hp = Some(hp);
                    }

                    if let Some(msg) = alert_msg {
                        alert = Some(format!("EVT GLOBAL [ALERT] {}\n", msg));
                    }

                    tracing::info!(
                        event = "boss_spawn",
                        room = %room_id,
                        boss = %boss_id,
                        "world boss spawned"
                    );
                }
            }

            if let Some(msg) = alert {
                let reg = registry.lock().await;
                for tx in reg.values() {
                    let _ = tx.send(msg.clone());
                }
            }
        }
    });
}
