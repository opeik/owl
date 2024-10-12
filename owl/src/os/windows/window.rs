use std::ptr;

use tracing::debug;

use crate::os::{
    self,
    windows::{
        get_owl_handle,
        handlers::{handle_low_level_key_event, handle_window_event},
        send_err, OwlHandle, OWL_HANDLE,
    },
};

mod win32 {
    pub use windows::{
        core::{w, Error, PCWSTR},
        Win32::{
            Foundation::{HMODULE, HWND, LPARAM, WPARAM},
            System::{
                LibraryLoader,
                Power::{self, HPOWERNOTIFY},
                SystemServices::{self},
            },
            UI::WindowsAndMessaging::{self, HHOOK, WINDOW_EX_STYLE, WNDCLASSW},
        },
    };
}

#[derive(Debug)]
pub struct Window {
    /// See: <https://learn.microsoft.com/en-us/windows/win32/winprog/windows-data-types#HWND>
    handle: win32::HWND,
    /// See: <https://learn.microsoft.com/en-us/windows/win32/winprog/windows-data-types#HHOOK>
    key_hook: win32::HHOOK,
    /// See: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerpowersettingnotification>
    power_notify: win32::HPOWERNOTIFY,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to create window")]
    InitFailed,

    #[error("failed to drop window")]
    DropFailed(win32::Error),

    #[error("failed to initialize owl handle")]
    OwlHandleInitFailed,

    #[error("module handle is invalid")]
    InvalidModuleHandle,
    #[error("failed to get module handle")]
    GetModuleHandleFailed(win32::Error),

    #[error("failed to register class")]
    RegisterClassFailed,

    #[error("failed to create power setting notifications")]
    InitPowerSettingNotificationFailed(win32::Error),

    #[error("failed to initialize global hook")]
    InitHookFailed(win32::Error),

    #[error("failed to send message to window")]
    PostWindowFailed(win32::Error),

    #[error("failed to drop power settings notifications")]
    DropPowerSettingNotificationFailed(win32::Error),

    #[error("failed to drop global hook")]
    DropHookFailed(win32::Error),
}

impl Window {
    const WINDOW_CLASS: win32::PCWSTR = win32::w!("window");

    pub fn new(err_tx: os::ErrorTx, event_tx: os::EventTx) -> Result<Self, Error> {
        OWL_HANDLE
            .set(OwlHandle { err_tx, event_tx })
            .map_err(|_| Error::OwlHandleInitFailed)?;

        debug!("creating window...");
        let module = Self::module_handle()?;
        let _window_class = Self::new_window_class(module)?;
        let window = Self::new_window(module)?;
        let key_hook = Self::new_key_hook(module)?;
        let power_notify = Self::new_power_notify(window)?;
        debug!("window created!");

        Ok(Self {
            handle: window,
            key_hook,
            power_notify,
        })
    }

    /// See: <https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getmodulehandlew>
    fn module_handle() -> Result<win32::HMODULE, Error> {
        debug!("getting module handle...");

        let module = unsafe {
            win32::LibraryLoader::GetModuleHandleW(None).map_err(Error::GetModuleHandleFailed)?
        };

        if module.is_invalid() {
            return Err(Error::InvalidModuleHandle);
        }

        Ok(module)
    }

    /// See: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerclassw>
    fn new_window_class(module: win32::HMODULE) -> Result<win32::WNDCLASSW, Error> {
        debug!("registering window class...");
        let window_class = win32::WNDCLASSW {
            hInstance: module.into(),
            style: win32::WindowsAndMessaging::CS_HREDRAW | win32::WindowsAndMessaging::CS_VREDRAW,
            lpszClassName: Self::WINDOW_CLASS,
            lpfnWndProc: Some(handle_window_event),
            ..Default::default()
        };

        let atom = unsafe { win32::WindowsAndMessaging::RegisterClassW(&window_class) };
        if atom == 0 {
            return Err(Error::RegisterClassFailed);
        }

        Ok(window_class)
    }

    /// See: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-createwindowexw>
    fn new_window(module: win32::HMODULE) -> Result<win32::HWND, Error> {
        debug!("creating window...");

        let window = unsafe {
            win32::WindowsAndMessaging::CreateWindowExW(
                win32::WINDOW_EX_STYLE::default(),
                Self::WINDOW_CLASS,
                win32::w!("owl"),
                win32::WindowsAndMessaging::WS_DISABLED,
                win32::WindowsAndMessaging::CW_USEDEFAULT,
                win32::WindowsAndMessaging::CW_USEDEFAULT,
                win32::WindowsAndMessaging::CW_USEDEFAULT,
                win32::WindowsAndMessaging::CW_USEDEFAULT,
                None,
                None,
                module,
                None,
            )
        };

        #[allow(clippy::cast_sign_loss)]
        if ptr::with_exposed_provenance::<usize>(window.0 as usize).is_null() {
            return Err(Error::InitFailed);
        }

        Ok(window)
    }

    /// See: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerpowersettingnotification>
    fn new_power_notify(window: win32::HWND) -> Result<win32::HPOWERNOTIFY, Error> {
        debug!("registering for power notifications...");

        unsafe {
            win32::Power::RegisterPowerSettingNotification(
                window,
                &win32::SystemServices::GUID_CONSOLE_DISPLAY_STATE,
                win32::WindowsAndMessaging::DEVICE_NOTIFY_WINDOW_HANDLE,
            )
            .map_err(Error::InitPowerSettingNotificationFailed)
        }
    }

    /// See: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw>
    fn new_key_hook(module: win32::HMODULE) -> Result<win32::HHOOK, Error> {
        debug!("registering key hook...");

        unsafe {
            win32::WindowsAndMessaging::SetWindowsHookExW(
                win32::WindowsAndMessaging::WH_KEYBOARD_LL,
                Some(handle_low_level_key_event),
                module,
                0,
            )
            .map_err(Error::InitHookFailed)
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        let inner = |window: &mut Self| -> Result<(), Error> {
            // See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-postmessagew
            debug!("requesting the window be closed...");
            unsafe {
                win32::WindowsAndMessaging::PostMessageW(
                    window.handle,
                    win32::WindowsAndMessaging::WM_CLOSE,
                    win32::WPARAM::default(),
                    win32::LPARAM::default(),
                )
                .map_err(Error::PostWindowFailed)?;
            };

            // See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-unregisterpowersettingnotification
            debug!("unregistering power notifications...");
            unsafe {
                win32::Power::UnregisterPowerSettingNotification(window.power_notify)
                    .map_err(Error::DropPowerSettingNotificationFailed)?;
            };

            // See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-unhookwindowshookex
            debug!("unregistering key hook...");
            unsafe {
                win32::WindowsAndMessaging::UnhookWindowsHookEx(window.key_hook)
                    .map_err(Error::DropHookFailed)?;
            };
            Ok(())
        };

        debug!("dropping window...");
        if let Err(e) = inner(self) {
            let OwlHandle {
                err_tx,
                event_tx: _,
            } = get_owl_handle!(|| {});
            send_err(&err_tx, e.into());
        }
    }
}

unsafe impl Send for Window {}
