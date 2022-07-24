use env_logger::Builder;
use futures::stream::{select, select_all, SelectAll, SplitStream};
use futures::{SinkExt, Stream, StreamExt};
use log::{debug, error, info, LevelFilter};
use std::pin::Pin;
use std::{collections::HashMap, env, io::Error as IoError, net::SocketAddr, sync::Arc};
use thiserror::Error;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{tungstenite, WebSocketStream};
use yamm::inqueue::{InqueueReceiver, InqueueSender};
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

async fn top_lvl_handle(tcp_stream: TcpStream, mm: Arc<Mutex<Matchmaker>>) {
    #[derive(Debug)]
    enum NetMM {
        Net(Result<tungstenite::Message, tungstenite::Error>),
        Event(Event),
    }

    enum State {
        Idle,
        Inqueue(InqueueSender),
        Ingame,
    }

    impl State {
        fn is_idle(&self) -> bool {
            match self {
                Self::Idle => true,
                _ => false,
            }
        }

        fn to_inqueue(&mut self, iq: InqueueSender) {
            *self = State::Inqueue(iq);
        }
    }

    let ws_stream = tokio_tungstenite::accept_async(tcp_stream).await.unwrap();
    let (net_tx, mut net_rx) = ws_stream.split();

    let mut net_rx_map = net_rx.map(|i| NetMM::Net(i));

    let mut select: SelectAll<Pin<Box<dyn Stream<Item = NetMM> + Send>>> = SelectAll::new();

    select.push(Box::pin(net_rx_map));

    let mut state = State::Idle;

    while let Some(pdu) = select.next().await {
        debug!("msg: {:?}, select count: {}", &pdu, select.len());
        match pdu {
            NetMM::Net(net) => match net {
                Ok(m) => {
                    if m == Message::text("queue".to_string()) && state.is_idle() {
                        if let Ok(inqueue) = mm.lock().await.join().await {
                            let (inqueue_tx, inqueue_rx) = inqueue.split();
                            let inqueue_rx_map = inqueue_rx.map(|i| NetMM::Event(i));
                            select.push(Box::pin(inqueue_rx_map));
                            state.to_inqueue(inqueue_tx);
                        }
                    }
                }
                Err(e) => error!("{}", e),
            },
            NetMM::Event(_) => (),
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
