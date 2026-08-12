use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;
use crate::{
	World,
	commands::group::create::create_group,
	commands::group::info::info_group,
	commands::group::invite::invite_group,
	commands::group::join::join_group,
	commands::group::leave::leave_group,
	utils::get_args,
	TapError
};


pub async fn handle_group(
	username: &str,
	args: &str,
	world: &Arc<Mutex<World>>,
	registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>
) -> String {
	let args_upper = args.to_uppercase();
	let group_args = get_args(args);

	if args_upper.starts_with("CREATE") {
		return create_group(username, &world, group_args).await;
	} else if args_upper.starts_with("JOIN") {
		return join_group(username, group_args, &world, &registry).await;
	} else {
		let group_id = {
			let w = world.lock().await;
			match w.get_player(username) {
				Some(p) => match &p.group_id {
					Some(id) => id.clone(),
					None => return TapError::NotInGroup.message()
				}
				None => return TapError::PlayerNotFound.message()
			}
		};

		if args_upper.starts_with("INFO") {
			return info_group(&group_id, &world).await;
		} else if args_upper.starts_with("INVITE") {
			return invite_group(username, &group_id, &group_args, &world, &registry).await;
		} else if args_upper.starts_with("LEAVE") {
			return leave_group(username, &group_id, &world, &registry).await;
		}
	}


	"nico ta grosse mere\n".to_string()
}