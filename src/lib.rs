pub mod inqueue;
mod state;

use crate::inqueue::Inqueue;
use crate::state::State;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{self, Duration, Instant};
use thiserror::Error;

const SHEDULE_DELAY: Duration = Duration::from_millis(100);
const QUORUM: usize = 4;
const TIME_TO_CONFIRM: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub enum KickedReason {
    ConfirmTimeout,
}

#[derive(Debug)]
struct Player {
    tx: UnboundedSender<Event>,
    state: State,
}

impl Player {
    fn new(tx: UnboundedSender<Event>) -> Player {
        Player {
            tx,
            state: State::WaitForQuorum,
        }
    }
}

type Players = Arc<Mutex<HashMap<usize, Player>>>;

#[derive(Error, Debug)]
pub enum JoinErr {
    #[error("duplicate unique id")]
    DuplicateUniqueId,
}

#[derive(Debug)]
enum FromPlayer {
    Leave(usize),
    Confirm(usize),
}

struct Storage {
    last_uid: usize,
    rx_sender: UnboundedSender<FromPlayer>,
    jh: JoinHandle<()>,
    jh2: JoinHandle<()>,
}

impl Storage {
    fn new(
        rx_sender: UnboundedSender<FromPlayer>,
        jh: JoinHandle<()>,
        jh2: JoinHandle<()>,
    ) -> Storage {
        Storage {
            last_uid: 0,
            rx_sender,
            jh,
            jh2,
        }
    }
}

#[derive(Debug)]
pub enum Event {
    ConfirmRequired { timeout: Duration },
    Kicked { reason: KickedReason },
    WaitForQuorum,
    Ready,
}

pub struct Matchmaker {
    players: Players,
    storage: Storage,
}

impl Matchmaker {
    pub fn new() -> Matchmaker {
        let q = Arc::new(Mutex::new(HashMap::new()));
        let from_players = unbounded_channel();

        let jh = tokio::spawn(Matchmaker::schedule(q.clone()));
        let jh2 = tokio::spawn(Matchmaker::process_from_players(q.clone(), from_players.1));

        let storage = Storage::new(from_players.0, jh, jh2);

        Matchmaker {
            players: q,
            storage,
        }
    }

    pub async fn join(&mut self) -> Result<Inqueue, JoinErr> {
        let mut l = self.players.lock().await;

        let uid = self.storage.last_uid;

        if l.contains_key(&uid) {
            return Err(JoinErr::DuplicateUniqueId);
        }

        let to_player = unbounded_channel();

        let inqueue = Inqueue {
            rx: to_player.1,
            tx: self.storage.rx_sender.clone(),
            uid,
        };

        let p = Player::new(to_player.0);
        l.insert(uid, p);

        self.storage.last_uid = uid.wrapping_add(1);
        Ok(inqueue)
    }

    async fn schedule(queue: Players) {
        let mut interval = time::interval(SHEDULE_DELAY);
        loop {
            {
                let mut l = queue.lock().await;

                // garbage collecting (delete who removed)
                l.retain(|_, i| !i.state.is_to_remove());

                let now = Instant::now();

                // send ConfirmRequired to quorum's
                let mut waiters = l
                    .values_mut()
                    .filter(|i| i.state.is_wait_for_quorum())
                    .collect::<Vec<_>>();

                waiters.chunks_exact_mut(QUORUM).for_each(|chunk| {
                    for i in chunk {
                        let cr = Event::ConfirmRequired {
                            timeout: TIME_TO_CONFIRM,
                        };
                        match i.tx.send(cr) {
                            Ok(_) => i.state.wait_confirm(now),
                            Err(_) => i.state.to_remove(),
                        }
                    }
                });

                // remove from queue who timeout and not confirmed
                let failed = l
                    .values_mut()
                    .filter(|i| i.state.is_confirm_failed(now, TIME_TO_CONFIRM));
                for i in failed {
                    i.state.to_remove();
                    let k = Event::Kicked {
                        reason: KickedReason::ConfirmTimeout,
                    };
                    #[allow(unused_must_use)]
                    {
                        i.tx.send(k);
                    }
                }

                // return to matchmaking who timeout but confirmed
                let timeout = l
                    .values_mut()
                    .filter(|i| i.state.is_confirm_timeout(now, TIME_TO_CONFIRM));
                for i in timeout {
                    match i.tx.send(Event::WaitForQuorum) {
                        Ok(_) => i.state.wait_for_quorum(),
                        Err(_) => i.state.to_remove(),
                    }
                }

                // ready to game
                let mut ready = l
                    .values_mut()
                    .filter(|i| i.state.is_confirm_success(now, TIME_TO_CONFIRM))
                    .collect::<Vec<_>>();
                ready.chunks_exact_mut(QUORUM).for_each(|chunk| {
                    for i in chunk {
                        #[allow(unused_must_use)]
                        {
                            i.tx.send(Event::Ready);
                        }
                        i.state.to_remove()
                    }
                });
            }
            interval.tick().await;
        }
    }

    async fn process_from_players(players: Players, mut rx: UnboundedReceiver<FromPlayer>) {
        while let Some(from_player) = rx.recv().await {
            {
                let mut l = players.lock().await;

                match from_player {
                    FromPlayer::Leave(uid) => {
                        if let Some(player) = l.get_mut(&uid) {
                            player.state.to_remove();
                        }
                    }
                    FromPlayer::Confirm(uid) => {
                        if let Some(player) = l.get_mut(&uid) {
                            player.state.wait_confirm_if_actual(TIME_TO_CONFIRM)
                        }
                    }
                }
            }
        }
    }
}

impl Drop for Matchmaker {
    fn drop(&mut self) {
        self.storage.jh.abort();
        self.storage.jh2.abort();
    }
}
