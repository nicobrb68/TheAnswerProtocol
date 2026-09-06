use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use crate::World;

struct BossConfig {
    id: &'static str,
    room: &'static str,
    hp: u32,
    alert: &'static str,
}

const BOSS_INTERVAL_SECS: u64 = 60;

const BOSSES: &[BossConfig] = &[
    BossConfig {
        id: "npc.dragon",
        room: "room.lair",
        hp: 500,
        alert: "EVT GLOBAL [ALERT] A thunderous roar echoes... The Ancestral Dragon has invaded the Dragon's Lair!\n",
    },
    BossConfig {
        id: "npc.lich",
        room: "room.catacombs",
        hp: 300,
        alert: "EVT GLOBAL [ALERT] A chilling darkness spreads... The Lich King has risen in the Dark Catacombs!\n",
    },
    BossConfig {
        id: "npc.kraken",
        room: "room.beach",
        hp: 400,
        alert: "EVT GLOBAL [ALERT] The sea churns violently... The Kraken has surfaced at Shipwreck Beach!\n",
    },
];

pub fn is_boss(npc_id: &str) -> bool {
    BOSSES.iter().any(|b| b.id == npc_id)
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

            let mut alert: Option<&str> = None;

            {
                let mut w = world.lock().await;

                let any_active = BOSSES.iter().any(|boss| {
                    w.get_room(boss.room)
                        .map(|r| r.npcs.contains(&boss.id.to_string()))
                        .unwrap_or(false)
                });

                if !any_active {
                    let boss = &BOSSES[next % BOSSES.len()];
                    next += 1;

                    if let Some(room) = w.get_mut_room(boss.room) {
                        room.npcs.push(boss.id.to_string());

                        if let Some(npc) = w.get_mut_npc(boss.id) {
                            npc.hp = Some(boss.hp);
                        }

                        alert = Some(boss.alert);
                        tracing::info!(
                            event = "boss_spawn",
                            room = boss.room,
                            boss = boss.id,
                            "world boss spawned"
                        );
                    }
                }
            }

            if let Some(msg) = alert {
                let reg = registry.lock().await;
                for tx in reg.values() {
                    let _ = tx.send(msg.to_string());
                }
            }
        }
    });
}
