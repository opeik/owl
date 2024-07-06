use std::{ptr, sync::OnceLock};

use color_eyre::eyre::{eyre, Context, Result};
use tracing::{error, trace};

use crate::os as owl;

mod api {
    pub use windows::{
        core::{w, GUID, PCWSTR},
        Win32::{
            Foundation::{HMODULE, HWND, LPARAM, LRESULT, WPARAM},
            System::{
                LibraryLoader,
                Power::{self, HPOWERNOTIFY, POWERBROADCAST_SETTING},
                SystemServices::{self, MONITOR_DISPLAY_STATE},
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
static OWL_HANDLE: OnceLock<OwlHandle> = OnceLock::new();

#[derive(Debug)]
pub struct Window {
    /// See: <https://learn.microsoft.com/en-us/windows/win32/winprog/windows-data-types#HWND>
    handle: api::HWND,
    /// See: <https://learn.microsoft.com/en-us/windows/win32/winprog/windows-data-types#HHOOK>
    key_hook: api::HHOOK,
    /// See: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerpowersettingnotification>
    power_notify: api::HPOWERNOTIFY,
}

struct OwlHandle {
    pub event_tx: owl::EventTx,
}

/// See: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-powerbroadcast_setting>
#[derive(Debug, Clone, Copy, derive_more::Deref)]
struct PowerEvent(pub api::POWERBROADCAST_SETTING);

/// See: <https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes>
#[derive(Debug, Clone, Copy, derive_more::Deref)]
struct KeyCode(pub api::VIRTUAL_KEY);

/// See: [`WM_KEYDOWN`] and [`WM_KEYUP`].
///
/// [`WM_KEYDOWN`]: https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-keydown
/// [`WM_KEYUP`]: https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-keyup
#[derive(Debug, Clone, Copy, derive_more::Deref)]
struct KeyEventKind(pub u32);

/// See: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-kbdllhookstruct>
#[derive(Debug, Clone, Copy, derive_more::Deref)]
struct KeyEventContext(pub api::KBDLLHOOKSTRUCT);

#[derive(Debug, Clone, Copy)]
struct KeyEvent {
    #[allow(dead_code)]
    pub context: KeyEventContext,
    pub kind: KeyEventKind,
    pub code: KeyCode,
}

pub fn event_loop() {
    let mut msg = api::WindowsAndMessaging::MSG::default();

    unsafe {
        // Get a message from the window's event queue.
        // See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmessagew
        while api::WindowsAndMessaging::GetMessageW(&mut msg, None, 0, 0).into() {
            // Dispatch the received message.
            // See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-dispatchmessagew
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

    /// See: <https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getmodulehandlew>
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

    /// See: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerclassw>
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

    /// See: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-createwindowexw>
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

    /// See: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerpowersettingnotification>
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

    /// See: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw>
    fn new_key_hook(module: api::HMODULE) -> Result<api::HHOOK> {
        trace!("registering key hook...");

        unsafe {
            api::WindowsAndMessaging::SetWindowsHookExW(
                api::WindowsAndMessaging::WH_KEYBOARD_LL,
                Some(handle_low_level_key_event),
                module,
                0,
            )
            .context("failed to register key hook")
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        let inner = |window: &mut Self| -> Result<()> {
            // See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-postmessagew
            trace!("requesting the window be closed...");
            unsafe {
                api::WindowsAndMessaging::PostMessageW(
                    window.handle,
                    api::WindowsAndMessaging::WM_CLOSE,
                    api::WPARAM::default(),
                    api::LPARAM::default(),
                )?;
            };

            // See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-unregisterpowersettingnotification
            trace!("unregistering power notifications...");
            unsafe { api::Power::UnregisterPowerSettingNotification(window.power_notify)? };

            // See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-unhookwindowshookex
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

impl PowerEvent {
    /// See: <https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/ne-wdm-_monitor_display_state>
    fn state(&self) -> api::MONITOR_DISPLAY_STATE {
        api::MONITOR_DISPLAY_STATE(i32::from(self.Data[0]))
    }

    /// See: <https://learn.microsoft.com/en-us/windows/win32/power/power-setting-guids>
    fn target(&self) -> api::GUID {
        self.PowerSetting
    }
}

impl TryFrom<(api::WPARAM, api::LPARAM)> for KeyEvent {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: (api::WPARAM, api::LPARAM)) -> Result<Self> {
        let wparam = value.0;
        let lparam = value.1;

        let context = KeyEventContext::try_from(lparam)?;
        let kind = KeyEventKind::try_from(wparam)?;
        let code = context.key_code()?;

        Ok(Self {
            context,
            kind,
            code,
        })
    }
}

impl KeyEvent {
    fn to_owl_event(self) -> Option<owl::Event> {
        let owl_event = match *self.kind {
            api::WindowsAndMessaging::WM_KEYDOWN => owl::Event::Press,
            api::WindowsAndMessaging::WM_KEYUP => owl::Event::Release,
            _ => return None,
        };

        let result = match *self.code {
            api::KeyboardAndMouse::VK_VOLUME_DOWN => owl_event(owl::Key::VolumeDown),
            api::KeyboardAndMouse::VK_VOLUME_UP => owl_event(owl::Key::VolumeUp),
            api::KeyboardAndMouse::VK_VOLUME_MUTE => owl_event(owl::Key::VolumeMute),
            _ => owl::Event::Focus,
        };

        Some(result)
    }
}

impl KeyEventContext {
    fn key_code(&self) -> Result<KeyCode> {
        let inner =
            api::VIRTUAL_KEY(u16::try_from(self.vkCode).context("failed to convert key code")?);
        Ok(KeyCode(inner))
    }
}

impl TryFrom<api::LPARAM> for KeyEventContext {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: api::LPARAM) -> Result<Self> {
        #[allow(clippy::cast_sign_loss)]
        let event = ptr::with_exposed_provenance::<api::KBDLLHOOKSTRUCT>(value.0 as usize);
        if event.is_null() {
            return Err(eyre!("null key event"));
        }

        Ok(Self(unsafe { *event }))
    }
}

impl TryFrom<api::WPARAM> for KeyEventKind {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: api::WPARAM) -> Result<Self> {
        match u32::try_from(value.0) {
            Ok(x) => Ok(Self(x)),
            Err(e) => Err(eyre!("failed to convert key state: {e}")),
        }
    }
}

impl TryFrom<api::LPARAM> for PowerEvent {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: api::LPARAM) -> Result<Self> {
        #[allow(clippy::cast_sign_loss)]
        let power_settings =
            ptr::with_exposed_provenance::<api::POWERBROADCAST_SETTING>(value.0 as usize);

        if !power_settings.is_null() {
            return Err(eyre!("null power settings"));
        }

        Ok(Self(unsafe { *power_settings }))
    }
}

fn send_event(event_tx: &owl::EventTx, event: owl::Event) {
    trace!("relaying event: {event:?}");
    if let Err(e) = event_tx.send(event) {
        error!("failed to relay event: {event:?}: {e}");
    };
}

/// Our window event handler. This allows us listen to a variety of interesting
/// events, including power events.
///
/// See: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nc-winuser-wndproc>
extern "system" fn handle_window_event(
    window: api::HWND,
    msg: u32,
    wparam: api::WPARAM,
    lparam: api::LPARAM,
) -> api::LRESULT {
    // See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-defwindowprocw
    let defer = || unsafe { api::WindowsAndMessaging::DefWindowProcW(window, msg, wparam, lparam) };
    let ok = || api::LRESULT(0);
    let OwlHandle { event_tx } = get_owl_handle!(defer);

    let msg_params = match u32::try_from(wparam.0) {
        Ok(x) => x,
        Err(e) => {
            error!("failed to convert window message params: {e}");
            return defer();
        }
    };

    match msg {
        // The window should terminate.
        // See: https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-close
        api::WindowsAndMessaging::WM_CLOSE => {
            // See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-destroywindow
            trace!("received `WM_CLOSE` event, destroying window...");
            unsafe {
                api::WindowsAndMessaging::DestroyWindow(window).expect("failed to destroy window");
            };
            return ok();
        }

        // The window is being destroyed.
        // See: https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-destroy
        api::WindowsAndMessaging::WM_DESTROY => {
            // See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-postquitmessage
            trace!("received `WM_DESTROY` event, stopping event loop...");
            unsafe { api::WindowsAndMessaging::PostQuitMessage(0) };
            return ok();
        }

        // A power-management event has occurred.
        // See: https://learn.microsoft.com/en-us/windows/win32/power/wm-powerbroadcast
        api::WindowsAndMessaging::WM_POWERBROADCAST => match msg_params {
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

            // A power setting change occurred.
            // See: https://learn.microsoft.com/en-us/windows/win32/power/pbt-powersettingchange
            api::WindowsAndMessaging::PBT_POWERSETTINGCHANGE => {
                if let Ok(power_event) = PowerEvent::try_from(lparam)
                    // Check the current display is turning off.
                    && power_event.target() == api::SystemServices::GUID_CONSOLE_DISPLAY_STATE
                    && power_event.state() == api::SystemServices::PowerMonitorOff
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

/// Our low-level key event handler. As per the docs, it's important to do our
/// work as quickly as possible to avoid impacting system performance. We need
/// to use a low-level hook ([`WH_KEYBOARD_LL`]) as opposed to a normal hook
/// ([`WH_KEYBOARD`]) since it allow us to suppress certain keys being handled.
///
/// See: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw>
///
/// [`WH_KEYBOARD`]: https://learn.microsoft.com/en-us/windows/win32/winmsg/keyboardproc
/// [`WH_KEYBOARD_LL`]: https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc
extern "system" fn handle_low_level_key_event(
    ncode: i32,
    wparam: api::WPARAM,
    lparam: api::LPARAM,
) -> api::LRESULT {
    /// Indicates the event is a keyboard event.
    /// See: <https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc>
    #[allow(clippy::cast_possible_wrap)]
    const HC_ACTION: i32 = api::WindowsAndMessaging::HC_ACTION as i32;

    // See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-callnexthookex
    let defer = || unsafe { api::WindowsAndMessaging::CallNextHookEx(None, ncode, wparam, lparam) };
    let suppress = || api::LRESULT(1);

    // Bail if this isn't a keyboard event.
    if ncode < 0 || ncode != HC_ACTION {
        return defer();
    }

    let OwlHandle { event_tx } = get_owl_handle!(defer);
    match KeyEvent::try_from((wparam, lparam)) {
        Ok(key_event) => match key_event.to_owl_event() {
            // We got an event we care about!
            Some(owl_event) => {
                send_event(&event_tx, owl_event);

                // Unless volume events are suppressed, they'll operate as normal. This isn't
                // desirable since we're trying to replace software mixing with
                // hardware mixing. The software mixer works by reducing audio
                // bit-depth to make the audio quieter, at the expense of audio quality.
                match *key_event.code {
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
