pub type SpawnResult<T> = Result<(JoinHandle<Result<()>>, T)>;

use std::thread::JoinHandle;

use color_eyre::{eyre::eyre, Result};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tracing::error;

#[allow(async_fn_in_trait)]
pub trait Spawn {
    /// Spawns a new owl job. Depending on the implementation the job may use
    /// tasks or threads.
    async fn spawn(run_token: CancellationToken) -> SpawnResult<Self>
    where
        Self: Sized;
}

#[allow(async_fn_in_trait)]
pub trait Recv<T> {
    /// Receives a value from an owl job.
    async fn recv(&mut self) -> Result<T>;
}

#[allow(async_fn_in_trait)]
pub trait Send<T> {
    /// Sends a value to an owl job.
    async fn send(&self, value: T) -> Result<()>;
}

pub fn send_ready_status<T, F>(ready_tx: oneshot::Sender<Result<()>>, func: F) -> Result<T>
where
    T: std::fmt::Debug,
    F: FnOnce() -> Result<T>,
{
    let (result, status) = match func() {
        Ok(x) => (Ok(x), Ok(())),
        // This is only used to kill the job early, so the message doesn't matter.
        Err(e) => (Err(eyre!("func failed")), Err(e)),
    };

    if let Err(e) = ready_tx.send(status) {
        error!("failed to send job status: {:?}", e);
    }

    result
}
