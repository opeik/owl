use std::{ptr, sync::OnceLock};

use color_eyre::eyre::{eyre, Context, Result};
use tracing::{error, trace};

use crate::os as owl;

mod api {
    pub use windows::{
        core::{w, PCWSTR},
        Win32::{
            Foundation::{HMODULE, HWND, LPARAM, LRESULT, WPARAM},
            System::{
                LibraryLoader,
                Power::{self, HPOWERNOTIFY, POWERBROADCAST_SETTING},
                SystemServices,
            },
            UI::{
                Input::KeyboardAndMouse::{self, VIRTUAL_KEY},
                WindowsAndMessaging::{self, HHOOK, KBDLLHOOKSTRUCT, WINDOW_EX_STYLE, WNDCLASSW},
            },
        },
    };
}

macro_rules! get_owl_handle {
    ($on_err:expr) => {
        match OWL_HANDLE.get() {
            Some(x) => OwlHandle {
                event_tx: x.event_tx.clone(),
            },
            None => {
                error!("owl state unset");
                return { $on_err() };
            }
        }
    };
}

// I hate global, mutable state as much as you do, but we have no other options. There doesn't appear
// to be another way to smuggle a reference to our code from win32 land.
static OWL_HANDLE: OnceLock<OwlHandle> = OnceLock::new();

struct OwlHandle {
    pub event_tx: owl::EventTx,
}

#[derive(Debug)]
pub struct Window {
    pub(crate) handle: api::HWND,
    pub(crate) key_hook: api::HHOOK,
    pub(crate) power_notify: api::HPOWERNOTIFY,
}

#[derive(Debug, Clone, Copy, derive_more::Deref)]
struct PowerSettings(pub api::POWERBROADCAST_SETTING);

#[derive(Debug, Clone, Copy, derive_more::Deref)]
struct KeyEvent(pub api::KBDLLHOOKSTRUCT);

#[derive(Debug, Clone, Copy, derive_more::Deref)]
struct KeyState(pub u32);

#[derive(Debug, Clone, Copy, derive_more::Deref)]
struct KeyCode(pub api::VIRTUAL_KEY);

pub fn event_loop() {
    // TODO: there's _got_ to be a better way to do this
    let mut msg = api::WindowsAndMessaging::MSG::default();
    unsafe {
        while api::WindowsAndMessaging::GetMessageW(&mut msg, None, 0, 0).into() {
            api::WindowsAndMessaging::DispatchMessageW(&msg);
        }
    }
}

impl Window {
    const WINDOW_CLASS: api::PCWSTR = api::w!("window");

    pub fn new(event_tx: owl::EventTx) -> Result<Self> {
        OWL_HANDLE
            .set(OwlHandle { event_tx })
            .map_err(|_| eyre!("failed to set owl state"))?;

        trace!("creating window...");
        let module = Self::module_handle()?;
        let _window_class = Self::new_window_class(module)?;
        let window = Self::new_window(module)?;
        let key_hook = Self::new_key_hook(module)?;
        let power_notify = Self::new_power_notify(window)?;
        trace!("window created!");

        Ok(Self {
            handle: window,
            key_hook,
            power_notify,
        })
    }

    fn module_handle() -> Result<api::HMODULE> {
        trace!("getting module handle...");
        let module = unsafe {
            api::LibraryLoader::GetModuleHandleW(None).context("failed to get module handle")?
        };

        if module.is_invalid() {
            return Err(eyre!("failed to get module handle"));
        }

        Ok(module)
    }

    fn new_window_class(module: api::HMODULE) -> Result<api::WNDCLASSW> {
        trace!("registering window class...");
        let window_class = api::WNDCLASSW {
            hInstance: module.into(),
            style: api::WindowsAndMessaging::CS_HREDRAW | api::WindowsAndMessaging::CS_VREDRAW,
            lpszClassName: Self::WINDOW_CLASS,
            lpfnWndProc: Some(handle_window_event),
            ..Default::default()
        };

        let atom = unsafe { api::WindowsAndMessaging::RegisterClassW(&window_class) };
        if atom == 0 {
            return Err(eyre!("failed to register class"));
        }

        Ok(window_class)
    }

    fn new_window(module: api::HMODULE) -> Result<api::HWND> {
        trace!("creating window...");
        let window = unsafe {
            api::WindowsAndMessaging::CreateWindowExW(
                api::WINDOW_EX_STYLE::default(),
                Self::WINDOW_CLASS,
                api::w!("owl"),
                api::WindowsAndMessaging::WS_DISABLED,
                api::WindowsAndMessaging::CW_USEDEFAULT,
                api::WindowsAndMessaging::CW_USEDEFAULT,
                api::WindowsAndMessaging::CW_USEDEFAULT,
                api::WindowsAndMessaging::CW_USEDEFAULT,
                None,
                None,
                module,
                None,
            )
        };

        #[allow(clippy::cast_sign_loss)]
        if ptr::with_exposed_provenance::<usize>(window.0 as usize).is_null() {
            return Err(eyre!("failed to create window"));
        }

        Ok(window)
    }

