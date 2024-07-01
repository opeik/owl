use std::{ptr, sync::OnceLock, thread};

use color_eyre::eyre::{eyre, Context, Result};
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
            SystemServices,
        },
        UI::{
            Input::KeyboardAndMouse::{self, VIRTUAL_KEY},
            WindowsAndMessaging::{
                self, CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow,
                DispatchMessageW, GetMessageW, PostMessageW, PostQuitMessage, RegisterClassW,
                SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, WINDOW_EX_STYLE,
                WNDCLASSW,
            },
        },
    },
};

use super::Key;
use crate::{
    job::{self, Recv, SpawnResult},
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

// I hate global, mutable state as much as you do, but we have no other options. There doesn't appear
// to be another way to smuggle a reference to our code from win32 land.
static HOOK: OnceLock<Hook> = OnceLock::new();

/// Represents a Windows OS job, responsible for sending and receiving Windows events.
pub struct Job {
    event_rx: EventRx,
}

struct Hook {
    event_tx: EventTx,
}

#[derive(Debug)]
struct Window {
    handle: HWND,
    key_hook: HHOOK,
    power_notify: HPOWERNOTIFY,
}

#[derive(Debug, Clone, Copy, derive_more::Deref)]
struct PowerSettings(pub POWERBROADCAST_SETTING);

#[derive(Debug, Clone, Copy, derive_more::Deref)]
struct KeyEvent(pub KBDLLHOOKSTRUCT);

#[derive(Debug, Clone, Copy, derive_more::Deref)]
struct KeyState(pub u32);

impl Spawn for Job {
    /// Spawns a new Windows job. The job runs on a thread.
    async fn spawn(run_token: CancellationToken) -> SpawnResult<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel::<Event>();
        let (window_tx, window_rx) = oneshot::channel::<Window>();
        let (ready_tx, ready_rx) = oneshot::channel::<Result<()>>();

        trace!("spawning os job...");
        let join_handle = thread::spawn(move || {
            debug!("os job starting...");

            // Windows will get mad if you try to use resources outside the thread that created it.
            // Fortunately, the `Drop` implementation sidesteps this with message passing. So,
            // create the window in the job thread then send it back to async land.
            job::send_ready_status(ready_tx, || match Window::new(event_tx.clone()) {
                Ok(x) => {
                    trace!("sending window handle to task...");
                    window_tx
                        .send(x)
                        .map_err(|_| eyre!("failed to send window handle to task"))
                }
                Err(e) => Err(e),
            })?;

            event_loop();
            Result::Ok(())
        });

        let window = window_rx.await?;
        trace!("received window handle from thread!");

        ready_rx
            .await
            .context("failed to read job status")?
            .context("job failed to start")?;
        debug!("os job ready!");

        // Dropping the `Window` will stop the event loop, saving us having to poll.
        let _watchdog = tokio::spawn(async move {
            run_token.cancelled().await;
            drop(window);
        });

        Ok((join_handle, Self { event_rx }))
    }
}

impl Recv<Event> for Job {
    async fn recv(&mut self) -> Result<Event> {
        self.event_rx
            .recv()
            .await
            .ok_or_else(|| eyre!("event rx closed"))
    }
}

