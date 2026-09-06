use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use crate::World;

const REGEN_INTERVAL_SECS: u64 = 60;

pub fn start_npc_regen(world: Arc<Mutex<World>>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(REGEN_INTERVAL_SECS));
        interval.tick().await;

        loop {
            interval.tick().await;
            let mut w = world.lock().await;
            for npc in w.npcs.values_mut() {
                if let (Some(hp), Some(max_hp)) = (npc.hp, npc.max_hp) {
                    if hp < max_hp {
                        npc.hp = Some(max_hp);
                        tracing::info!(event = "npc_regen", npc = %npc.id, hp = max_hp, "npc hp regenerated");
                    }
                }
            }
        }
    });
}
