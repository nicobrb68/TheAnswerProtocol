use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tower_http::services::ServeDir;

#[derive(Deserialize)]
struct ConnectParams {
    host: String,
    port: u16,
    username: String,
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .fallback_service(ServeDir::new("static"));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("TAP web client running on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind port 3000");

    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(socket: WebSocket) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    let connect_raw = match ws_rx.next().await {
        Some(Ok(Message::Text(txt))) => txt,
        _ => return,
    };

    let params: ConnectParams = match serde_json::from_str(connect_raw.as_str()) {
        Ok(p) => p,
        Err(e) => {
            let _ = ws_tx
                .send(Message::Text(format!("ERR connection failed: invalid connect message ({e})").into()))
                .await;
            return;
        }
    };

    let addr = format!("{}:{}", params.host, params.port);
    let stream = match tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(&addr)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            let _ = ws_tx
                .send(Message::Text(format!("ERR connection failed: {e}").into()))
                .await;
            return;
        }
        Err(_) => {
            let _ = ws_tx
                .send(Message::Text("ERR connection failed: timeout after 5s".into()))
                .await;
            return;
        }
    };

    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);

    let connect_cmd = format!("CONNECT {}\n", params.username);
    if writer.write_all(connect_cmd.as_bytes()).await.is_err() {
        let _ = ws_tx
            .send(Message::Text("ERR connection failed: failed to send CONNECT".into()))
            .await;
        return;
    }

    let read_task = tokio::spawn(async move {
        let mut line = String::new();
        loop {
            line.clear();
            match buf_reader.read_line(&mut line).await {
                Ok(0) => {
                    let _ = ws_tx
                        .send(Message::Text("EVT DISCONNECTED server closed connection".into()))
                        .await;
                    break;
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if !trimmed.is_empty()
                        && ws_tx.send(Message::Text(trimmed.to_string().into())).await.is_err()
                    {
                        break;
                    }
                }
                Err(e) => {
                    let _ = ws_tx
                        .send(Message::Text(format!("EVT DISCONNECTED read error: {e}").into()))
                        .await;
                    break;
                }
            }
        }
    });

    let write_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            if let Message::Text(cmd) = msg {
                let line = format!("{}\n", cmd);
                if writer.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
            }
        }
    });

    let _ = tokio::join!(read_task, write_task);
}