use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <host> <port>", args[0]);
        std::process::exit(1);
    }
    let addr = format!("{}:{}", args[1], args[2]);

    let stream = match TcpStream::connect(&addr).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to connect to {}: {}", addr, e);
            std::process::exit(1);
        }
    };
    eprintln!("Connected to {}", addr);

    let (reader, writer) = stream.into_split();

    // lecture de ce quon recoit du server
    let mut server_reader = BufReader::new(reader);
    let task_recv = tokio::spawn(async move {
        let mut line = String::new();
        loop {
            line.clear();
            match server_reader.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => print!("{}", line),
                Err(_) => break,
            }
        }
        eprintln!("Server disconnected.");
    });

    // envoie de commande au server
    let mut writer = writer;
    let task_send = tokio::spawn(async move {
        let stdin = tokio::io::stdin();
        let mut stdin_reader = BufReader::new(stdin);
        let mut line = String::new();
        loop {
            line.clear();
            match stdin_reader.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    if writer.write_all(line.as_bytes()).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // On attend que l'une des deux taches se termine (QUIT ou deconnexion)
    tokio::select! {
        _ = task_recv => {},
        _ = task_send => {},
    }

}