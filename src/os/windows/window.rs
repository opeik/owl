use crate::os::{
    windows::{HookResult, RawEvent},
    Event, EventRx, EventTx,
};
use color_eyre::eyre::{eyre, Result};
use std::sync::OnceLock;
use tokio::sync::mpsc;
use tracing::{debug, error};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{HMODULE, HWND, LPARAM, LRESULT, WPARAM},
        System::{
            LibraryLoader::GetModuleHandleW,
            Power::{
                RegisterPowerSettingNotification, UnregisterPowerSettingNotification, HPOWERNOTIFY,
                POWERBROADCAST_SETTING,
            },
            SystemServices::GUID_CONSOLE_DISPLAY_STATE,
        },
        UI::{
            Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP},
            WindowsAndMessaging::{
                CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW,
                SetWindowsHookExW, UnhookWindowsHookEx, UnregisterClassW, CS_HREDRAW, CS_VREDRAW,
                CW_USEDEFAULT, DEVICE_NOTIFY_WINDOW_HANDLE, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT,
                PBT_APMRESUMEAUTOMATIC, PBT_APMSUSPEND, PBT_POWERSETTINGCHANGE, WH_KEYBOARD_LL,
                WINDOW_EX_STYLE, WM_KEYDOWN, WM_POWERBROADCAST, WNDCLASSW, WS_DISABLED,
            },
        },
    },
};

static EVENT_HOOK: OnceLock<Hook> = OnceLock::new();

#[allow(dead_code)]
pub struct Window {
    window_class: WindowClassHandle,
    window: WindowHandle,
    hook: HookHandle,
    power_notifier: PowerNotifyHandle,
    event_tx: EventTx,
}

struct Hook {
    pub tx: EventTx,
    pub hook: fn(EventTx, RawEvent) -> Result<HookResult>,
}

struct ModuleHandle(HMODULE);
struct WindowHandle(HWND);
struct WindowClassHandle(WNDCLASSW);
struct HookHandle(HHOOK);
struct PowerNotifyHandle(HPOWERNOTIFY);

impl Window {
    const WINDOW_CLASS: PCWSTR = w!("window");

    pub fn new() -> Result<(Self, EventRx)> {
        debug!("creating window...");
        let (event_tx, event_rx) = mpsc::unbounded_channel::<Event>();

        let module = ModuleHandle::new()?;
        let window_class = WindowClassHandle::new(&module)?;
        let window = WindowHandle::new(&module)?;
        let hook = HookHandle::new(&module)?;
        let power_notifier = PowerNotifyHandle::new(&window)?;

        EVENT_HOOK
            .set(Hook {
                tx: event_tx.clone(),
                hook: event_hook,
            })
            .map_err(|_| eyre!("failed to set event hook"))?;
        debug!("window created!");

        Ok((
            Self {
                window_class,
                window,
                hook,
                power_notifier,
                event_tx,
            },
            event_rx,
        ))
    }

    pub fn window(&self) -> HWND {
        self.window.0
    }
}

impl ModuleHandle {
    pub fn new() -> Result<ModuleHandle> {
        debug!("getting module handle...");
        let module = unsafe { GetModuleHandleW(None)? };
        Ok(ModuleHandle(module))
    }
}

impl WindowClassHandle {
    pub fn new(module: &ModuleHandle) -> Result<WindowClassHandle> {
        debug!("registering window class...");
        let window_class = WNDCLASSW {
            hInstance: module.0.into(),
            style: CS_HREDRAW | CS_VREDRAW,
            lpszClassName: Window::WINDOW_CLASS,
            lpfnWndProc: Some(handle_window_event),
            ..Default::default()
        };

        let atom = unsafe { RegisterClassW(&window_class) };
        if atom == 0 {
            return Err(eyre!("failed to register class"));
        }

        Ok(WindowClassHandle(window_class))
    }
}

impl WindowHandle {
    pub fn new(module: &ModuleHandle) -> Result<WindowHandle> {
        debug!("creating window...");
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                Window::WINDOW_CLASS,
                w!("owl"),
                WS_DISABLED,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                module.0,
                None,
            )
        };

        if (window.0 as *const isize).is_null() {
            return Err(eyre!("failed to create window"));
        }

        Ok(WindowHandle(window))
    }
}

impl PowerNotifyHandle {
    pub fn new(window: &WindowHandle) -> Result<PowerNotifyHandle> {
        debug!("registering for power notifications...");
        let power_notifier = unsafe {
            RegisterPowerSettingNotification(
                window.0,
                &GUID_CONSOLE_DISPLAY_STATE,
                DEVICE_NOTIFY_WINDOW_HANDLE.0,
            )
        }?;

        Ok(PowerNotifyHandle(power_notifier))
    }
}

