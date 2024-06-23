pub type SpawnResult<T> = Result<(JoinHandle<Result<()>>, T)>;

use std::thread::JoinHandle;

use color_eyre::Result;
use tokio_util::sync::CancellationToken;

#[allow(async_fn_in_trait)]
pub trait Spawn {
    async fn spawn(run_token: CancellationToken) -> SpawnResult<Self>
    where
        Self: Sized;
}

#[allow(async_fn_in_trait)]
pub trait Recv<T> {
    async fn recv(&mut self) -> Result<T>;
}

#[allow(async_fn_in_trait)]
pub trait Send<T> {
    async fn send(&self, value: T) -> Result<()>;
}
