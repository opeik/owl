use super::{Event, EventRx, EventTx};
use ::windows::{
    core::w,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::ValidateRect,
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP},
            WindowsAndMessaging::{
                CallNextHookEx, CreateWindowExW, DefWindowProcW, DispatchMessageW, PostQuitMessage,
                RegisterClassW, SetWindowsHookExW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT,
                HC_ACTION, KBDLLHOOKSTRUCT, MSG, PBT_APMRESUMEAUTOMATIC, PBT_APMSUSPEND,
                WH_KEYBOARD_LL, WINDOW_EX_STYLE, WM_DESTROY, WM_KEYDOWN, WM_PAINT,
                WM_POWERBROADCAST, WNDCLASSW, WS_DISABLED,
            },
        },
    },
};
use color_eyre::eyre::{eyre, Result};
use std::{
    hash::{Hash, Hasher},
    sync::{Arc, OnceLock, RwLock},
    thread::{self, JoinHandle},
    time::Duration,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::HMODULE,
        System::{
            Power::{RegisterPowerSettingNotification, POWERBROADCAST_SETTING},
            SystemServices::GUID_CONSOLE_DISPLAY_STATE,
        },
        UI::WindowsAndMessaging::{
            PeekMessageW, DEVICE_NOTIFY_WINDOW_HANDLE, HHOOK, PBT_POWERSETTINGCHANGE, PM_REMOVE,
        },
    },
};

static JOB_DATA: OnceLock<Arc<RwLock<JobData>>> = OnceLock::new();

pub struct Job {
    event_rx: EventRx,
}

struct JobData {
    event_tx: EventTx,
}

#[allow(dead_code)]
struct Window {
    module: HMODULE,
    window: HWND,
    key_hook: HHOOK,
}

#[derive(Eq, PartialEq)]
struct WindowHandle(pub HWND);

impl Hash for WindowHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0 .0.hash(state);
    }
}

impl super::Spawn for Job {
    fn spawn(cancel_token: CancellationToken) -> (JoinHandle<Result<()>>, Self) {
        let (event_tx, event_rx) = mpsc::channel::<Event>(32);

        debug!("spawning os job...");
        let handle = thread::spawn(move || {
            debug!("os job started!");
            let _window = Window::new(event_tx.clone())?;
            debug!("os job ready!");

            loop {
                if cancel_token.is_cancelled() {
                    debug!("stopping os job...");
                    break;
                }

                poll_window_event();
                std::thread::sleep(Duration::from_micros(100));
            }

            Ok(())
        });

        (handle, Self { event_rx })
    }

    async fn recv_event(&mut self) -> Result<Event> {
        self.event_rx
            .recv()
            .await
            .ok_or_else(|| eyre!("event rx closed"))
    }
}

fn poll_window_event() {
    let mut message = MSG::default();
    let found_msg = unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.into();
    if found_msg {
        unsafe { DispatchMessageW(&message) };
    }
}

impl Window {
    fn new(event_tx: EventTx) -> Result<Self> {
        const WINDOW_CLASS: PCWSTR = w!("window");

        debug!("creating window...");
        debug!("getting module handle...");
        let module = unsafe { GetModuleHandleW(None)? };
        if module.0 == 0 {
            return Err(eyre!("failed to get module handle"));
        }

        debug!("registering window class...");
        let window_class = WNDCLASSW {
            hInstance: module.into(),
            style: CS_HREDRAW | CS_VREDRAW,
            lpszClassName: WINDOW_CLASS,
            lpfnWndProc: Some(handle_window_event),
            ..Default::default()
        };

        let atom = unsafe { RegisterClassW(&window_class) };
        if atom == 0 {
            return Err(eyre!("failed to register class"));
        }

        debug!("creating window...");
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                WINDOW_CLASS,
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

        let foo = unsafe {
            RegisterPowerSettingNotification(
                window,
                &GUID_CONSOLE_DISPLAY_STATE,
                DEVICE_NOTIFY_WINDOW_HANDLE.0,
            )
        }?;

        debug!("registering key event hook...");
        let key_hook =
            unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(handle_key_event), module, 0)? };

        JOB_DATA
            .set(Arc::new(RwLock::new(JobData { event_tx })))
            .map_err(|_| eyre!("failed to set window data"))?;

        debug!("window created!");
        Ok(Self {
            module,
            window,
            key_hook,
        })
    }
}

