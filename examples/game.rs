use tokio::net::{TcpListener, TcpStream};
use std::{
    collections::HashMap,
    env,
    io::Error as IoError,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use env_logger::Builder;
use futures::{SinkExt, Stream, StreamExt};
use log::{error, info, LevelFilter};
use thiserror::Error;

#[derive(Error, Debug)]
enum ConnectionHandlerError {
    #[error("WebSocket handshake failed")]
    WebSocketHandshake(#[from] tokio_tungstenite::tungstenite::Error)
}

async fn handle_connection(raw_stream: TcpStream, addr: SocketAddr) -> Result<(), ConnectionHandlerError> {
    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await?;
    let (outgoing, mut incoming) = ws_stream.split();
    while let Some(i) = incoming.next().await {
        dbg!(i);
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    Builder::new().filter(None, LevelFilter::Debug).init();

    let try_socket = TcpListener::bind("0.0.0.0:32145").await;
    let listener = try_socket.expect("Failed to bind");

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection( stream, addr));
    }
}