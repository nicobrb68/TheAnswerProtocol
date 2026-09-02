use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;

use crate::events::room::notify_room;
use crate::{PlayerState, TapError, World};

pub async fn handle_sleep(
    username: &str,
    world: &Arc<Mutex<World>>,
    registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>,
) -> String {
    let mut w = world.lock().await;

    // 1. Récupération des infos du joueur
    let (room_id, is_dead) = match w.get_player(username) {
        Some(p) => (p.current_room.clone(), matches!(p.status, PlayerState::Dead)),
        None => return TapError::PlayerNotFound.message(),
    };

    if is_dead {
        return TapError::PlayerDead.message();
    }

    // 2. Comparaison directe avec le champ sleep_room du monde
    if room_id != w.sleep_room {
        return TapError::CannotSleepHere.message();
    }

    // 3. Application du soin
    let (new_hp, max_hp) = match w.get_mut_player(username) {
        Some(player) => {
            player.hp = player.max_hp;
            (player.hp, player.max_hp)
        }
        None => return TapError::PlayerNotFound.message(),
    };

    drop(w);

    notify_room(
        &room_id,
        &format!("EVT SLEEP {}\n", username),
        Some(username),
        world,
        registry,
    )
    .await;

    format!("OK hp={}/{} You feel really good now !\n", new_hp, max_hp)
}