use color_eyre::Result;
use std::{future::Future, thread::JoinHandle};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub type EventTx = mpsc::Sender<Event>;
pub type EventRx = mpsc::Receiver<Event>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::Display)]
pub enum Event {
    Suspend,
    Resume,
    VolumeUp,
    VolumeDown,
    VolumeMute,
    UserActivity,
}

pub trait Spawn {
    fn spawn(cancel_token: CancellationToken) -> (JoinHandle<Result<()>>, Self);
    fn recv_event(&mut self) -> impl Future<Output = Result<Event>> + Send;
}

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        pub mod windows;
        pub use windows::Job;
    }
}
