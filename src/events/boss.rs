use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use crate::World;

const BOSS_INTERVAL_MINS: u64 = 1;
const BOSS_ID: &str = "npc.dragon";
const BOSS_ROOM: &str = "room.ruins";
const BOSS_HP: u32 = 500;

pub fn start_boss_spawner(
    world: Arc<Mutex<World>>,
    registry: Arc<Mutex<HashMap<String, UnboundedSender<String>>>>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(BOSS_INTERVAL_MINS * 60));

        // Consomme le premier tick immédiat pour attendre avant le 1er spawn
        interval.tick().await;

        loop {
            interval.tick().await;

            let mut spawned = false;

            {
                let mut w = world.lock().await;
                if let Some(room) = w.get_mut_room(BOSS_ROOM) {
                    if !room.npcs.contains(&BOSS_ID.to_string()) {
                        room.npcs.push(BOSS_ID.to_string());

                        if let Some(boss) = w.get_mut_npc(BOSS_ID) {
                            boss.hp = Some(BOSS_HP);
                        }

                        spawned = true;
                        tracing::info!(
                            event = "boss_spawn",
                            room = BOSS_ROOM,
                            boss = BOSS_ID,
                            "world boss spawned"
                        );
                    }
                }
            }

            if spawned {
                let msg = "EVT GLOBAL [ALERTE] Un rugissement retentit... Le Dragon Ancestral a envahi les Ruines !\n";
                let reg = registry.lock().await;
                for tx in reg.values() {
                    let _ = tx.send(msg.to_string());
                }
            }
        }
    });
}