use tokio::net::TcpListener;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:4242").await.unwrap();
    println!("server running on {}", listener.local_addr().unwrap());


    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        println!("accepted connection from {}", addr);



        tokio::spawn(async move {
            let (r_socket, mut w_socket) = socket.into_split();
            w_socket.write_all(b"OK hello proto=1\n").await.unwrap();
            let mut reader = BufReader::new(r_socket);
            let mut line = String::new();
            while reader.read_line(&mut line).await.unwrap() > 0 {
                println!("Recu : {}", line.trim());
                line.clear()
            }
            println!("connection closed: {}", addr);
        });


    }
}