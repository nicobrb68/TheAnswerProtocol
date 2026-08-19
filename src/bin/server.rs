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
use tap::utils::{fatal, get_args};

use tap::events::room::notify_room;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("0.0.0.0:7534").await.expect("An error occured while binding TCP listener");
    println!("Server up, listening on => {}", listener.local_addr().expect("Failed to get local addr"));
    
    let args: Vec<String> = std::env::args().collect();
    let world_path = args.get(1).map(|s| s.as_str()).unwrap_or("src/assets/default_world.json");
    let file = std::fs::read_to_string(world_path).unwrap_or_else(|_| fatal("Failed to load any world file"));
    let world: World = serde_json::from_str(&file).unwrap_or_else(|_| fatal("Failed to properly read world file."));
    if !world.rooms.contains_key(&world.spawn) {
        fatal("Spawn room does not exist in world file");
    }
    let world: Arc<Mutex<World>> = Arc::new(Mutex::new(world));

    let registry: Arc<Mutex<HashMap<String, UnboundedSender<String>>>> = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (socket, addr) = match listener.accept().await {
            Ok(val) => val,
            Err(e) => {
                eprintln!("Failed to accept connection: {}", e);
                continue
            }
        };
        println!("Established connection with '{}'", addr);


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
                eprintln!("Failed to write message for {} : {}", addr, e)
            };

            let mut reader = BufReader::new(r_socket);
            let mut line = String::new();

            loop {
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("Failed to read user's input: {}", e);
                        break;
                    }
                };
                let line_trimmed = line.trim().to_string();
                let line_upper = line_trimmed.to_uppercase();

                if line_upper.starts_with("CONNECT ") {
                    if authenticated.is_some() {
                        match tx.send("ERR already connected\n".to_string()) {
                            Ok(()) => {},
                            Err(e) => eprintln!("Failed to write on client side: {}",e)
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
                            eprintln!("Failed to write on client side: {}", e);
                            break
                        }
                    };
                    
                    line.clear();

                } else if line_upper.starts_with("QUIT") {
                    if let Err(e) = tx.send("OK bye\n".to_string()) {
                        eprintln!("Failed to write on client side: {}", e);
                    }
                    break;
                } else if let Some(name) = &authenticated {
                    if line_upper.starts_with("LOOK") {
                        match tx.send(handle_look(&name, &world).await) {
                            Ok(val) => val,
                            Err(e) => {
                                eprintln!("Failed to write on client side: {}", e);
                                break
                            }
                        };
                        line.clear();
                    } else if line_upper.starts_with("MOVE ") {
                        let direction = get_args(&line_trimmed).to_lowercase();
                        match tx.send(handle_move(&name, &direction, &world, &registry).await) {
                            Ok(val) => val,
                            Err(e) => {
                                eprintln!("Failed to write on client side: {}", e);
                                break
                            }
                        };
                        line.clear()
                    } else if line_upper.starts_with("WHO") {
                        match tx.send(handle_who( &world).await) {
                            Ok(val) => val,
                            Err(e) => {
                                eprintln!("Failed to write on client side: {}", e);
                                break
                            }
                        };
                        line.clear()
                    } else if line_upper.starts_with("CHAT ") {
                        let args = get_args(&line_trimmed);
                        let args_upper = args.to_uppercase();
                        if args_upper.starts_with("GLOBAL ") {
                            let message = get_args(args);
                            let res = handle_chat_global(name, &registry, message).await;
                            let _ = tx.send(res);
                        } else if args_upper.starts_with("ROOM ") {
                            let message = get_args(args);
                            let res = handle_chat_room(name, &registry, message, &world).await;
                            let _ = tx.send(res);
                        } else if args_upper.starts_with("GROUP ") {
                            let message = get_args(args);
                            let res = handle_chat_group(name, &registry, message, &world).await;
                            let _ = tx.send(res);
                        }
                        line.clear();
                    } else if line_upper.starts_with("GROUP ") {
                        let res = handle_group(name, get_args(&line_trimmed), &world, &registry).await;
                        let _ = tx.send(res);
                        line.clear();
                    } else if line_upper.starts_with("TALK ") {
                        let npc_id = get_args(&line_trimmed).to_lowercase();
                        let res = handle_talk(name, &npc_id, &world).await;
                        let _ = tx.send(res);
                        line.clear();
                    } else if line_upper.starts_with("TAKE ") {
                        let item_id = get_args(&line_trimmed).to_lowercase();
                        let res = handle_take(name, &item_id, &world).await;
                        let _ = tx.send(res);
                        line.clear();
                    } else if line_upper.starts_with("DROP ") {
                        let item_id = get_args(&line_trimmed).to_lowercase();
                        let res = handle_drop(name, &item_id, &world).await;
                        let _ = tx.send(res);
                        line.clear();
                    } else if line_upper.starts_with("INVENTORY") {
                        let res = handle_inventory(name, &world).await;
                        let _ = tx.send(res);
                        line.clear();
                    } else if line_upper.starts_with("ATTACK ") {
                        let npc_id = get_args(&line_trimmed).to_lowercase();
                        let res = handle_attack(name, &npc_id, &world).await;
                        let _ = tx.send(res);
                        line.clear();
                    } else if line_upper.starts_with("STATUS") {
                        let res = handle_status(name, &world).await;
                        let _ = tx.send(res);
                        line.clear();
                    } else if line_upper.starts_with("QUESTS") {
                        let res = handle_quests(name, &world).await;
                        let _ = tx.send(res);
                        line.clear();
                    } else if line_upper.starts_with("QUEST ") {
                        let npc_id = get_args(&line_trimmed).to_lowercase();
                        let res = handle_quest(name, &npc_id ,&world).await;
                        let _ = tx.send(res);
                        line.clear();
                    }
                    else {
                        line.clear()
                    }
                } else {
                    line.clear();
                }
            }

            match handle_disconnect(&authenticated, &world, &registry).await {
                Ok(()) => {},
                Err(e) => eprintln!("Failed to disconnect client: {}", e)
            };

            println!("'{}' successfully cut connection", addr);
        });

    }
}