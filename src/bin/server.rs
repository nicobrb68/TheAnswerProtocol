use tap::commands::group_dispatcher::handle_group;
use tokio::net::TcpListener;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;

use std::sync::Arc;
use std::collections::HashMap;

use tap::{World};

use tap::commands::connect::handle_connect;
use tap::commands::look::handle_look;
use tap::commands::movement::handle_move;
use tap::commands::who::handle_who;
use tap::commands::disconnect::handle_disconnect;
use tap::commands::chat::{handle_chat_global, handle_chat_room, handle_chat_group};
use tap::commands::talk::handle_talk;
use tap::commands::take::handle_take;
use tap::commands::drop::handle_drop;
use tap::commands::inventory::handle_inventory;
use tap::commands::quest::{handle_quest, handle_quests};
use tap::commands::status::handle_status;
use tap::commands::attack::handle_attack;
use tap::commands::sleep::handle_sleep;
use tap::commands::examine::handle_examine;
use tap::utils::{fatal, get_args};

use tap::events::room::notify_room;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("info".parse().expect("invalid filter")))
        .init();

    let port = 7534;
    let addr = format!("0.0.0.0:{}", port);

    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(event = "bind_failed", error = %e, "Failed to bind to {} (port already in use?)", addr);
            eprintln!("Error: Port {} is already in use. Is another server running?", port);
            std::process::exit(1);
        }
    };
    tracing::info!(event = "server_start", addr = %listener.local_addr().expect("Failed to get local addr"), "server started");
    
    let args: Vec<String> = std::env::args().collect();
    let world_path = args.get(1).map(|s| s.as_str()).unwrap_or("src/assets/default_world.json");
    let file = std::fs::read_to_string(world_path).unwrap_or_else(|_| fatal("Failed to load any world file"));
    let world: World = serde_json::from_str(&file).unwrap_or_else(|_| fatal("Failed to properly read world file."));
    if !world.rooms.contains_key(&world.spawn) {
        fatal("Spawn room does not exist in world file");
    }
    if !world.rooms.contains_key(&world.sleep_room) {
        fatal("Sleep room does not exist in world file");
    }
    for (qid, quest) in &world.quests {
        if !world.items.contains_key(&quest.target_item) {
            fatal(&format!("Quest {} references unknown target_item: {}", qid, quest.target_item));
        }
        if !world.items.contains_key(&quest.reward) {
            fatal(&format!("Quest {} references unknown reward: {}", qid, quest.reward));
        }
        if !world.npcs.contains_key(&quest.giver) {
            fatal(&format!("Quest {} references unknown giver: {}", qid, quest.giver));
        }
    }
    let world: Arc<Mutex<World>> = Arc::new(Mutex::new(world));

    let registry: Arc<Mutex<HashMap<String, UnboundedSender<String>>>> = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (socket, addr) = match listener.accept().await {
            Ok(val) => val,
            Err(e) => {
                tracing::error!(event = "accept_failed", error = %e, "failed to accept connection");
                continue
            }
        };
        tracing::info!(event = "connection", ip = %addr, "client connected");


        let world = Arc::clone(&world);
        let registry = Arc::clone(&registry); 
        tokio::spawn(async move {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
            let mut authenticated: Option<String> = None;

            let (r_socket, mut w_socket) = socket.into_split();
            tokio::spawn(async move {
                while let Some(message) = rx.recv().await {
                    if w_socket.write_all(message.as_bytes()).await.is_err() {
                        break;
                    }
                }
            });
            if let Err(e) = tx.send("OK hello proto=1\n".to_string()) {
                tracing::error!(event = "send_failed", ip = %addr, error = %e, "failed to send greeting");
            };

            let mut reader = BufReader::new(r_socket);
            let mut line = String::new();
            let mut flood_window = tokio::time::Instant::now();
            let mut flood_count: u32 = 0;

            loop {
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(e) => {
                        tracing::error!(event = "read_failed", ip = %addr, error = %e, "failed to read client input");
                        break;
                    }
                };
                let line_trimmed = line.trim().to_string();
                let line_upper = line_trimmed.to_uppercase();

                if !line_trimmed.is_empty() {
                    if flood_window.elapsed().as_secs() >= 1 {
                        flood_window = tokio::time::Instant::now();
                        flood_count = 0;
                    }
                    flood_count += 1;
                    if flood_count > 20 {
                        tracing::error!(event = "abuse_kick", ip = %addr, player = ?authenticated, rate = flood_count, "client kicked for flooding");
                        let _ = tx.send(tap::TapError::Flooding.message());
                        break;
                    } else if flood_count > 10 {
                        tracing::warn!(event = "abuse_flood", ip = %addr, player = ?authenticated, rate = flood_count, "command flooding detected");
                    }
                    tracing::info!(event = "command", ip = %addr, player = ?authenticated, command = %line_trimmed, "command received");
                }

                if line_upper.starts_with("CONNECT ") {
                    if authenticated.is_some() {
                        match tx.send("ERR already connected\n".to_string()) {
                            Ok(()) => {},
                            Err(e) => tracing::warn!(event = "send_failed", ip = %addr, error = %e, "failed to send to client")
                        }

                        line.clear();
                        continue;
                    }

                    let username = get_args(&line_trimmed);

                    let res = handle_connect(&username, &world).await;

                    if res.starts_with("OK") {
                        authenticated = Some(username.to_string());
                        registry.lock().await.insert(username.to_string(), tx.clone());
                        let player_room = {
                            let w = world.lock().await;
                            match w.get_player(username) {
                                Some(p) => p.current_room.clone(),
                                None => { line.clear(); continue; }
                            }
                        };
                        notify_room(
                            &player_room,
                            &format!("EVT ROOM PRESENCE ENTER {}\n", username),
                            Some(username),
                            &world,
                            &registry
                        ).await;
                    }

                    match tx.send(res) {
                        Ok(val) => val,
                        Err(e) => {
                            tracing::warn!(event = "send_failed", ip = %addr, error = %e, "failed to send to client");
                            break
                        }
                    };
                    
                    line.clear();

                } else if line_upper.starts_with("QUIT") {
                    if let Err(e) = tx.send("OK bye\n".to_string()) {
                        tracing::warn!(event = "send_failed", ip = %addr, error = %e, "failed to send to client");
                    }
                    break;
                } else if let Some(name) = &authenticated {
                    let res = if line_upper.starts_with("LOOK") {
                        Some(handle_look(&name, &world).await)
                    } else if line_upper.starts_with("MOVE ") {
                        let direction = get_args(&line_trimmed).to_lowercase();
                        Some(handle_move(&name, &direction, &world, &registry).await)
                    } else if line_upper.starts_with("WHO") {
                        Some(handle_who(&world).await)
                    } else if line_upper.starts_with("CHAT ") {
                        let args = get_args(&line_trimmed);
                        let args_upper = args.to_uppercase();
                        if args_upper.starts_with("GLOBAL ") {
                            Some(handle_chat_global(name, &registry, get_args(args)).await)
                        } else if args_upper.starts_with("ROOM ") {
                            Some(handle_chat_room(name, &registry, get_args(args), &world).await)
                        } else if args_upper.starts_with("GROUP ") {
                            Some(handle_chat_group(name, &registry, get_args(args), &world).await)
                        } else {
                            None
                        }
                    } else if line_upper.starts_with("GROUP ") {
                        Some(handle_group(name, get_args(&line_trimmed), &world, &registry).await)
                    } else if line_upper.starts_with("TALK ") {
                        let npc_id = get_args(&line_trimmed).to_lowercase();
                        Some(handle_talk(name, &npc_id, &world).await)
                    } else if line_upper.starts_with("TAKE ") {
                        let item_id = get_args(&line_trimmed).to_lowercase();
                        Some(handle_take(name, &item_id, &world, &registry).await)
                    } else if line_upper.starts_with("DROP ") {
                        let item_id = get_args(&line_trimmed).to_lowercase();
                        Some(handle_drop(name, &item_id, &world, &registry).await)
                    } else if line_upper.starts_with("INVENTORY") {
                        Some(handle_inventory(name, &world).await)
                    } else if line_upper.starts_with("EXAMINE") {
                        let item_id = get_args(&line_trimmed).to_lowercase();
                        Some(handle_examine(name, &item_id, &world).await)
                    }
                     else if line_upper.starts_with("ATTACK ") {
                        let npc_id = get_args(&line_trimmed).to_lowercase();
                        Some(handle_attack(name, &npc_id, &world, &registry).await)
                    } else if line_upper.starts_with("STATUS") {
                        Some(handle_status(name, &world).await)
                    } else if line_upper.starts_with("SLEEP") {
                        Some(handle_sleep(name, &world, &registry).await)
                    } else if line_upper.starts_with("QUESTS") {
                        Some(handle_quests(name, &world).await)
                    } else if line_upper.starts_with("QUEST ") {
                        let npc_id = get_args(&line_trimmed).to_lowercase();
                        Some(handle_quest(name, &npc_id, &world).await)
                    } else {
                        None
                    };

                    if let Some(res) = res {
                        if res.starts_with("ERR") {
                            tracing::warn!(event = "response", ip = %addr, player = %name, response = %res.trim(), "error response sent");
                        } else {
                            tracing::info!(event = "response", ip = %addr, player = %name, response = %res.trim(), "response sent");
                        }
                        match tx.send(res) {
                            Ok(()) => {},
                            Err(e) => {
                                tracing::warn!(event = "send_failed", ip = %addr, error = %e, "failed to send to client");
                                break
                            }
                        }
                    }
                    line.clear();
                } else {
                    line.clear();
                }
            }

            match handle_disconnect(&authenticated, &world, &registry).await {
                Ok(()) => {},
                Err(e) => tracing::error!(event = "disconnect_failed", ip = %addr, error = %e, "failed to disconnect client")
            };

            tracing::info!(event = "disconnection", ip = %addr, "client disconnected");
        });

    }
}