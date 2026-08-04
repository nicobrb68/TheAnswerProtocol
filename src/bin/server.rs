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
            if let Err(e) = w_socket.write_all(b"OK hello proto=1\n").await {
                eprintln!("Failed to write for message for {}", addr)
            };

            let mut reader = BufReader::new(r_socket);
            let mut line = String::new();

            loop {
                let bytes_read = match reader.read_line(&mut line).await {
                    Ok(0) => break,          // EOF propre : le client a fermé la connexion
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("Failed to read user's input: {}", e);
                        break;
                    }
                };
                            // let line_trimmed = &line.trim().to_string();
                let line_upper = &line.to_uppercase();

                if line_upper.starts_with("CONNECT ") {
                    if authenticated.is_some() {
                        w_socket.write_all(b"ERR already connected\n").await.unwrap();
                        line.clear();
                        continue;
                    }

                    let username = line_upper.strip_prefix("CONNECT ").unwrap().trim();

                    let res = handle_connect(&username, &world).await;

                    if res.starts_with("OK") {
                        authenticated = Some(username.to_string());
                    }

                    match w_socket.write_all(res.as_bytes()).await {
                        Ok(val) => val,
                        Err(e) => {
                            eprintln!("Failed to write on client side: {}", e);
                            break
                        }
                    };
                    
                    line.clear();

                } else if let Some(name) = &authenticated {
                    if line_upper.starts_with("LOOK") {
                        match w_socket.write_all(handle_look(&name, &world).await.as_bytes()).await {
                            Ok(val) => val,
                            Err(e) => {
                                eprintln!("Failed to write on client side: {}", e);
                                break
                            }
                        };
                        line.clear();
                    } else if line_upper.starts_with("MOVE ") {
                        let direction = line_upper.strip_prefix("MOVE ").unwrap().trim().to_lowercase();
                        match w_socket.write_all(handle_move(&name, &direction, &world, &registry).await.as_bytes()).await  {
                            Ok(val) => val,
                            Err(e) => {
                                eprintln!("Failed to write on client side: {}", e);
                                break
                            }
                        };
                        line.clear()
                    } else {
                        line.clear()
                    }
                } else {
                    line.clear();
                }
            }

            let mut w = world.lock().await;
            if let Some(name) = &authenticated {
                println!("{} disconnected", &name);
                w.players.remove(name);
                match w_socket.write_all(b"OK bye\n").await {
                    Ok(val) => val,
                    Err(e) => {
                        eprintln!("Failed to write on client side: {}", e);
                    }
                };
            }
            println!("'{}' successfully cut connection", addr);
        });

    }
}