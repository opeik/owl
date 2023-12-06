use color_eyre::{eyre::eyre, Result};
use futures::executor::block_on;
use tracing::error;
use windows::{
    core::w,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::ValidateRect,
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP},
            WindowsAndMessaging::{
                CallNextHookEx, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW,
                LoadCursorW, PostQuitMessage, RegisterClassW, SetWindowsHookExW, CS_HREDRAW,
                CS_VREDRAW, CW_USEDEFAULT, HC_ACTION, IDC_ARROW, KBDLLHOOKSTRUCT, MSG,
                PBT_APMRESUMEAUTOMATIC, PBT_APMSUSPEND, WH_KEYBOARD_LL, WINDOW_EX_STYLE,
                WM_DESTROY, WM_KEYDOWN, WM_PAINT, WM_POWERBROADCAST, WNDCLASSW, WS_DISABLED,
            },
        },
    },
};

use crate::cec::Event;

macro_rules! send_event {
    ($tx:expr, $event:expr) => {
        match block_on($tx.send($event)) {
            Ok(_) => {}
            Err(e) => {
                error!("failed to send event: {}", e);
            }
        }
    };
}

pub fn spawn_window() -> Result<()> {
    unsafe {
        let module = GetModuleHandleW(None)?;
        if module.0 == 0 {
            return Err(eyre!("failed to get module handle"));
        }

        let window_class = w!("window");

        let wc = WNDCLASSW {
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hInstance: module.into(),
            lpszClassName: window_class,

            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_handler),
            ..Default::default()
        };

        let atom = RegisterClassW(&wc);
        if atom == 0 {
            return Err(eyre!("failed to register class"));
        }

        let _window = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            window_class,
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
        );

        // begin the crimes
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_handler), module, 0)?;
        crate::KEYBOARD_HOOK.set(hook).unwrap();

        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).into() {
            DispatchMessageW(&message);
        }
    }

    Ok(())
}

extern "system" fn window_handler(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let tx = crate::EVENT_TX
        .get()
        .expect("event tx uninitalized")
        .clone();

    unsafe {
        match message {
            WM_PAINT => {
                ValidateRect(window, None);
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            WM_POWERBROADCAST => {
                match wparam.0 as u32 {
                    PBT_APMRESUMEAUTOMATIC => send_event!(tx, Event::Resume),
                    PBT_APMSUSPEND => send_event!(tx, Event::Suspend),
                    _ => {}
                }

                DefWindowProcW(window, message, wparam, lparam)
            }
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }
}

extern "system" fn keyboard_handler(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        let tx = crate::EVENT_TX
            .get()
            .expect("event tx uninitalized")
            .clone();
        let hook = crate::KEYBOARD_HOOK
            .get()
            .expect("keyboard hook uninitialized");

        if ncode < 0 || ncode != HC_ACTION as i32 {
            return CallNextHookEx(*hook, ncode, wparam, lparam);
        }

        let event = std::mem::transmute::<LPARAM, *const KBDLLHOOKSTRUCT>(lparam);
        if wparam.0 as u32 == WM_KEYDOWN {
            match VIRTUAL_KEY((*event).vkCode as _) {
                VK_VOLUME_UP => {
                    send_event!(tx, Event::VolumeUp);
                    return LRESULT(1);
                }
                VK_VOLUME_DOWN => {
                    send_event!(tx, Event::VolumeDown);
                    return LRESULT(1);
                }
                VK_VOLUME_MUTE => {
                    send_event!(tx, Event::ToggleMute);
                    return LRESULT(1);
                }
                _ => {}
            }
        }

        CallNextHookEx(*hook, ncode, wparam, lparam)
    }
}
