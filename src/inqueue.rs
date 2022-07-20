use crate::{Event, FromPlayer};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub struct Inqueue {
    pub(crate) rx: UnboundedReceiver<Event>,
    pub(crate) tx: UnboundedSender<FromPlayer>,
    pub(crate) uid: usize,
}

impl Inqueue {
    pub fn leave(&self) {
        // unwrap coz receiver alive while alive matchmaker
        self.tx.send(FromPlayer::Leave(self.uid)).unwrap();
    }

    // None if event() call when player already kicked
    pub async fn event(&mut self) -> Option<Event> {
        self.rx.recv().await
    }

    pub fn confirm(&self) {
        self.tx.send(FromPlayer::Confirm(self.uid)).unwrap();
    }
}
