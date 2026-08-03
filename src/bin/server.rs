use tokio::net::TcpListener;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::io::AsyncWriteExt;
use tap::{World};
use std::sync::Arc;
use tokio::sync::Mutex;

use tap::commands::connect::handle_connect;
use tap::commands::look::handle_look;
use tap::commands::movement::handle_move;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:4242").await.unwrap();
    println!("server running on {}", listener.local_addr().unwrap());

    let file = std::fs::read_to_string("world.json").unwrap();
    let world: World = serde_json::from_str(&file).unwrap();
    let world: Arc<Mutex<World>> = Arc::new(Mutex::new(world));
    // let world: Arc<Mutex<World>> = Arc::new(Mutex::new(serde_json::from_str(&file).unwrap())); PEUT ETRE REFACTOR COMME CA LES DEUX LIGNES DAVANT mais pas lisible, a voir ce que tu en penses

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        println!("accepted connection from {}", addr);


        let world = Arc::clone(&world);
        tokio::spawn(async move {

            let mut authenticated: Option<String> = None;

            let (r_socket, mut w_socket) = socket.into_split();
            w_socket.write_all(b"OK hello proto=1\n").await.unwrap();
            let mut reader = BufReader::new(r_socket);
            let mut line = String::new();
            while reader.read_line(&mut line).await.unwrap() > 0 {

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
                    w_socket.write_all(res.as_bytes()).await.unwrap();
                    line.clear();

                } else if let Some(name) = &authenticated {
                    if line_upper.starts_with("LOOK") {
                        w_socket.write_all(handle_look(&name, &world).await.as_bytes()).await.unwrap();
                        line.clear();
                    } else if line_upper.starts_with("MOVE ") {
                        let direction = line_upper.strip_prefix("MOVE ").unwrap().trim().to_lowercase();
                        w_socket.write_all(handle_move(&name, &direction, &world).await.as_bytes()).await.unwrap();
                        line.clear();
                    }
                } else {
                    line.clear();
                }
            }

            let mut w = world.lock().await;
            if let Some(name) = &authenticated {
                println!("{} disconnected", &name);
                w.players.remove(name);
                w_socket.write_all(b"OK bye\n").await.unwrap();
            }
        });


    }
}