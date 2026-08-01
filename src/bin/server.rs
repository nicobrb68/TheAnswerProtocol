use tokio::net::TcpListener;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::io::AsyncWriteExt;
use tap::World;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:4242").await.unwrap();
    println!("server running on {}", listener.local_addr().unwrap());

    let file = std::fs::read_to_string("world.json").unwrap();
    let world: World = serde_json::from_str(&file).unwrap();
    println!("world: {:?}", world);

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        println!("accepted connection from {}", addr);



        tokio::spawn(async move {
            let (r_socket, mut w_socket) = socket.into_split();
            w_socket.write_all(b"OK hello proto=1\n").await.unwrap();
            let mut reader = BufReader::new(r_socket);
            let mut line = String::new();
            while reader.read_line(&mut line).await.unwrap() > 0 {

                if line.starts_with("CONNECT ") {
                    let username = line.strip_prefix("CONNECT ").unwrap().trim();
                    let msg = format!("CONNECT recu de {}", username);
                    println!("{}", msg);
                    w_socket.write(b"OK connected\n").await.unwrap();
                    line.clear();
                } else {
                    line.clear();
                }
            }
            println!("connection closed: {}", addr);
        });


    }
}