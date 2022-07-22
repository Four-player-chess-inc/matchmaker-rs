use env_logger::Builder;
use futures::stream::SplitStream;
use futures::{SinkExt, Stream, StreamExt};
use log::{error, info, LevelFilter};
use std::{collections::HashMap, env, io::Error as IoError, net::SocketAddr, sync::Arc};
use thiserror::Error;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use yamm::inqueue::InqueueReceiver;
use yamm::{Event, Matchmaker};

#[derive(Error, Debug)]
enum ConnectionHandlerError {
    #[error("WebSocket handshake failed")]
    WebSocketHandshake(#[from] tokio_tungstenite::tungstenite::Error),
}

#[derive(Debug)]
enum Chan {
    Net(Message),
    MatchM(Event),
}

async fn handle_connection(
    mut net_rx: SplitStream<WebSocketStream<TcpStream>>,
    to_processor: UnboundedSender<Chan>,
) {
    while let Some(p) = net_rx.next().await {
        /*match p {
            Ok(p) => chan_tx.send(Chan::Net(p)).unwrap(),
            Err(e) => error!("{}", e)
        }*/
    }
}

async fn handle_matchmaking(mut inqueue_rx: InqueueReceiver, to_processor: UnboundedSender<Chan>) {
    while let Some(e) = inqueue_rx.next().await {}
}

async fn top_lvl_handle(tcp_stream: TcpStream, mm: Arc<Mutex<Matchmaker>>) {
    let ws_stream = tokio_tungstenite::accept_async(tcp_stream).await.unwrap();
    let (net_tx, net_rx) = ws_stream.split();

    let (from_handlers_tx, mut from_handlers_rx) = unbounded_channel();

    tokio::spawn(handle_connection(net_rx, from_handlers_tx.clone()));

    while let Some(pdu) = from_handlers_rx.recv().await {
        match pdu {
            Chan::Net(msg) => {
                if msg == Message::text("startqueue".to_string()) {
                    let inqueue = mm.lock().await.join().await.unwrap();
                    let (inqueue_tx, inqueue_rx) = inqueue.split();
                    tokio::spawn(handle_matchmaking(inqueue_rx, from_handlers_tx.clone()));
                }
            }
            Chan::MatchM(event) => (),
        }
    }
}

#[tokio::main]
async fn main() {
    Builder::new().filter(None, LevelFilter::Debug).init();

    let try_socket = TcpListener::bind("0.0.0.0:32145").await;
    let listener = try_socket.expect("Failed to bind");

    let mm = Arc::new(Mutex::new(Matchmaker::new()));

    while let Ok((tcp_stream, addr)) = listener.accept().await {
        tokio::spawn(top_lvl_handle(tcp_stream, mm.clone()));
    }
}