impl HookHandle {
    fn new(module: &ModuleHandle) -> Result<HookHandle> {
        debug!("registering key event hook...");
        let hook =
            unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(handle_key_event), module.0, 0)? };
        Ok(HookHandle(hook))
    }
}

impl Drop for WindowHandle {
    fn drop(&mut self) {
        let result = unsafe { DestroyWindow(self.0) };
        if let Err(e) = result {
            error!("failed to drop window handle: {e}");
        }
    }
}

impl Drop for WindowClassHandle {
    fn drop(&mut self) {
        let result = unsafe { UnregisterClassW(self.0.lpszClassName, self.0.hInstance) };
        if let Err(e) = result {
            error!("failed to drop window class handle: {e}");
        }
    }
}

impl Drop for PowerNotifyHandle {
    fn drop(&mut self) {
        let result = unsafe { UnregisterPowerSettingNotification(self.0) };
        if let Err(e) = result {
            error!("failed to drop power notify handle: {e}");
        }
    }
}

impl Drop for HookHandle {
    fn drop(&mut self) {
        let result = unsafe { UnhookWindowsHookEx(self.0) };
        if let Err(e) = result {
            error!("failed to drop power hook handle: {e}");
        }
    }
}

fn event_hook(event_tx: EventTx, raw_event: RawEvent) -> Result<HookResult> {
    const DISPLAY_TURNED_OFF: u8 = 0;
    let event = match raw_event {
        RawEvent::Resume => Some(Event::Resume),
        RawEvent::Suspend => Some(Event::Suspend),
        RawEvent::PowerSettingChange(settings)
            if settings.PowerSetting == GUID_CONSOLE_DISPLAY_STATE
                && settings.Data[0] == DISPLAY_TURNED_OFF =>
        {
            Some(Event::Suspend)
        }
        RawEvent::KeyDown(key) => match key {
            VK_VOLUME_UP => Some(Event::VolumeUp),
            VK_VOLUME_DOWN => Some(Event::VolumeDown),
            VK_VOLUME_MUTE => Some(Event::VolumeMute),
            _ => None,
        },
        _ => None,
    };

    if let Some(event) = event {
        event_tx.send(event)?;
    }

    Ok(HookResult::Forward)
}

macro_rules! get_hook {
    ($on_err:expr) => {
        match EVENT_HOOK.get() {
            Some(event_hook) => {
                let Hook { tx, hook } = event_hook;
                let tx = tx.to_owned();
                let hook = hook.to_owned();
                Hook { tx, hook }
            }
            None => {
                error!("window hook uninitialized");
                return { $on_err() };
            }
        }
    };
}

// See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nc-winuser-wndproc
extern "system" fn handle_window_event(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let forward_event = || unsafe { DefWindowProcW(window, message, wparam, lparam) };
    let Hook { tx, hook } = get_hook!(forward_event);

    let result = match message {
        WM_POWERBROADCAST => match wparam.0 as u32 {
            PBT_APMRESUMEAUTOMATIC => hook(tx, RawEvent::Resume),
            PBT_APMSUSPEND => hook(tx, RawEvent::Suspend),
            PBT_POWERSETTINGCHANGE => {
                let power_settings = unsafe {
                    *std::mem::transmute::<LPARAM, *const POWERBROADCAST_SETTING>(lparam)
                };
                hook(tx, RawEvent::PowerSettingChange(power_settings))
            }
            _ => Ok(HookResult::Forward),
        },
        _ => Ok(HookResult::Forward),
    };

    if let Ok(x) = result
        && x == HookResult::Suppress
    {
        LRESULT(1)
    } else {
        forward_event()
    }
}

// See: https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc#code-in
extern "system" fn handle_key_event(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let forward_event = || unsafe { CallNextHookEx(None, ncode, wparam, lparam) };
    let Hook { tx, hook } = get_hook!(forward_event);

    let result = match ncode as u32 {
        // Keyboard event.
        HC_ACTION => {
            let event = unsafe { std::mem::transmute::<LPARAM, *const KBDLLHOOKSTRUCT>(lparam) };
            if !event.is_null() {
                Ok(HookResult::Forward)
            } else {
                match wparam.0 as u32 {
                    WM_KEYDOWN => {
                        let key = VIRTUAL_KEY(unsafe { (*event).vkCode as _ });
                        hook(tx, RawEvent::KeyDown(key))
                    }
                    _ => Ok(HookResult::Forward),
                }
            }
        }
        _ => Ok(HookResult::Forward),
    };

    if let Ok(x) = result
        && x == HookResult::Suppress
    {
        LRESULT(1)
    } else {
        forward_event()
    }
}

unsafe impl Send for Window {}
