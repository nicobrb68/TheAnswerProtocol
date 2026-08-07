use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{World, commands::group::create::create_group, utils::get_args};


pub async fn handle_group(
	username: &str,
	args: &str,
	world: &Arc<Mutex<World>>,
) -> String {
	let args_upper = args.to_uppercase();

	if args_upper.starts_with("CREATE") {
		let group_args = get_args(args);
    	return create_group(username, &world, group_args).await;
	}




	"nico ta grosse mere".to_string()
}