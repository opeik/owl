use std::{
    ffi::{c_char, CStr},
    sync::{Arc, OnceLock},
};

use cec_rs::{
    CecCommand, CecConnection, CecConnectionCfgBuilder, CecDeviceType, CecDeviceTypeVec,
    CecKeypress, CecLogMessage, CecLogicalAddress, CecPowerStatus, CecVersion,
};
use color_eyre::{
    eyre::{eyre, Context},
    Result,
};
use futures::executor::block_on;
use tokio::{
    sync::{
        mpsc::{self, Sender},
        Mutex, MutexGuard,
    },
    task,
};
use tracing::{debug, error, info, level_filters::LevelFilter, trace, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use windows::{
    core::*,
    Win32::Foundation::*,
    Win32::Graphics::Gdi::ValidateRect,
    Win32::System::LibraryLoader::GetModuleHandleA,
    Win32::UI::Input::KeyboardAndMouse::*,
    Win32::UI::Input::*,
    Win32::{Devices::HumanInterfaceDevice::*, UI::WindowsAndMessaging::*},
};

enum Event {
    Paint,
    Destroy,
    Suspend,
    Resume,
    VolumeUp,
    VolumeDown,
    ToggleMute,
}

#[derive(Debug)]
struct Device {
    vendor: String,
    name: String,
    addr: String,
    logical_addr: i32,
    physical_addr: u16,
    cec_version: CecVersion,
    power_status: CecPowerStatus,
    is_active: bool,
}

static EVENT_TX: OnceLock<Sender<Event>> = OnceLock::new();
static KEYBOARD_HOOK: OnceLock<HHOOK> = OnceLock::new();

async fn spawn_cec_task<F>(cec: Arc<Mutex<CecConnection>>, f: F) -> Result<()>
where
    F: FnOnce(MutexGuard<CecConnection>) -> Result<()> + std::marker::Send + 'static,
{
    let cec = cec.clone();
    task::spawn_blocking(|| async move {
        let cec_guard = cec.lock().await;
        f(cec_guard)
    })
    .await?
    .await
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing()?;

    let (tx, mut rx) = mpsc::channel::<Event>(32);
    EVENT_TX.set(tx).unwrap();

    task::spawn_blocking(move || {
        spawn_window().unwrap();
    });
    let cec = Arc::new(Mutex::new(init_cec()?));

    while let Some(message) = rx.recv().await {
        match message {
            Event::Suspend => {
                info!("suspend");
                spawn_cec_task(cec.clone(), |cec| {
                    cec.send_standby_devices(CecLogicalAddress::Unregistered)?;
                    Ok(())
                })
                .await?;
            }
            Event::Resume => {
                info!("resume");
                spawn_cec_task(cec.clone(), |cec| {
                    cec.send_power_on_devices(CecLogicalAddress::Unregistered)?;
                    Ok(())
                })
                .await?;
            }
            Event::Paint => info!("window paint"),
            Event::Destroy => info!("window destroy"),
            Event::VolumeUp => {
                info!("volume up");

                spawn_cec_task(cec.clone(), |cec| {
                    cec.volume_up(false)?;
                    Ok(())
                })
                .await?;
            }
            Event::VolumeDown => {
                info!("volume down");
                spawn_cec_task(cec.clone(), |cec| {
                    cec.volume_down(false)?;
                    Ok(())
                })
                .await?;
            }
            Event::ToggleMute => {
                info!("toggle mute");
                spawn_cec_task(cec.clone(), |cec| {
                    cec.audio_toggle_mute()?;
                    Ok(())
                })
                .await?;
            }
        }
    }

    error!("this shouldn't happen");

    Ok(())
}

fn init_tracing() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::registry()
        .with(fmt::layer().without_time())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::ERROR.into())
                .parse("owl=trace")?,
        )
        .init();
    Ok(())
}

fn init_cec() -> Result<CecConnection> {
    let cfg = CecConnectionCfgBuilder::default()
        .device_name("cec-rs".to_owned())
        .device_types(CecDeviceTypeVec::new(CecDeviceType::RecordingDevice))
        .activate_source(false)
        .key_press_callback(Box::new(on_key_press))
        .command_received_callback(Box::new(on_command_received))
        .log_message_callback(Box::new(on_log_level))
        .hdmi_port(2)
        .build()
        .context("invalid cec config")?;

    info!("connecting to cec adapter...");
    let cec = cfg
        .autodetect()
        .context("failed to connect to cec adapter")?;
    info!("connected!");

    info!("setting active source to tv");
    cec.set_active_source(CecDeviceType::Tv)?;

    // cec.set_active_source(CecDeviceType)?;
    // cec.send_standby_devices(CecLogicalAddress::Unregistered)?;

    Ok(cec)
}

