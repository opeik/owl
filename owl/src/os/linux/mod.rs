use color_eyre::eyre::Result;
use tokio_util::sync::CancellationToken;

use crate::{
    job::{Recv, SpawnResult},
    os::Event,
    Spawn,
};

pub struct Job;

impl Spawn for Job {
    /// Spawns a new Linux job.
    async fn spawn(_run_token: CancellationToken) -> SpawnResult<Self> {
        unimplemented!()
    }
}

impl Recv<Event> for Job {
    async fn recv(&mut self) -> Result<Event> {
        unimplemented!()
    }
}
