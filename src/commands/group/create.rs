use crate::{Group, TapError, World, utils::is_valid_group_name};
use tokio::sync::Mutex;
use std::sync::Arc;

pub async fn create_group(
	username: &str,
	world: &Arc<Mutex<World>>,
	group_name: &str
) -> String {
	let group_id = if group_name.is_empty() {
		username.to_string()
	} else {
		if !is_valid_group_name(group_name) {
			return TapError::InvalidGroupName.message();
		}
		let w = world.lock().await;
		if w.has_group(group_name) {
			return TapError::GroupNameInUse.message();
		}
		drop(w);
		group_name.to_string()
	};
	
	let group = Group::new(group_id.clone(), username.to_string());
	let mut w = world.lock().await;
	w.add_group(group);
	
	format!("OK group={}\n", group_id)
}