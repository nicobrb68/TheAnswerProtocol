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

use tap::utils::fatal;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:1234").await.expect("An error occured while binding TCP listener");
    println!("Server up, listening on => {}", listener.local_addr().expect("Failed to get local addr"));
    
    let args: Vec<String> = std::env::args().collect();
    let world_path = args.get(1).map(|s| s.as_str()).unwrap_or("src/assets/default_world.json");
    let file = std::fs::read_to_string(world_path).unwrap_or_else(|_| fatal("Failed to load any world file"));
    let world: World = serde_json::from_str(&file).unwrap_or_else(|_| fatal("Failed to properly read world file."));
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
                let line_upper = &line.to_uppercase();

                if line_upper.starts_with("CONNECT ") {
                    if authenticated.is_some() {
                        match tx.send("ERR already connected\n".to_string()) {
                            Ok(()) => {},
                            Err(e) => eprintln!("Failed to write on client side: {}",e)
                        }

                        line.clear();
                        continue;
                    }

                    let username = line_upper.strip_prefix("CONNECT ").unwrap().trim();

                    let res = handle_connect(&username, &world).await;

                    if res.starts_with("OK") {
                        authenticated = Some(username.to_string());
                        registry.lock().await.insert(username.to_string(), tx.clone());
                        let w = world.lock().await;
                        let room_players = w.get_room("room.square").unwrap().players.clone();
                        let reg = registry.lock().await;
                        for player in &room_players {
                            if player != username {
                                if let Some(player_tx) = reg.get(player) {
                                    let _ = player_tx.send(format!("EVT ROOM PRESENCE ENTER {}\n", username));
                                }
                            }
                        }
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
                        let direction = line_upper.strip_prefix("MOVE ").unwrap().trim().to_lowercase();
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
                    } 
                    else {
                        line.clear()
                    }
                } else {
                    line.clear();
                }
            }

            // match handle_disconnect(&authenticated, &world).await {
            //     Ok(()) => {},
            //     Err(e) => eprintln!("Failed to disconnect client: {}", e)
            // };

            
            let mut w = world.lock().await;
            if let Some(name) = &authenticated {
                println!("{} disconnected", &name);
                let room_id = w.get_player(name).unwrap().current_room.clone();
                let room_players = w.get_room(&room_id).unwrap().players.clone();
                let reg = registry.lock().await;
                for player in &room_players {
                    if player != name {
                        if let Some(tx) = reg.get(player) {
                            let _ = tx.send(format!("EVT ROOM PRESENCE LEAVE {}\n", name));
                        }
                    }
                }
                drop(reg);
                w.get_mut_room(&room_id).unwrap().players.retain(|p| p != name);
                w.players.remove(name);
                registry.lock().await.remove(name);
                match tx.send("OK bye\n".to_string()) {
                    Ok(_) => {},
                    Err(e) => eprintln!("Failed to write on client side: {}", e),
                };
            }
            println!("'{}' successfully cut connection", addr);
        });

    }
}