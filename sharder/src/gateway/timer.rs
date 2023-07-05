use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;

pub fn timer(tx: mpsc::Sender<()>, duration: Duration, tick_immediately: bool) {
    tokio::spawn(async move {
        let mut interval = interval(duration);

        if !tick_immediately {
            interval.tick().await;
        }

        loop {
            interval.tick().await;
            if tx.send(()).await.is_err() {
                // Stop looping when receiver dropped
                break;
            }
        }
    });
}
