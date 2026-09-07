use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use crate::World;

const REGEN_INTERVAL_SECS: u64 = 15;
const REGEN_COOLDOWN_SECS: u64 = 60;

pub fn start_npc_regen(world: Arc<Mutex<World>>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(REGEN_INTERVAL_SECS));
        interval.tick().await;

        loop {
            interval.tick().await;
            let mut w = world.lock().await;
            let now = Instant::now();
            for npc in w.npcs.values_mut() {
                if let (Some(hp), Some(max_hp)) = (npc.hp, npc.max_hp) {
                    if hp < max_hp {
                        let can_regen = match npc.last_hit {
                            Some(t) => now.duration_since(t).as_secs() >= REGEN_COOLDOWN_SECS,
                            None => true,
                        };
                        if can_regen {
                            npc.hp = Some(max_hp);
                            npc.last_hit = None;
                            tracing::info!(event = "npc_regen", npc = %npc.id, hp = max_hp, "npc hp regenerated");
                        }
                    }
                }
            }
        }
    });
}
