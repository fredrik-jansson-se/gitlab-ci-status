use termion::{event::Key, input::TermRead};

pub(crate) enum Event {
    Tick,
    Key(Key),
}

pub(crate) async fn event_handler(tx: tokio::sync::mpsc::Sender<Event>) {
    let mut keys = termion::async_stdin().keys();
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(200));
    loop {
        let res = match keys.next() {
            None => {
                interval.tick().await;
                tx.send(Event::Tick).await
            }
            Some(Ok(k)) => tx.send(Event::Key(k)).await,
            Some(Err(_)) => return,
        };

        if res.is_err() {
            return;
        }
    }
}
