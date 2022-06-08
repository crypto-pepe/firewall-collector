use futures::Future;
use tokio::sync::{oneshot, Mutex};

pub struct Shutdowner {
    // sender and receiver self-consumed while send, therefore it has to be optional
    pub tick_sender: Mutex<Option<oneshot::Sender<()>>>,
    pub tick_receiver: Mutex<Option<oneshot::Receiver<()>>>,
    pub tock_sender: Mutex<oneshot::Sender<()>>,
    pub tock_receiver: Mutex<oneshot::Receiver<()>>,
}

impl Shutdowner {
    pub fn new() -> Self {
        let (tick_sender, tick_receiver) = oneshot::channel();
        let (tock_sender, tock_receiver) = oneshot::channel();
        Self {
            tick_sender: Mutex::new(Some(tick_sender)),
            tick_receiver: Mutex::new(Some(tick_receiver)),
            tock_sender: Mutex::new(tock_sender),
            tock_receiver: Mutex::new(tock_receiver),
        }
    }

    /// Start the shutdown
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

    /// Raise stopped signal from current process
    pub async fn stopped(&self) {
        let mut tock_receiver = self.tock_receiver.lock().await;

        tock_receiver.close()
    }

    /// Get stop signal handle
    ///
    /// Example:
    /// ```rust
    /// let mut stopper = tick_tock.stop_handle().await.expect("Error occurred while extracting stopper from tick tock");
    ///
    /// tokio::select!{
    ///   _ = task_handle => {
    ///     // task handler
    ///   }
    ///   _ = &mut stopper => {
    ///     debug!("handle stop signal");
    ///   }
    /// }
    /// ```
    pub async fn stop_handle(
        &self,
    ) -> Result<impl Future<Output = Result<(), impl std::error::Error>>, anyhow::Error> {
        let mut tick_receiver = self.tick_receiver.lock().await;
        if let Some(tick_receiver) = tick_receiver.take() {
            Ok(tick_receiver)
        } else {
            Err(anyhow::anyhow!("Tried to take tick_receiver twice"))
        }
    }

    /// Handle process stopping completion
    pub async fn finished(&self) {
        let mut tock_sender = self.tock_sender.lock().await;
        tock_sender.closed().await
    }
}
