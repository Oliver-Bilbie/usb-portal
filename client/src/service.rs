use log::*;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

pub struct Service {
    name: String,
    thread: Option<JoinHandle<()>>,
    token: CancellationToken,
}

impl Service {
    pub fn init<F, Fut>(name: &str, loop_fn: F) -> Service
    where
        F: Fn() -> Fut + std::marker::Send + 'static,
        Fut: Future<Output = Result<(), ()>> + std::marker::Send,
    {
        debug!("Starting {} thread", name);
        let token = CancellationToken::new();
        let thread = Some(tokio::spawn({
            let t = token.clone();
            async move {
                loop {
                    tokio::select! {
                        _ = t.cancelled() => { return; },
                        result = loop_fn() => {
                            if result.is_err() { return; }
                        }
                    };
                }
            }
        }));
        Service {
            name: name.to_string(),
            thread,
            token,
        }
    }

    pub async fn shutdown(&mut self) {
        debug!("Gracefully stopping the {} thread", self.name);
        self.token.cancel();
        if let Some(thread) = self.thread.take() {
            _ = thread.await;
        }
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        // Async drop is not yet supported in stable Rust, so we cannot await the termination of the
        // thread. Use the shutdown method instead.
        if !self.token.is_cancelled() {
            warn!("The {} thread was not terminated gracefully", self.name);
            self.token.cancel();
        }
    }
}
