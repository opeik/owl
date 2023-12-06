pub mod cec;
pub mod win32;

use std::sync::OnceLock;

use crate::cec::Event;
use tokio::sync::mpsc::Sender;
use windows::Win32::UI::WindowsAndMessaging::HHOOK;

pub static EVENT_TX: OnceLock<Sender<Event>> = OnceLock::new();
pub static KEYBOARD_HOOK: OnceLock<HHOOK> = OnceLock::new();
