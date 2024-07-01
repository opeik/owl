mod internal;

use std::thread;

use color_eyre::eyre::{eyre, Context, Result};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tracing::{debug, trace};

use crate::{
    job::{self, Recv, SpawnResult},
    os::{windows::internal::Window, Event, EventRx},
    Spawn,
};

/// Represents a Windows job, responsible for sending and receiving Windows
/// events.
pub struct Job {
    event_rx: EventRx,
}

impl Spawn for Job {
    /// Spawns a new Windows job. The job runs on a thread.
    async fn spawn(run_token: CancellationToken) -> SpawnResult<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel::<Event>();
        let (window_tx, window_rx) = oneshot::channel::<Window>();
        let (ready_tx, ready_rx) = oneshot::channel::<Result<()>>();

        trace!("spawning os job...");
        let join_handle = thread::spawn(move || {
            debug!("os job starting...");

            // Windows will get mad if you try to use resources outside the thread that
            // created it. Fortunately, the `Drop` implementation sidesteps this
            // with message passing. So, create the window in the job thread
            // then send it back to async land.
            job::send_ready_status(ready_tx, || match Window::new(event_tx.clone()) {
                Ok(x) => {
                    trace!("sending window handle to task...");
                    window_tx
                        .send(x)
                        .map_err(|_| eyre!("failed to send window handle to task"))
                }
                Err(e) => Err(e),
            })?;

            self::internal::event_loop();
            Result::Ok(())
        });

        let window = window_rx.await?;
        trace!("received window handle from thread!");

        ready_rx
            .await
            .context("failed to read job status")?
            .context("job failed to start")?;
        debug!("os job ready!");

        // Dropping the `Window` will stop the event loop, saving us having to poll.
        let _watchdog = tokio::spawn(async move {
            run_token.cancelled().await;
            drop(window);
        });

        Ok((join_handle, Self { event_rx }))
    }
}

impl Recv<Event> for Job {
    async fn recv(&mut self) -> Result<Event> {
        self.event_rx
            .recv()
            .await
            .ok_or_else(|| eyre!("event rx closed"))
    }
}
