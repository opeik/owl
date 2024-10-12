mod handlers;
mod key;
mod power;
mod window;

use std::{sync::OnceLock, thread};

use color_eyre::eyre::{eyre, Context, Result};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, trace};

use crate::{
    job::{self, Recv, SpawnResult},
    os::{self, windows::window::Window, Event, EventRx},
    Spawn,
};

/// Represents a Windows job, responsible for sending and receiving Windows
/// events.
pub struct Job {
    event_rx: EventRx,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("key error")]
    KeyError(#[from] key::Error),
    #[error("power error")]
    PowerError(#[from] power::Error),
    #[error("window error")]
    WindowError(#[from] window::Error),
}

pub(crate) struct OwlHandle {
    pub err_tx: os::ErrorTx,
    pub event_tx: os::EventTx,
}

/// A handle to owl.
///
/// I hate global, mutable state as much as you do, but we have no other
/// options. Sure, for [`handle_window_event`] we can use `cbWndExtra` via
/// [`SetWindowPtrLong`] and [`GetWindowPtrLong`], but that's not an option for
/// [`handle_low_level_keyboard_event`]. Getting a value from the window
/// requires us to have a window handle, which the low-level hook doesn't have,
/// as it doesn't know which window will receive the event.
///
/// [`GetWindowPtrLong`]: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowlongptra
/// [`SetWindowPtrLong`]: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowlongptrw
pub(crate) static OWL_HANDLE: OnceLock<OwlHandle> = OnceLock::new();

impl Spawn for Job {
    /// Spawns a new Windows job. The job runs on a thread.
    async fn spawn(run_token: CancellationToken) -> SpawnResult<Self> {
        let (err_tx, err_rx) = mpsc::unbounded_channel::<Error>();
        let (event_tx, event_rx) = mpsc::unbounded_channel::<Event>();
        let (window_tx, window_rx) = oneshot::channel::<Window>();
        let (ready_tx, ready_rx) = oneshot::channel::<Result<()>>();

        let _err_logger = tokio::spawn(async move {
            let mut err_rx = err_rx;
            loop {
                match err_rx.recv().await.ok_or_else(|| eyre!("event rx closed")) {
                    Ok(err) => error!("os error occurred: {err}"),
                    Err(e) => error!("failed to receive os error: {e:?}"),
                }
            }
        });

        debug!("spawning os job...");
        let join_handle = thread::spawn(move || {
            debug!("os job starting...");

            // Windows will get mad if you try to use resources outside the thread that
            // created it. Fortunately, the `Drop` implementation sidesteps this
            // with message passing. So, create the window in the job thread
            // then send it back to async land.
            job::send_ready_status(ready_tx, || {
                match Window::new(err_tx.clone(), event_tx.clone()) {
                    Ok(x) => {
                        debug!("sending window handle to task...");
                        window_tx
                            .send(x)
                            .map_err(|_| eyre!("failed to send window handle to task"))
                    }
                    Err(e) => Err(color_eyre::eyre::Error::from(e)),
                }
            })?;

            self::handlers::event_loop();
            Result::Ok(())
        });

        ready_rx
            .await
            .context("failed to read job status")?
            .context("job failed to start")?;
        debug!("os job ready!");

        let window = window_rx
            .await
            .context("failed to receive window handle from job")?;
        debug!("received window handle from job!");

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

pub(crate) fn send_err(err_tx: &os::ErrorTx, err: os::Error) {
    trace!("relaying error: {err:?}");
    if let Err(e) = err_tx.send(err) {
        error!("failed to relay err: {e}");
    };
}

pub(crate) fn send_event(event_tx: &os::EventTx, event: os::Event) {
    trace!("relaying event: {event:?}");
    if let Err(e) = event_tx.send(event) {
        error!("failed to relay event: {event:?}: {e}");
    };
}

macro_rules! get_owl_handle {
    ($on_err:expr) => {{
        use tracing::error;

        use crate::os::windows::OWL_HANDLE;

        match OWL_HANDLE.get() {
            Some(x) => OwlHandle {
                err_tx: x.err_tx.clone(),
                event_tx: x.event_tx.clone(),
            },
            None => {
                error!("owl state unset");
                return { $on_err() };
            }
        }
    }};
}

pub(crate) use get_owl_handle;