    fn new_power_notify(window: api::HWND) -> Result<api::HPOWERNOTIFY> {
        trace!("registering for power notifications...");
        unsafe {
            api::Power::RegisterPowerSettingNotification(
                window,
                &api::SystemServices::GUID_CONSOLE_DISPLAY_STATE,
                api::WindowsAndMessaging::DEVICE_NOTIFY_WINDOW_HANDLE,
            )
            .context("failed to register power notifications")
        }
    }

    fn new_key_hook(module: api::HMODULE) -> Result<api::HHOOK> {
        trace!("registering key hook...");
        unsafe {
            api::WindowsAndMessaging::SetWindowsHookExW(
                api::WindowsAndMessaging::WH_KEYBOARD_LL,
                Some(handle_key_event),
                module,
                0,
            )
            .context("failed to register key hook")
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        let inner = |window: &mut Window| -> Result<()> {
            trace!("posting `WM_CLOSE` message...");

            unsafe {
                api::WindowsAndMessaging::PostMessageW(
                    window.handle,
                    api::WindowsAndMessaging::WM_CLOSE,
                    api::WPARAM::default(),
                    api::LPARAM::default(),
                )?;
            };

            trace!("unregistering power notifications...");
            unsafe { api::Power::UnregisterPowerSettingNotification(window.power_notify)? };

            trace!("unregistering key hook...");
            unsafe { api::WindowsAndMessaging::UnhookWindowsHookEx(window.key_hook)? };
            Ok(())
        };

        trace!("dropping window...");
        if let Err(e) = inner(self) {
            error!("failed to drop window: {e}");
        }
    }
}

unsafe impl Send for Window {}

impl KeyEvent {
    fn key_code(&self) -> Result<KeyCode> {
        let inner =
            api::VIRTUAL_KEY(u16::try_from(self.vkCode).context("failed to convert key code")?);
        Ok(KeyCode(inner))
    }

    // fn to_owl_event(self) -> owl::Event {}
}

impl TryFrom<api::LPARAM> for KeyEvent {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: api::LPARAM) -> Result<Self, Self::Error> {
        #[allow(clippy::cast_sign_loss)]
        let event = ptr::with_exposed_provenance::<api::KBDLLHOOKSTRUCT>(value.0 as usize);
        if event.is_null() {
            return Err(eyre!("null key event"));
        }

        Ok(KeyEvent(unsafe { *event }))
    }
}

impl TryFrom<api::WPARAM> for KeyState {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: api::WPARAM) -> Result<Self, Self::Error> {
        match u32::try_from(value.0) {
            Ok(x) => Ok(KeyState(x)),
            Err(e) => Err(eyre!("failed to convert key state: {e}")),
        }
    }
}

impl TryFrom<api::LPARAM> for PowerSettings {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: api::LPARAM) -> Result<PowerSettings> {
        #[allow(clippy::cast_sign_loss)]
        let power_settings =
            ptr::with_exposed_provenance::<api::POWERBROADCAST_SETTING>(value.0 as usize);

        if !power_settings.is_null() {
            return Err(eyre!("null power settings"));
        }

        Ok(PowerSettings(unsafe { *power_settings }))
    }
}

fn key_to_event(key_code: KeyCode, key_state: KeyState) -> Option<owl::Event> {
    let event = match *key_state {
        api::WindowsAndMessaging::WM_KEYDOWN => owl::Event::Press,
        api::WindowsAndMessaging::WM_KEYUP => owl::Event::Release,
        _ => return None,
    };

    match *key_code {
        api::KeyboardAndMouse::VK_VOLUME_DOWN => Some(event(owl::Key::VolumeDown)),
        api::KeyboardAndMouse::VK_VOLUME_UP => Some(event(owl::Key::VolumeUp)),
        api::KeyboardAndMouse::VK_VOLUME_MUTE => Some(event(owl::Key::VolumeMute)),
        _ => Some(owl::Event::Focus),
    }
}

fn send_event(event_tx: &owl::EventTx, event: owl::Event) {
    trace!("relaying event: `{event:?}`");
    if let Err(e) = event_tx.send(event) {
        error!("failed to relay event `{event:?}`: {e}");
    };
}

