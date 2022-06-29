use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use yamm::{Matchmaker, Notify, Queuer};

struct Player {
    mm_notify: (UnboundedSender<Notify<usize>>, UnboundedReceiver<Notify<usize>>),
    uid: usize,
}

impl Player {
    fn new(uid: usize) -> Player {
        Player {
            mm_notify: unbounded_channel(),
            uid,
        }
    }
    async fn wait_game(&mut self) {
        self.mm_notify.1.recv().await;
    }
}

impl Queuer for Player {
    type Uid = usize;

    fn get_sender(&self) -> UnboundedSender<Notify<Self::Uid>> {
        self.mm_notify.0.clone()
    }

    fn get_unique_id(&self) -> Self::Uid {
        self.uid
    }
}

#[tokio::main]
async fn main() {
    let mut p = Player::new(1);

    let mut mm = Matchmaker2::new();

    let process = mm.join().await;
    //process.leave();
    /*match process.game().await {
        Kick =>,
        Ready =>
    }*/
}
