use tokio::net::TcpListener;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::io::AsyncWriteExt;
use tap::{Player, PlayerState, World};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:4242").await.unwrap();
    println!("server running on {}", listener.local_addr().unwrap());

    let file = std::fs::read_to_string("world.json").unwrap();
    let world: World = serde_json::from_str(&file).unwrap();
    println!("world: {:?}", world);

    let world: Arc<Mutex<World>> = Arc::new(Mutex::new(world));

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

                if line.starts_with("CONNECT ") {
                    if authenticated.is_some() {
                        w_socket.write_all(b"ERR already connected\n").await.unwrap();
                        line.clear();
                        continue;
                    }

                    let username = line.strip_prefix("CONNECT ").unwrap().trim();
                    println!("Connection demand for {}", username);

                    let mut w = world.lock().await;
                    if w.players.contains_key(username) {
                        w_socket.write(b"ERR 201 NAME_IN_USE\n").await.unwrap();
                    } else {
                        let mut user = Player {
                            name: username.to_string(),
                            inventory: Vec::new(),
                            quests_active: Vec::new(),
                            quests_done: Vec::new(),
                            hp: 100,
                            status: PlayerState::Alive,
                            current_room: "room.square".to_string(),
                            group_id: None,
                        };
                        w.players.insert(username.to_string(), user);
                        authenticated = Some(username.to_string());
                        w_socket.write(b"OK connected\n").await.unwrap();
                    }
                    line.clear();
                    println!("{:?}", w.players);
                } else {
                    line.clear();
                }
            }
            println!("connection closed: {}", addr);
        });


    }
}