extern "system" fn handle_window_event(
    window: api::HWND,
    message: u32,
    wparam: api::WPARAM,
    lparam: api::LPARAM,
) -> api::LRESULT {
    const DISPLAY_OFF: u8 = 0;

    let defer =
        || unsafe { api::WindowsAndMessaging::DefWindowProcW(window, message, wparam, lparam) };
    let ok = || api::LRESULT(0);
    let OwlHandle { event_tx } = get_owl_handle!(defer);

    let message_params = match u32::try_from(wparam.0) {
        Ok(x) => x,
        Err(e) => {
            error!("failed to convert window message params: {e}");
            return defer();
        }
    };

    match message {
        // The window should terminate.
        // See: https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-close
        api::WindowsAndMessaging::WM_CLOSE => {
            trace!("received `WM_CLOSE` event, destroying window...");
            unsafe {
                api::WindowsAndMessaging::DestroyWindow(window).expect("failed to destroy window");
            };
            return ok();
        }
        // The window is being destroyed.
        // See: https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-destroy
        api::WindowsAndMessaging::WM_DESTROY => {
            trace!("received `WM_DESTROY` event, stopping event loop...");
            unsafe { api::WindowsAndMessaging::PostQuitMessage(0) };
            return ok();
        }

        // A power-management event has occurred.
        // See: https://learn.microsoft.com/en-us/windows/win32/power/wm-powerbroadcast
        api::WindowsAndMessaging::WM_POWERBROADCAST => match message_params {
            // The system is resuming from sleep.
            // See: https://learn.microsoft.com/en-us/windows/win32/power/pbt-apmresumeautomatic
            api::WindowsAndMessaging::PBT_APMRESUMEAUTOMATIC => {
                send_event(&event_tx, owl::Event::Resume);
            }

            // The system is about to sleep.
            // See: https://learn.microsoft.com/en-us/windows/win32/power/pbt-apmsuspend
            api::WindowsAndMessaging::PBT_APMSUSPEND => {
                send_event(&event_tx, owl::Event::Suspend);
            }

            // Power setting change occurred.
            // See: https://learn.microsoft.com/en-us/windows/win32/power/pbt-powersettingchange
            api::WindowsAndMessaging::PBT_POWERSETTINGCHANGE => {
                if let Ok(power_settings) = PowerSettings::try_from(lparam)
                    && let new_power_setting = power_settings.Data[0]
                    && let event_target = power_settings.PowerSetting
                    // The current monitor's display state has changed.
                    // See: https://learn.microsoft.com/en-us/windows/win32/power/power-setting-guids
                    && event_target == api::SystemServices::GUID_CONSOLE_DISPLAY_STATE
                    && new_power_setting == DISPLAY_OFF
                {
                    send_event(&event_tx, owl::Event::Suspend);
                }
            }
            _ => {}
        },
        _ => {}
    };

    defer()
}

fn convert_key_event(wparam: api::WPARAM, lparam: api::LPARAM) -> Result<(KeyCode, KeyState)> {
    let key_event = KeyEvent::try_from(lparam)?;
    let key_code = key_event.key_code()?;
    let key_state = KeyState::try_from(wparam)?;
    Ok((key_code, key_state))
}

extern "system" fn handle_key_event(
    ncode: i32,
    wparam: api::WPARAM,
    lparam: api::LPARAM,
) -> api::LRESULT {
    /// Indicates the event is a keyboard event.
    /// See: <https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc>
    #[allow(clippy::cast_possible_wrap)]
    const HC_ACTION: i32 = api::WindowsAndMessaging::HC_ACTION as i32;

    let defer = || unsafe { api::WindowsAndMessaging::CallNextHookEx(None, ncode, wparam, lparam) };
    let suppress = || api::LRESULT(1);

    // Bail if this isn't a keyboard event.
    if ncode < 0 || ncode != HC_ACTION {
        return defer();
    }

    let OwlHandle { event_tx } = get_owl_handle!(defer);
    match convert_key_event(wparam, lparam) {
        Ok((key_code, key_state)) => match key_to_event(key_code, key_state) {
            // We got a key event we're interested in!
            Some(event) => {
                send_event(&event_tx, event);

                // Unless volume events are suppressed, they'll operate as normal. This isn't desirable since
                // we're trying to replace software mixing with hardware mixing. The software mixer works
                // by reducing audio bit-depth to make the audio quieter, at the expense of audio quality.
                match *key_code {
                    api::KeyboardAndMouse::VK_VOLUME_DOWN
                    | api::KeyboardAndMouse::VK_VOLUME_UP
                    | api::KeyboardAndMouse::VK_VOLUME_MUTE => suppress(),
                    _ => defer(),
                }
            }
            None => defer(),
        },
        Err(e) => {
            error!("failed to convert key event: {e}");
            defer()
        }
    }
}
