use tokio::sync::{oneshot, Mutex};

pub struct TickTock {
    // sender self-consumed while send, therefore it has to be optional
    pub tick_sender: Mutex<Option<oneshot::Sender<()>>>,
    pub tick_receiver: oneshot::Receiver<()>,
    pub tock_sender: Mutex<oneshot::Sender<()>>,
    pub tock_receiver: Mutex<oneshot::Receiver<()>>,
}

impl TickTock {
    pub fn new() -> TickTock {
        let (tick_sender, tick_receiver) = oneshot::channel();
        let (tock_sender, tock_receiver) = oneshot::channel();
        TickTock {
            tick_sender: Mutex::new(Some(tick_sender)),
            tick_receiver,
            tock_sender: Mutex::new(tock_sender),
            tock_receiver: Mutex::new(tock_receiver),
        }
    }

    pub async fn stop(&self) -> Result<(), anyhow::Error> {
        let mut tick_sender = self.tick_sender.lock().await;
        if let Some(sender) = tick_sender.take() {
            sender
                .send(())
                .map_err(|_| anyhow::anyhow!("Cannot send to oneshot channel"))
        } else {
            Err(anyhow::anyhow!("Tick sender already completed"))
        }
    }

    pub async fn on_stop(&self) -> &tokio::sync::oneshot::Receiver<()> {
        &self.tick_receiver
    }

    pub async fn close(&self) -> () {
        let mut tock_receiver = self.tock_receiver.lock().await;

        tock_receiver.close()
    }

    pub async fn closed(&self) -> () {
        let mut tock_sender = self.tock_sender.lock().await;
        tock_sender.closed().await
    }
}
