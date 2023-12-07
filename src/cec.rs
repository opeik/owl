use cec_rs::{
    CecCommand, CecConnection, CecConnectionCfgBuilder, CecDeviceType, CecDeviceTypeVec,
    CecKeypress, CecLogMessage, CecLogicalAddress, CecPowerStatus, CecVersion,
};
use color_eyre::{eyre::Context, Result};
use std::thread::{self, JoinHandle};
use tokio::sync::mpsc::{self, Sender};
use tracing::{debug, error, info, trace, warn};

use crate::os::Event;

#[derive(Debug, Clone, Copy, derive_more::Display)]
pub enum Command {
    PowerOn,
    PowerOff,
    VolumeUp,
    VolumeDown,
    VolumeMute,
}

#[derive(Debug, derive_more::Deref)]
pub struct Cec(CecConnection);

impl Cec {
    pub fn new() -> Result<Self> {
        debug!("configuring cec...");
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

        let cec = cfg.autodetect().context("failed to connect to cec")?;
        info!("connected to cec!");

        Ok(Self(cec))
    }
}

pub fn spawn_thread() -> (JoinHandle<Result<()>>, Sender<Command>) {
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<Command>(8);

    debug!("spawning cec thread...");
    let handle = thread::spawn(move || {
        debug!("cec thread started!");

        let cec = Cec::new()?;
        debug!("cec thread ready!");

        while let Some(cmd) = cmd_rx.blocking_recv() {
            debug!("sending command `{cmd}`...");

            match cmd {
                Command::PowerOn => cec.send_power_on_devices(CecLogicalAddress::Unregistered)?,
                Command::PowerOff => cec.send_standby_devices(CecLogicalAddress::Unregistered)?,
                Command::VolumeUp => cec.volume_up(true)?,
                Command::VolumeDown => cec.volume_down(true)?,
                Command::VolumeMute => cec.audio_toggle_mute()?,
            }
        }

        Ok(())
    });

    (handle, cmd_tx)
}

impl From<Event> for Command {
    fn from(value: Event) -> Self {
        match value {
            Event::Suspend => Command::PowerOff,
            Event::Resume => Command::PowerOn,
            Event::VolumeUp => Command::VolumeUp,
            Event::VolumeDown => Command::VolumeDown,
            Event::VolumeMute => Command::VolumeMute,
        }
    }
}

pub fn on_key_press(keypress: CecKeypress) {
    info!("got key: {:?}", keypress);
}

pub fn on_command_received(command: CecCommand) {
    info!("got cmd: {:?}", command);
}

pub fn on_log_level(log: CecLogMessage) {
    match log.level {
        cec_rs::CecLogLevel::Error => error!("{}", log.message),
        cec_rs::CecLogLevel::Warning => warn!("{}", log.message),
        cec_rs::CecLogLevel::Notice => info!("notice: {}", log.message),
        cec_rs::CecLogLevel::Traffic => trace!("{}", log.message),
        cec_rs::CecLogLevel::Debug => debug!("{}", log.message),
        cec_rs::CecLogLevel::All => info!("all: {}", log.message),
    }
}

// #[derive(Debug)]
// pub struct Device {
//     vendor: String,
//     name: String,
//     addr: String,
//     logical_addr: i32,
//     physical_addr: u16,
//     cec_version: CecVersion,
//     power_status: CecPowerStatus,
//     is_active: bool,
// }

// pub async fn print_devices(cec: Arc<Mutex<CecConnection>>) {
//     unsafe {
//         let cec = cec.lock().await;

//         let connection = cec.1;
//         let devices = libcec_sys::libcec_get_active_devices(connection);

//         let mut parsed_devices = Vec::<Device>::new();
//         for (addr, does_exist_raw) in devices.addresses.iter().enumerate() {
//             let does_exist = *does_exist_raw != 0;
//             if !does_exist {
//                 continue;
//             }

//             let logical_addr = addr as i32;
//             info!("getting device {} info", logical_addr);

//             let mut vendor_buf = [0 as c_char; 64];
//             // Size from: https://github.com/Pulse-Eight/libcec/blob/bf5a97d7673033ef6228c63109f6baf2bdbe1a0c/include/cectypes.h#L900
//             let mut name_buf = [0 as c_char; 14];
//             let vendor_id = libcec_sys::libcec_get_device_vendor_id(connection, logical_addr);
//             libcec_sys::libcec_vendor_id_to_string(
//                 vendor_id,
//                 &mut vendor_buf as _,
//                 vendor_buf.len(),
//             );
//             let physical_addr =
//                 libcec_sys::libcec_get_device_physical_address(connection, logical_addr);
//             let is_active = libcec_sys::libcec_is_active_source(connection, logical_addr) != 0;
//             let cec_version = CecVersion::try_from(libcec_sys::libcec_get_device_cec_version(
//                 connection,
//                 logical_addr,
//             ))
//             .unwrap();
//             let power_status = CecPowerStatus::try_from(
//                 libcec_sys::libcec_get_device_power_status(connection, logical_addr),
//             )
//             .unwrap();
//             libcec_sys::libcec_get_device_osd_name(connection, logical_addr, &mut name_buf as _);

//             let vendor = CStr::from_ptr(vendor_buf.as_ptr()).to_owned();
//             let name = CStr::from_ptr(name_buf.as_ptr()).to_owned();

//             let addr = format!(
//                 "{}.{}.{}.{}",
//                 (physical_addr >> 12) & 0xF,
//                 (physical_addr >> 8) & 0xF,
//                 (physical_addr >> 4) & 0xF,
//                 physical_addr & 0xF
//             );

//             let device = Device {
//                 vendor: vendor.to_string_lossy().to_string(),
//                 name: name.to_string_lossy().to_string(),
//                 addr,
//                 logical_addr,
//                 physical_addr,
//                 cec_version,
//                 power_status,
//                 is_active,
//             };

//             parsed_devices.push(device)
//         }

//         info!("found devices: {:#?}", parsed_devices);
//     }
// }
