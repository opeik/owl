use tracing::{debug, error};

use super::{get_owl_handle, power::Event, send_event, OwlHandle};
use crate::os::{
    self,
    windows::{key, send_err, window},
};

mod win32 {
    pub use windows::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::SystemServices,
        UI::{
            Input::KeyboardAndMouse::{self},
            WindowsAndMessaging::{self},
        },
    };
}

pub fn event_loop() {
    let mut msg = win32::WindowsAndMessaging::MSG::default();

    unsafe {
        // Get a message from the window's event queue.
        // See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmessagew
        while win32::WindowsAndMessaging::GetMessageW(&mut msg, None, 0, 0).into() {
            // Dispatch the received message.
            // See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-dispatchmessagew
            win32::WindowsAndMessaging::DispatchMessageW(&msg);
        }
    }
}

/// Our window event handler. This allows us listen to a variety of interesting
/// events, including power events.
///
/// See: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nc-winuser-wndproc>
pub extern "system" fn handle_window_event(
    window: win32::HWND,
    msg: u32,
    wparam: win32::WPARAM,
    lparam: win32::LPARAM,
) -> win32::LRESULT {
    // See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-defwindowprocw
    let defer =
        || unsafe { win32::WindowsAndMessaging::DefWindowProcW(window, msg, wparam, lparam) };
    let ok = || win32::LRESULT(0);
    let OwlHandle {
        err_tx: error_tx,
        event_tx,
    } = get_owl_handle!(defer);

    match msg {
        // The window should terminate.
        // See: https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-close
        win32::WindowsAndMessaging::WM_CLOSE => {
            // See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-destroywindow
            debug!("received `WM_CLOSE` event, destroying window...");
            unsafe {
                if let Err(e) = win32::WindowsAndMessaging::DestroyWindow(window) {
                    send_err(&error_tx, window::Error::DropFailed(e).into());
                }
            };
            return ok();
        }

        // The window is being destroyed.
        // See: https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-destroy
        win32::WindowsAndMessaging::WM_DESTROY => {
            // See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-postquitmessage
            debug!("received `WM_DESTROY` event, stopping event loop...");
            unsafe { win32::WindowsAndMessaging::PostQuitMessage(0) };
            return ok();
        }

        // A power-management event has occurred.
        // See: https://learn.microsoft.com/en-us/windows/win32/power/wm-powerbroadcast
        win32::WindowsAndMessaging::WM_POWERBROADCAST => {
            let power_msg = match u32::try_from(wparam.0) {
                Ok(x) => x,
                Err(e) => {
                    error!("failed to convert window message params: {e}");
                    return defer();
                }
            };

            match power_msg {
                // The system is resuming from sleep.
                // See: https://learn.microsoft.com/en-us/windows/win32/power/pbt-apmresumeautomatic
                win32::WindowsAndMessaging::PBT_APMRESUMEAUTOMATIC => {
                    send_event(&event_tx, os::Event::Resume);
                }

                // The system is about to sleep.
                // See: https://learn.microsoft.com/en-us/windows/win32/power/pbt-apmsuspend
                win32::WindowsAndMessaging::PBT_APMSUSPEND => {
                    send_event(&event_tx, os::Event::Suspend);
                }

                // A power setting change occurred.
                // See: https://learn.microsoft.com/en-us/windows/win32/power/pbt-powersettingchange
                win32::WindowsAndMessaging::PBT_POWERSETTINGCHANGE => {
                    if let Ok(power_event) = Event::try_from(lparam)
                    // Check the current display is turning off.
                    && power_event.target() == win32::SystemServices::GUID_CONSOLE_DISPLAY_STATE
                    && power_event.state() == win32::SystemServices::PowerMonitorOff
                    {
                        send_event(&event_tx, os::Event::Suspend);
                    }
                }
                _ => {}
            };
        }

        _ => {}
    };

    defer()
}

/// Our low-level key event handler. As per the docs, it's important to do our
/// work as quickly as possible to avoid impacting system performance. We need
/// to use a low-level hook ([`WH_KEYBOARD_LL`]) as opposed to a normal hook
/// ([`WH_KEYBOARD`]) since it allow us to suppress certain keys being handled.
///
/// See: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw>
///
/// [`WH_KEYBOARD`]: https://learn.microsoft.com/en-us/windows/win32/winmsg/keyboardproc
/// [`WH_KEYBOARD_LL`]: https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc
pub extern "system" fn handle_low_level_key_event(
    ncode: i32,
    wparam: win32::WPARAM,
    lparam: win32::LPARAM,
) -> win32::LRESULT {
    /// Indicates the event is a keyboard event.
    /// See: <https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc>
    #[allow(clippy::cast_possible_wrap)]
    const HC_ACTION: i32 = win32::WindowsAndMessaging::HC_ACTION as i32;

    // See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-callnexthookex
    let defer =
        || unsafe { win32::WindowsAndMessaging::CallNextHookEx(None, ncode, wparam, lparam) };
    let suppress = || win32::LRESULT(1);

    // Bail if this isn't a keyboard event.
    if ncode < 0 || ncode != HC_ACTION {
        return defer();
    }

    let OwlHandle { err_tx, event_tx } = get_owl_handle!(defer);
    match key::Event::try_from((wparam, lparam)) {
        Ok(key_event) => match key_event.to_owl_event() {
            // We got an event we care about!
            Some(owl_event) => {
                send_event(&event_tx, owl_event);

                // Unless volume events are suppressed, they'll operate as normal. This isn't
                // desirable since we're trying to replace software mixing with
                // hardware mixing. The software mixer works by reducing audio
                // bit-depth to make the audio quieter, at the expense of audio quality.
                match *key_event.code {
                    win32::KeyboardAndMouse::VK_VOLUME_DOWN
                    | win32::KeyboardAndMouse::VK_VOLUME_UP
                    | win32::KeyboardAndMouse::VK_VOLUME_MUTE => suppress(),
                    _ => defer(),
                }
            }
            None => defer(),
        },
        Err(e) => {
            send_err(&err_tx, e.into());
            defer()
        }
    }
}
