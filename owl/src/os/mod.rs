use tokio::sync::mpsc;

pub type EventTx = mpsc::UnboundedSender<Event>;
pub type EventRx = mpsc::UnboundedReceiver<Event>;

/// Represents a keyboard key targetted for HDMI-CEC integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    VolumeUp,
    VolumeDown,
    VolumeMute,
}

/// Represents an OS event targetted for HDMI-CEC integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Suspend,
    Resume,
    Focus,
    Press(Key),
    Release(Key),
}

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        pub mod windows;
        pub use windows::Job;
    }
}
