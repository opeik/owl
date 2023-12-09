use super::Event;
use ::windows::{
    core::w,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::ValidateRect,
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP},
            WindowsAndMessaging::{
                CallNextHookEx, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW,
                PostQuitMessage, RegisterClassW, SetWindowsHookExW, CS_HREDRAW, CS_VREDRAW,
                CW_USEDEFAULT, HC_ACTION, KBDLLHOOKSTRUCT, MSG, PBT_APMRESUMEAUTOMATIC,
                PBT_APMSUSPEND, WH_KEYBOARD_LL, WINDOW_EX_STYLE, WM_DESTROY, WM_KEYDOWN, WM_PAINT,
                WM_POWERBROADCAST, WNDCLASSW, WS_DISABLED,
            },
        },
    },
};
use color_eyre::{eyre::eyre, Result};
use std::{
    sync::OnceLock,
    thread::{self, JoinHandle},
    time::Duration,
};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::HMODULE,
        UI::WindowsAndMessaging::{PeekMessageW, HHOOK, PM_REMOVE},
    },
};

static KEY_HOOK: OnceLock<HHOOK> = OnceLock::new();
static EVENT_TX: OnceLock<Sender<Event>> = OnceLock::new();

pub struct Job {
    event_rx: Receiver<Event>,
}

struct Window {
    module: HMODULE,
    window: HWND,
    key_hook: HHOOK,
}

impl super::Spawn for Job {
    fn spawn(cancel_token: CancellationToken) -> (JoinHandle<Result<()>>, Self) {
        let (event_tx, event_rx) = mpsc::channel::<Event>(32);
        EVENT_TX
            .set(event_tx)
            .expect("failed to set `os::event_tx`");

        debug!("spawning os job...");
        let handle = thread::spawn(move || {
            debug!("os job started!");
            let _window = Window::new()?;

            debug!("os job ready!");
            let mut message = MSG::default();

            loop {
                if cancel_token.is_cancelled() {
                    debug!("stopping os job...");
                    break;
                }

                let found_msg = unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.into();
                if found_msg {
                    unsafe { DispatchMessageW(&message) };
                }

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

impl Window {
    fn new() -> Result<Self> {
        const WINDOW_CLASS: PCWSTR = w!("window");

        debug!("getting module handle...");
        let module = unsafe { GetModuleHandleW(None)? };
        if module.0 == 0 {
            return Err(eyre!("failed to get module handle"));
        }

        let wc = WNDCLASSW {
            hInstance: module.into(),
            style: CS_HREDRAW | CS_VREDRAW,
            lpszClassName: WINDOW_CLASS,
            lpfnWndProc: Some(handle_window_event),
            ..Default::default()
        };

        debug!("registering class...");
        let atom = unsafe { RegisterClassW(&wc) };
        if atom == 0 {
            return Err(eyre!("failed to register class"));
        }

        debug!("creating window...");
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                WINDOW_CLASS,
                w!("owl (crimes inside!)"),
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

        debug!("registering key event hook...");
        let key_hook =
            unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(handle_key_event), module, 0)? };
        self::KEY_HOOK
            .set(key_hook)
            .expect("`os::key_hook` already set");

        Ok(Self {
            module,
            window,
            key_hook,
        })
    }
}

fn send_event(event_tx: Sender<Event>, event: Event) {
    debug!("got event: {event}");
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
    let event_tx = EVENT_TX.get().expect("`os::event_tx` unset").clone();

    match message {
        WM_PAINT => {
            unsafe { ValidateRect(window, None) };
            return LRESULT(0);
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            return LRESULT(0);
        }
        WM_POWERBROADCAST => match wparam.0 as u32 {
            PBT_APMRESUMEAUTOMATIC => send_event(event_tx, Event::Resume),
            PBT_APMSUSPEND => send_event(event_tx, Event::Suspend),
            _ => {}
        },
        _ => {}
    };

    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

extern "system" fn handle_key_event(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let event_tx = self::EVENT_TX.get().expect("`os::event_tx` unset").clone();
    let hook = self::KEY_HOOK.get().expect("`os::key_hook` unset");

    if ncode < 0 || ncode != HC_ACTION as i32 {
        return unsafe { CallNextHookEx(*hook, ncode, wparam, lparam) };
    }

    let event = unsafe { std::mem::transmute::<LPARAM, *const KBDLLHOOKSTRUCT>(lparam) };
    if event.is_null() {
        return unsafe { CallNextHookEx(*hook, ncode, wparam, lparam) };
    }

    // Returning `LRESULT(1)` here "eats" the key event.
    if wparam.0 as u32 == WM_KEYDOWN {
        let key = VIRTUAL_KEY(unsafe { (*event).vkCode as _ });
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
            _ => {}
        }
    }

    unsafe { CallNextHookEx(*hook, ncode, wparam, lparam) }
}