async fn print_devices(cec: Arc<Mutex<CecConnection>>) {
    unsafe {
        let cec = cec.lock().await;

        let connection = cec.1;
        let devices = libcec_sys::libcec_get_active_devices(connection);

        let mut parsed_devices = Vec::<Device>::new();
        for (addr, does_exist_raw) in devices.addresses.iter().enumerate() {
            let does_exist = *does_exist_raw != 0;
            if !does_exist {
                continue;
            }

            let logical_addr = addr as i32;
            info!("getting device {} info", logical_addr);

            let mut vendor_buf = [0 as c_char; 64];
            // Size from: https://github.com/Pulse-Eight/libcec/blob/bf5a97d7673033ef6228c63109f6baf2bdbe1a0c/include/cectypes.h#L900
            let mut name_buf = [0 as c_char; 14];
            let vendor_id = libcec_sys::libcec_get_device_vendor_id(connection, logical_addr);
            libcec_sys::libcec_vendor_id_to_string(
                vendor_id,
                &mut vendor_buf as _,
                vendor_buf.len(),
            );
            let physical_addr =
                libcec_sys::libcec_get_device_physical_address(connection, logical_addr);
            let is_active = libcec_sys::libcec_is_active_source(connection, logical_addr) != 0;
            let cec_version = CecVersion::try_from(libcec_sys::libcec_get_device_cec_version(
                connection,
                logical_addr,
            ))
            .unwrap();
            let power_status = CecPowerStatus::try_from(
                libcec_sys::libcec_get_device_power_status(connection, logical_addr),
            )
            .unwrap();
            libcec_sys::libcec_get_device_osd_name(connection, logical_addr, &mut name_buf as _);

            let vendor = CStr::from_ptr(vendor_buf.as_ptr()).to_owned();
            let name = CStr::from_ptr(name_buf.as_ptr()).to_owned();

            let addr = format!(
                "{}.{}.{}.{}",
                (physical_addr >> 12) & 0xF,
                (physical_addr >> 8) & 0xF,
                (physical_addr >> 4) & 0xF,
                physical_addr & 0xF
            );

            let device = Device {
                vendor: vendor.to_string_lossy().to_string(),
                name: name.to_string_lossy().to_string(),
                addr,
                logical_addr,
                physical_addr,
                cec_version,
                power_status,
                is_active,
            };

            parsed_devices.push(device)
        }

        info!("found devices: {:#?}", parsed_devices);
    }
}

fn spawn_window() -> Result<()> {
    unsafe {
        let module = GetModuleHandleA(None)?;
        if module.0 == 0 {
            return Err(eyre!("failed to get module handle"));
        }

        let window_class = s!("window");

        let wc = WNDCLASSA {
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hInstance: module.into(),
            lpszClassName: window_class,

            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_handler),
            ..Default::default()
        };

        let atom = RegisterClassA(&wc);
        if atom == 0 {
            return Err(eyre!("failed to register class"));
        }

        let _window = CreateWindowExA(
            WINDOW_EX_STYLE::default(),
            window_class,
            s!("owl (crimes inside!)"),
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
        let hook = SetWindowsHookExA(WH_KEYBOARD_LL, Some(keyboard_handler), module, 0)?;
        KEYBOARD_HOOK.set(hook).unwrap();

        let mut message = MSG::default();
        while GetMessageA(&mut message, None, 0, 0).into() {
            DispatchMessageA(&message);
        }
    }

    Ok(())
}

fn on_key_press(keypress: CecKeypress) {
    info!("got key: {:?}", keypress);
}

fn on_command_received(command: CecCommand) {
    info!("got cmd: {:?}", command);
}

fn on_log_level(log: CecLogMessage) {
    match log.level {
        cec_rs::CecLogLevel::Error => error!("{}", log.message),
        cec_rs::CecLogLevel::Warning => warn!("{}", log.message),
        cec_rs::CecLogLevel::Notice => info!("notice: {}", log.message),
        cec_rs::CecLogLevel::Traffic => trace!("{}", log.message),
        cec_rs::CecLogLevel::Debug => debug!("{}", log.message),
        cec_rs::CecLogLevel::All => info!("all: {}", log.message),
    }
}

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

extern "system" fn window_handler(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let tx = EVENT_TX.get().expect("event tx uninitalized").clone();

    unsafe {
        match message {
            WM_PAINT => {
                send_event!(tx, Event::Paint);
                ValidateRect(window, None);
                LRESULT(0)
            }
            WM_DESTROY => {
                send_event!(tx, Event::Destroy);
                PostQuitMessage(0);
                LRESULT(0)
            }
            WM_POWERBROADCAST => {
                match wparam.0 as u32 {
                    PBT_APMRESUMEAUTOMATIC => send_event!(tx, Event::Resume),
                    PBT_APMSUSPEND => send_event!(tx, Event::Suspend),
                    _ => {}
                }

                DefWindowProcA(window, message, wparam, lparam)
            }
            _ => DefWindowProcA(window, message, wparam, lparam),
        }
    }
}

extern "system" fn keyboard_handler(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        let tx = EVENT_TX.get().expect("event tx uninitalized").clone();
        let hook = KEYBOARD_HOOK.get().expect("keyboard hook uninitialized");

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
