use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use std::error::Error;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::{Sender, channel};
use tokio_websockets::{Message, ServerBuilder, WebSocketStream};

async fn handle_connection(
    addr: SocketAddr,
    mut ws_stream: WebSocketStream<TcpStream>,
    bcast_tx: Sender<(SocketAddr, String)>, // Menggunakan tuple agar tahu siapa pengirimnya
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut bcast_rx = bcast_tx.subscribe();

    loop {
        tokio::select! {
            // 1. Menerima pesan dari Client melalui WebSocket
            incoming = ws_stream.next() => {
                match incoming {
                    Some(Ok(msg)) => {
                        if let Some(text) = msg.as_text() {
                            // Kirim teks beserta alamat (addr) pengirim ke channel broadcast
                            let _ = bcast_tx.send((addr, text.to_string()));
                        }
                    }
                    // Putuskan loop jika koneksi ditutup atau error
                    _ => break,
                }
            }

            // 2. Menerima pesan dari Broadcast Channel
            msg = bcast_rx.recv() => {
                match msg {
                    Ok((sender_addr, text)) => {
                        // HANYA kirim ke client jika pengirimnya BUKAN dirinya sendiri
                        if sender_addr != addr {
                            ws_stream.send(Message::text(text)).await?;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (bcast_tx, _) = channel::<(SocketAddr, String)>(16);

    let listener = TcpListener::bind("127.0.0.1:2000").await?;
    println!("listening on port 2000");

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("New connection from {addr:?}");
        let bcast_tx = bcast_tx.clone();
        tokio::spawn(async move {
            // Wrap the raw TCP stream into a websocket.
            let (_req, ws_stream) = ServerBuilder::new().accept(socket).await?;

            handle_connection(addr, ws_stream, bcast_tx).await
        });
    }
}