use futures::future::join_all;
use std::sync::Arc;
use tokio::sync::Mutex;
use yamm::{Event, Matchmaker};

async fn test(mm: Arc<Mutex<Matchmaker>>, uid: usize) {
    let mut inqueue = mm.lock().await.join().await.unwrap();
    //println!("locked");
    while let Some(e) = inqueue.event().await {
        match e {
            Event::WaitForQuorum => println!("-> wait form quorum"),
            Event::Kicked { reason } => println!("-> kicked {:?}", reason),
            Event::ConfirmRequired { timeout } => {
                println!("-> comfirm requered {:?}", timeout);
                if uid != 3 {
                    inqueue.confirm();
                }
            }
            Event::Ready => println!("ready!"),
        }
    }
}

#[tokio::main]
async fn main() {
    {
        let mm = Arc::new(Mutex::new(Matchmaker::new()));

        join_all(vec![
            tokio::spawn(test(mm.clone(), 0)),
            tokio::spawn(test(mm.clone(), 1)),
            tokio::spawn(test(mm.clone(), 2)),
            tokio::spawn(test(mm.clone(), 3)),
        ])
        .await;
    }

    let mut line = String::new();
    let _input = std::io::stdin()
        .read_line(&mut line)
        .expect("Failed to read line");
}
