use std::sync::Arc;
use tokio::sync::Mutex;
use crate::World;

pub async fn handle_map(world: &Arc<Mutex<World>>) -> String {
    let w = world.lock().await;
    let mut rooms = serde_json::Map::new();
    for (id, room) in &w.rooms {
        let mut obj = serde_json::Map::new();
        obj.insert("name".into(), serde_json::Value::String(room.name.clone()));
        let exits: serde_json::Map<String, serde_json::Value> = room.exits.iter()
            .map(|(dir, target)| (dir.clone(), serde_json::Value::String(target.clone())))
            .collect();
        obj.insert("exits".into(), serde_json::Value::Object(exits));
        if let Some(x) = room.map_x {
            obj.insert("map_x".into(), serde_json::Value::Number(x.into()));
        }
        if let Some(y) = room.map_y {
            obj.insert("map_y".into(), serde_json::Value::Number(y.into()));
        }
        rooms.insert(id.clone(), serde_json::Value::Object(obj));
    }
    let mut result = serde_json::Map::new();
    result.insert("rooms".into(), serde_json::Value::Object(rooms));
    result.insert("spawn".into(), serde_json::Value::String(w.spawn.clone()));
    format!("OK {}\n", serde_json::Value::Object(result))
}
