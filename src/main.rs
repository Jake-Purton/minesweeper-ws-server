mod game_logic;

use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info};
use tracing_subscriber;

use game_logic::generate_mines_vec;

struct Client {
    sender: futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>, Message>,
    receiver: futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Server is starting...");
    let listener = TcpListener::bind("127.0.0.1:9001").await.unwrap();
    let clients = Arc::new(Mutex::new(Vec::new()));
    info!("Server listening on 127.0.0.1:9001");

    while let Ok((stream, _)) = listener.accept().await {
        let clients = clients.clone();
        tokio::spawn(async move {
            info!("New client connection accepted");
            let ws_stream = match accept_async(stream).await {
                Ok(ws) => ws,
                Err(e) => {
                    error!("Failed to upgrade to WebSocket: {:?}", e);
                    return;
                }
            };
            let (sender, receiver) = ws_stream.split();
            let client = Client { sender, receiver };

            {
                let mut clients_locked = clients.lock().await;
                clients_locked.push(client);
                info!("Client added. Total clients: {}", clients_locked.len());

                // If two clients are connected, start a game task
                if clients_locked.len() == 2 {
                    let mut pair = clients_locked.split_off(0); // take the two clients
                    let a = pair.remove(0);
                    let b = pair.remove(0);
                    tokio::spawn(run_pair(a, b));
                }
            }
        });
    }
}

async fn run_pair(
    mut a: Client,
    mut b: Client,
) {
    info!("Starting game with two clients");

    // Initial broadcast
    let mut mines = generate_mines_vec();
    mines.append(&mut generate_mines_vec());

    if a.sender.send(Message::Text("hello".to_string())).await.is_ok() {
        let mut m = mines.clone();
        m.push(0);
        let _ = a.sender.send(Message::Binary(m)).await;
    }

    if b.sender.send(Message::Text("hello".to_string())).await.is_ok() {
        let mut m = mines.clone();
        m.push(1);
        let _ = b.sender.send(Message::Binary(m)).await;
    }

    // Forward messages between clients
    let (mut a_sender, mut a_receiver) = (a.sender, a.receiver);
    let (mut b_sender, mut b_receiver) = (b.sender, b.receiver);

    let forward_a_to_b = async {
        while let Some(msg) = a_receiver.next().await {
            match msg {
                Ok(m) => {
                    if b_sender.send(m).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    error!("Error from client A: {:?}", e);
                    break;
                }
            }
        }
    };

    let forward_b_to_a = async {
        while let Some(msg) = b_receiver.next().await {
            match msg {
                Ok(m) => {
                    if a_sender.send(m).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    error!("Error from client B: {:?}", e);
                    break;
                }
            }
        }
    };

    tokio::select! {
        _ = forward_a_to_b => info!("A disconnected, closing pair"),
        _ = forward_b_to_a => info!("B disconnected, closing pair"),
    }
}
