use std::{mem::transmute, sync::OnceLock, thread};

use color_eyre::eyre::{eyre, Result};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, trace};
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
                CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
                GetMessageW, PostMessageW, PostQuitMessage, RegisterClassW, SetWindowsHookExW,
                UnhookWindowsHookEx, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT,
                DEVICE_NOTIFY_WINDOW_HANDLE, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, MSG,
                PBT_APMRESUMEAUTOMATIC, PBT_APMSUSPEND, PBT_POWERSETTINGCHANGE, WH_KEYBOARD_LL,
                WINDOW_EX_STYLE, WM_CLOSE, WM_DESTROY, WM_KEYDOWN, WM_KEYUP, WM_POWERBROADCAST,
                WNDCLASSW, WS_DISABLED,
            },
        },
    },
};

use super::Key;
use crate::{
    job::{RecvJob, SpawnResult},
    os::{Event, EventRx, EventTx},
    Spawn,
};

macro_rules! get_hook {
    ($on_err:expr) => {
        match HOOK.get() {
            Some(x) => Hook {
                event_tx: x.event_tx.clone(),
            },
            None => {
                error!("hook unset");
                return { $on_err() };
            }
        }
    };
}

static HOOK: OnceLock<Hook> = OnceLock::new();

pub struct Job {
    event_rx: EventRx,
}

struct Hook {
    event_tx: EventTx,
}

#[derive(Debug)]
struct Window {
    window: HWND,
    key_hook: HHOOK,
    power_notify: HPOWERNOTIFY,
}

#[derive(Debug, derive_more::Deref)]
struct PowerSettings(pub POWERBROADCAST_SETTING);

#[derive(Debug, derive_more::Deref)]
struct KeyEvent(pub KBDLLHOOKSTRUCT);

#[derive(Debug, derive_more::Deref)]
struct KeyState(pub u32);

impl Spawn for Job {
    async fn spawn(run_token: CancellationToken) -> SpawnResult<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel::<Event>();
        let (window_tx, window_rx) = oneshot::channel::<Window>();

        debug!("spawning os job...");
        let join_handle = thread::spawn(move || {
            debug!("os job started!");

            // Windows will get mad if you try to use resources outside the thread it was created.
            // Fortunately, the `Drop` implementation sidesteps this with message passing. So,
            // create the window in the job thread then send it back to async land.
            let window = Window::new(event_tx.clone())?;
            window_tx
                .send(window)
                .map_err(|_| eyre!("failed to send window"))?;
            debug!("os job ready!");

            event_loop();

            Result::Ok(())
        });

        // Dropping the `Window` will stop the event loop, saving us having to poll.
        let window = window_rx.await?;
        let _watchdog = tokio::spawn(async move {
            run_token.cancelled().await;
            drop(window);
        });

        Ok((join_handle, Self { event_rx }))
    }
}

impl RecvJob<Event> for Job {
    async fn recv(&mut self) -> Result<Event> {
        self.event_rx
            .recv()
            .await
            .ok_or_else(|| eyre!("event rx closed"))
    }
}

fn event_loop() {
    let mut message = MSG::default();
    unsafe {
        while GetMessageW(&mut message, None, 0, 0).into() {
            DispatchMessageW(&message);
        }
    }
}

impl Window {
    const WINDOW_CLASS: PCWSTR = w!("window");

    pub fn new(event_tx: EventTx) -> Result<Self> {
        HOOK.set(Hook { event_tx })
            .map_err(|_| eyre!("failed to set hook"))?;

        debug!("creating window...");
        let module = Self::module_handle()?;
        let _window_class = Self::new_window_class(module)?;
        let window = Self::new_window(module)?;
        let key_hook = Self::new_key_hook(module)?;
        let power_notify = Self::new_power_notify(window)?;
        debug!("window created!");

        Ok(Self {
            window,
            key_hook,
            power_notify,
        })
    }

    fn module_handle() -> Result<HMODULE> {
        debug!("getting module handle...");
        let module = unsafe { GetModuleHandleW(None)? };
        if (module.0 as *const isize).is_null() {
            return Err(eyre!("failed to get module handle"));
        }
        Ok(module)
    }

    fn new_window_class(module: HMODULE) -> Result<WNDCLASSW> {
        debug!("registering window class...");
        let window_class = WNDCLASSW {
            hInstance: module.into(),
            style: CS_HREDRAW | CS_VREDRAW,
            lpszClassName: Self::WINDOW_CLASS,
            lpfnWndProc: Some(handle_window_event),
            ..Default::default()
        };

        let atom = unsafe { RegisterClassW(&window_class) };
        if atom == 0 {
            return Err(eyre!("failed to register class"));
        }

        Ok(window_class)
    }