fn event_loop() {
    let mut message = WindowsAndMessaging::MSG::default();
    // TODO: there's _got_ to be a better way to do this
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

    fn module_handle() -> Result<HMODULE> {
        trace!("getting module handle...");
        let module = unsafe { GetModuleHandleW(None).context("failed to get module handle")? };
        if module.is_invalid() {
            return Err(eyre!("failed to get module handle"));
        }
        Ok(module)
    }

    fn new_window_class(module: HMODULE) -> Result<WNDCLASSW> {
        trace!("registering window class...");
        let window_class = WNDCLASSW {
            hInstance: module.into(),
            style: WindowsAndMessaging::CS_HREDRAW | WindowsAndMessaging::CS_VREDRAW,
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
        trace!("creating window...");
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                Self::WINDOW_CLASS,
                w!("owl"),
                WindowsAndMessaging::WS_DISABLED,
                WindowsAndMessaging::CW_USEDEFAULT,
                WindowsAndMessaging::CW_USEDEFAULT,
                WindowsAndMessaging::CW_USEDEFAULT,
                WindowsAndMessaging::CW_USEDEFAULT,
                None,
                None,
                module,
                None,
            )
        };

        if ptr::with_exposed_provenance::<usize>(window.0 as usize).is_null() {
            return Err(eyre!("failed to create window"));
        }

        Ok(window)
    }

    fn new_power_notify(window: HWND) -> Result<HPOWERNOTIFY> {
        trace!("registering for power notifications...");
        unsafe {
            RegisterPowerSettingNotification(
                window,
                &SystemServices::GUID_CONSOLE_DISPLAY_STATE,
                WindowsAndMessaging::DEVICE_NOTIFY_WINDOW_HANDLE,
            )
            .context("failed to register power notifications")
        }
    }

    fn new_key_hook(module: HMODULE) -> Result<HHOOK> {
        trace!("registering key event hook...");
        unsafe {
            SetWindowsHookExW(
                WindowsAndMessaging::WH_KEYBOARD_LL,
                Some(handle_key_event),
                module,
                0,
            )
            .context("failed to register keyboard hook")
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        fn inner(window: &mut Window) -> Result<()> {
            unsafe {
                PostMessageW(
                    window.handle,
                    WindowsAndMessaging::WM_CLOSE,
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

fn send_event(event_tx: &EventTx, event: Event) {
    trace!("relaying event: `{event:?}`");
    if let Err(e) = event_tx.send(event) {
        error!("failed to relay event `{event:?}`: {e}");
    };
}

impl TryFrom<LPARAM> for PowerSettings {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: LPARAM) -> Result<PowerSettings> {
        let power_settings =
            ptr::with_exposed_provenance::<POWERBROADCAST_SETTING>(value.0 as usize);
        if !power_settings.is_null() {
            return Err(eyre!("null power settings"));
        }

        Ok(PowerSettings(unsafe { *power_settings }))
    }
}

impl TryFrom<LPARAM> for KeyEvent {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: LPARAM) -> Result<Self, Self::Error> {
        let event = ptr::with_exposed_provenance::<KBDLLHOOKSTRUCT>(value.0 as usize);
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
        WindowsAndMessaging::WM_CLOSE => {
            trace!("received `WM_CLOSE` event, destroying window...");
            unsafe { DestroyWindow(window).expect("failed to destroy window") };
            return ok();
        }
        // The window is being destroyed.
        // See: https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-destroy
        WindowsAndMessaging::WM_DESTROY => {
            trace!("received `WM_DESTROY` event, stopping event loop...");
            unsafe { PostQuitMessage(0) };
            return ok();
        }

        // A power-management event has occurred.
        // See: https://learn.microsoft.com/en-us/windows/win32/power/wm-powerbroadcast
        WindowsAndMessaging::WM_POWERBROADCAST => match message_params {
            // The system is resuming from sleep.
            // See: https://learn.microsoft.com/en-us/windows/win32/power/pbt-apmresumeautomatic
            WindowsAndMessaging::PBT_APMRESUMEAUTOMATIC => send_event(&event_tx, Event::Resume),

            // The system is about to sleep.
            // See: https://learn.microsoft.com/en-us/windows/win32/power/pbt-apmsuspend
            WindowsAndMessaging::PBT_APMSUSPEND => send_event(&event_tx, Event::Suspend),

            // Power setting change occurred.
            // See: https://learn.microsoft.com/en-us/windows/win32/power/pbt-powersettingchange
            WindowsAndMessaging::PBT_POWERSETTINGCHANGE => {
                if let Ok(power_settings) = PowerSettings::try_from(lparam)
                    && let new_power_setting = power_settings.Data[0]
                    && let event_target = power_settings.PowerSetting
                    // The current monitor's display state has changed.
                    // See: https://learn.microsoft.com/en-us/windows/win32/power/power-setting-guids
                    && event_target == SystemServices::GUID_CONSOLE_DISPLAY_STATE
                    && new_power_setting == DISPLAY_OFF
                {
                    send_event(&event_tx, Event::Suspend);
                }
            }
            _ => {}
        },
        _ => {}
    };

    defer()
}

fn key_to_event(key_code: VIRTUAL_KEY, key_state: KeyState) -> Option<Event> {
    let event = match *key_state {
        WindowsAndMessaging::WM_KEYDOWN => Event::Press,
        WindowsAndMessaging::WM_KEYUP => Event::Release,
        _ => return None,
    };

    match key_code {
        KeyboardAndMouse::VK_VOLUME_DOWN => Some(event(Key::VolumeDown)),
        KeyboardAndMouse::VK_VOLUME_UP => Some(event(Key::VolumeUp)),
        KeyboardAndMouse::VK_VOLUME_MUTE => Some(event(Key::VolumeMute)),
        _ => Some(Event::Focus),
    }
}

extern "system" fn handle_key_event(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    /// Indicates the event is a keyboard event.
    /// See: https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc
    #[allow(clippy::cast_possible_wrap)]
    const HC_ACTION_I32: i32 = WindowsAndMessaging::HC_ACTION as i32;

    let defer = || unsafe { CallNextHookEx(None, ncode, wparam, lparam) };
    let suppress = || LRESULT(1);

    // Bail if this isn't a keyboard event.
    if ncode < 0 || ncode != HC_ACTION_I32 {
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

    let key_code = match u16::try_from(key_event.vkCode) {
        Ok(x) => VIRTUAL_KEY(x),
        Err(e) => {
            error!("failed to convert key code: {e}");
            return defer();
        }
    };

    let key_state = match u32::try_from(wparam.0) {
        Ok(x) => KeyState(x),
        Err(e) => {
            error!("failed to convert key state: {e}");
            return defer();
        }
    };

    let Some(event) = key_to_event(key_code, key_state) else {
        return defer();
    };

    send_event(&event_tx, event);

    // Unless volume events are suppressed, they'll operate as normal. This isn't desirable since
    // we're trying to replace software mixing with hardware mixing. The software mixer works
    // by reducing audio bit-depth to make the audio quieter, at the expense of audio quality.
    match key_code {
        KeyboardAndMouse::VK_VOLUME_DOWN
        | KeyboardAndMouse::VK_VOLUME_UP
        | KeyboardAndMouse::VK_VOLUME_MUTE => suppress(),
        _ => defer(),
    }
}

unsafe impl Send for Window {}
