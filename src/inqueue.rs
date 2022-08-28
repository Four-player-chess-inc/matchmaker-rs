use crate::{Event, FromPlayer};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;

pub struct Inqueue {
    pub(crate) rx: UnboundedReceiver<Event>,
    pub(crate) tx: UnboundedSender<FromPlayer>,
    pub(crate) uid: usize,
}

pub struct InqueueReceiver {
    pub(crate) rx: UnboundedReceiver<Event>,
}

impl InqueueReceiver {
    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}

pub struct InqueueSender {
    pub(crate) tx: UnboundedSender<FromPlayer>,
    pub(crate) uid: usize,
}

impl InqueueSender {
    pub fn leave(&self) {
        #[allow(unused_must_use)]
        {
            self.tx.send(FromPlayer::Leave(self.uid));
        }
    }
    pub fn confirm(&self) {
        #[allow(unused_must_use)]
        {
            self.tx.send(FromPlayer::Confirm(self.uid));
        }
    }
}

impl Inqueue {
    pub fn leave(&self) {
        // unwrap coz receiver alive while alive matchmaker
        self.tx.send(FromPlayer::Leave(self.uid)).unwrap();
    }

    // None if event() call when player already kicked
    pub async fn next_event(&mut self) -> Option<Event> {
        self.rx.recv().await
    }

    pub fn confirm(&self) {
        self.tx.send(FromPlayer::Confirm(self.uid)).unwrap();
    }

    pub fn split(self) -> (InqueueSender, UnboundedReceiverStream<Event>) {
        (
            InqueueSender {
                tx: self.tx,
                uid: self.uid,
            },
            UnboundedReceiverStream::new(self.rx),
        )
    }
}