    fn new_window(module: HMODULE) -> Result<HWND> {
        debug!("creating window...");
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                Self::WINDOW_CLASS,
                w!("owl"),
                WS_DISABLED,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                module,
                None,
            )
        };

        if (window.0 as *const isize).is_null() {
            return Err(eyre!("failed to create window"));
        }

        Ok(window)
    }

    fn new_power_notify(window: HWND) -> Result<HPOWERNOTIFY> {
        debug!("registering for power notifications...");
        Ok(unsafe {
            RegisterPowerSettingNotification(
                window,
                &GUID_CONSOLE_DISPLAY_STATE,
                DEVICE_NOTIFY_WINDOW_HANDLE,
            )
        }?)
    }

    fn new_key_hook(module: HMODULE) -> Result<HHOOK> {
        debug!("registering key event hook...");
        unsafe {
            Ok(SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(handle_key_event),
                module,
                0,
            )?)
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        fn inner(window: &mut Window) -> Result<()> {
            unsafe {
                PostMessageW(
                    window.window,
                    WM_CLOSE,
                    WPARAM::default(),
                    LPARAM::default(),
                )?;
                UnregisterPowerSettingNotification(window.power_notify)?;
                UnhookWindowsHookEx(window.key_hook)?;
                Ok(())
            }
        }

        if let Err(e) = inner(self) {
            error!("failed to drop window: {e}");
        }
    }
}

fn send_event(event_tx: EventTx, event: Event) {
    trace!("received event: {event:?}");
    if let Err(e) = event_tx.send(event) {
        error!("failed to send event {event:?}: {e}");
    };
}

fn key_to_event(key_code: VIRTUAL_KEY, key_state: KeyState) -> Option<Event> {
    let event = match *key_state {
        WM_KEYDOWN => Event::Press,
        WM_KEYUP => Event::Release,
        _ => return None,
    };

    match key_code {
        VK_VOLUME_DOWN => Some(event(Key::VolumeDown)),
        VK_VOLUME_UP => Some(event(Key::VolumeUp)),
        VK_VOLUME_MUTE => Some(event(Key::VolumeMute)),
        _ => Some(Event::Focus),
    }
}

impl TryFrom<LPARAM> for PowerSettings {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: LPARAM) -> Result<PowerSettings> {
        let power_settings = unsafe { transmute::<LPARAM, *const POWERBROADCAST_SETTING>(value) };
        if !power_settings.is_null() {
            return Err(eyre!("null power settings"));
        }

        Ok(PowerSettings(unsafe { *power_settings }))
    }
}

impl TryFrom<LPARAM> for KeyEvent {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: LPARAM) -> std::prelude::v1::Result<Self, Self::Error> {
        let event = unsafe { transmute::<LPARAM, *const KBDLLHOOKSTRUCT>(value) };
        if event.is_null() {
            return Err(eyre!("null key event"));
        }

        Ok(KeyEvent(unsafe { *event }))
    }
}

extern "system" fn handle_window_event(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    const DISPLAY_OFF: u8 = 0;

    let defer = || unsafe { DefWindowProcW(window, message, wparam, lparam) };
    let ok = || LRESULT(0);
    let Hook { event_tx } = get_hook!(defer);

    match message {
        WM_CLOSE => {
            trace!("got WM_CLOSE");
            unsafe { DestroyWindow(window).expect("failed to destroy window") };
            return ok();
        }
        WM_DESTROY => {
            trace!("got WM_DESTROY");
            unsafe { PostQuitMessage(0) };
            return ok();
        }
        WM_POWERBROADCAST => match wparam.0 as u32 {
            PBT_APMRESUMEAUTOMATIC => send_event(event_tx, Event::Resume),
            PBT_APMSUSPEND => send_event(event_tx, Event::Suspend),
            PBT_POWERSETTINGCHANGE => {
                if let Ok(x) = PowerSettings::try_from(lparam)
                    && x.PowerSetting == GUID_CONSOLE_DISPLAY_STATE
                    && x.Data[0] == DISPLAY_OFF
                {
                    send_event(event_tx, Event::Suspend)
                }
            }
            _ => {}
        },
        _ => {}
    };

    defer()
}

extern "system" fn handle_key_event(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let defer = || unsafe { CallNextHookEx(None, ncode, wparam, lparam) };
    let suppress = || LRESULT(1);

    // Bail if this isn't a keyboard event.
    if ncode < 0 || ncode != HC_ACTION as i32 {
        return defer();
    }

    let Hook { event_tx } = get_hook!(defer);
    let key_event = match KeyEvent::try_from(lparam) {
        Ok(x) => x,
        Err(e) => {
            error!("failed to convert key event: {e}");
            return defer();
        }
    };

    let key_code = VIRTUAL_KEY(key_event.vkCode as _);
    let key_state = KeyState(wparam.0 as u32);

    let event = match key_to_event(key_code, key_state) {
        Some(x) => x,
        None => return defer(),
    };

    match key_code {
        VK_VOLUME_DOWN | VK_VOLUME_UP | VK_VOLUME_MUTE => {
            send_event(event_tx, event);
            suppress()
        }
        _ => {
            send_event(event_tx, event);
            defer()
        }
    }
}

unsafe impl Send for Window {}
