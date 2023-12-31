use tokio::sync::mpsc;

pub type EventTx = mpsc::UnboundedSender<Event>;
pub type EventRx = mpsc::UnboundedReceiver<Event>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::Display)]
pub enum Event {
    Suspend,
    Resume,
    VolumeUp,
    VolumeDown,
    VolumeMute,
    Focus,
}

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        pub mod windows;
        pub use windows::Job;
    }
}
