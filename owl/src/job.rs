pub type SpawnResult<T> = Result<(JoinHandle<Result<()>>, T)>;

use std::thread::JoinHandle;

use color_eyre::Result;
use tokio_util::sync::CancellationToken;

#[allow(async_fn_in_trait)]
pub trait Spawn {
    /// Spawns a new owl job. Depending on the implementation the job may use tasks or threads.
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
