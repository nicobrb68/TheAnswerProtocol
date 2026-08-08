use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{World,
            commands::group::create::create_group,
            commands::group::info::info_group,
            utils::get_args
			};


pub async fn handle_group(
	username: &str,
	args: &str,
	world: &Arc<Mutex<World>>,
) -> String {
	let args_upper = args.to_uppercase();
	let group_args = get_args(args);

	if args_upper.starts_with("CREATE") {
		return create_group(username, &world, group_args).await;
	} else if args_upper.starts_with("INFO") {
		return info_group(username, &world).await;
	}


	"nico ta grosse mere".to_string()
}