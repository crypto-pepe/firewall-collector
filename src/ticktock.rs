use tokio::sync::oneshot;

pub struct TickTock {
    pub tick_sender: oneshot::Sender<()>,
    pub tick_receiver: oneshot::Receiver<()>,
    pub tock_sender: oneshot::Sender<()>,
    pub tock_receiver: oneshot::Receiver<()>,
}

impl TickTock {
    pub fn new() -> TickTock {
        let (tick_sender, tick_receiver) = oneshot::channel();
        let (tock_sender, tock_receiver) = oneshot::channel();
        TickTock {
            tick_sender,
            tick_receiver,
            tock_sender,
            tock_receiver,
        }
    }
}
