#[cfg(target_os = "windows")]
pub mod windows;

#[derive(Debug, Clone, Copy, derive_more::Display)]
pub enum Event {
    Suspend,
    Resume,
    VolumeUp,
    VolumeDown,
    VolumeMute,
}
