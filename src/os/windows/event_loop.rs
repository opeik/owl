use color_eyre::eyre::Result;
use std::thread::{self, JoinHandle};

use tracing::debug;
use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, MSG};

use crate::os::{windows::Window, EventRx};

pub fn spawn() -> Result<(JoinHandle<Result<()>>, EventRx)> {
    let (window, event_rx) = Window::new()?;

    let handle = thread::spawn(move || {
        debug!("os thread started!");

        let mut message = MSG::default();
        unsafe {
            while GetMessageW(&mut message, window.window(), 0, 0).into() {
                DispatchMessageW(&message);
            }
        }

        debug!("stopping os thread...");
        Result::Ok(())
    });

    Ok((handle, event_rx))
}