fn send_event(event_tx: EventTx, event: Event) {
    trace!("got event: {event}");
    if let Err(e) = event_tx.blocking_send(event) {
        error!("failed to send event {event}: {e}");
    };
}

extern "system" fn handle_window_event(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_PAINT => {
            unsafe { ValidateRect(window, None) };
            return LRESULT(0);
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            return LRESULT(0);
        }
        WM_POWERBROADCAST => {
            let window_data = match JOB_DATA.get() {
                Some(outer) => match outer.read() {
                    Ok(inner) => inner,
                    Err(e) => {
                        error!("failed to acquire window data lock: {e:?}");
                        return unsafe { DefWindowProcW(window, message, wparam, lparam) };
                    }
                },
                None => {
                    error!("window data uninitialized");
                    return unsafe { DefWindowProcW(window, message, wparam, lparam) };
                }
            };

            let event_tx = window_data.event_tx.clone();

            const DISPLAY_OFF: u8 = 0;
            match wparam.0 as u32 {
                PBT_APMRESUMEAUTOMATIC => send_event(event_tx, Event::Resume),
                PBT_APMSUSPEND => send_event(event_tx, Event::Suspend),
                PBT_POWERSETTINGCHANGE => {
                    let power_settings = unsafe {
                        std::mem::transmute::<LPARAM, *const POWERBROADCAST_SETTING>(lparam)
                    };

                    if !power_settings.is_null()
                        && let power_settings = unsafe { *power_settings }
                        && power_settings.PowerSetting == GUID_CONSOLE_DISPLAY_STATE
                        && power_settings.Data[0] == DISPLAY_OFF
                    {
                        send_event(event_tx, Event::Suspend)
                    }
                }
                _ => {}
            }
        }
        _ => {}
    };

    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

extern "system" fn handle_key_event(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if ncode < 0 || ncode != HC_ACTION as i32 {
        return unsafe { CallNextHookEx(None, ncode, wparam, lparam) };
    }

    let event = unsafe { std::mem::transmute::<LPARAM, *const KBDLLHOOKSTRUCT>(lparam) };
    let msg_sender_is_self = !event.is_null();
    if !msg_sender_is_self {
        return unsafe { CallNextHookEx(None, ncode, wparam, lparam) };
    }

    // Returning `LRESULT(1)` here "eats" the key event.
    if wparam.0 as u32 == WM_KEYDOWN {
        let key = VIRTUAL_KEY(unsafe { (*event).vkCode as _ });

        let window_data = match JOB_DATA.get() {
            Some(outer) => match outer.read() {
                Ok(inner) => inner,
                Err(e) => {
                    error!("failed to acquire window data lock: {e:?}");
                    return unsafe { CallNextHookEx(None, ncode, wparam, lparam) };
                }
            },
            None => {
                error!("window data uninitialized");
                return unsafe { CallNextHookEx(None, ncode, wparam, lparam) };
            }
        };

        let event_tx = window_data.event_tx.clone();

        match key {
            VK_VOLUME_UP => {
                send_event(event_tx, Event::VolumeUp);
                return LRESULT(1);
            }
            VK_VOLUME_DOWN => {
                send_event(event_tx, Event::VolumeDown);
                return LRESULT(1);
            }
            VK_VOLUME_MUTE => {
                send_event(event_tx, Event::VolumeMute);
                return LRESULT(1);
            }
            _ => send_event(event_tx, Event::UserActivity),
        }
    }

    unsafe { CallNextHookEx(None, ncode, wparam, lparam) }
}
