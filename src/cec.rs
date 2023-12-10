use crate::os::Event;
use cec_rs::{
    CecCommand, CecConnection, CecConnectionCfgBuilder, CecDeviceType, CecDeviceTypeVec,
    CecKeypress, CecLogMessage, CecLogicalAddress, CecPowerStatus, CecVersion,
};
use color_eyre::eyre::{eyre, Context, Result};
use std::{
    collections::HashMap,
    ffi::{c_char, CStr},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use tokio::sync::mpsc::{self, Sender};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display)]
pub enum Command {
    PowerOn,
    PowerOff,
    VolumeUp,
    VolumeDown,
    VolumeMute,
}

pub struct Job {
    cmd_tx: Sender<Command>,
}

#[allow(dead_code)]
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

#[derive(Debug, derive_more::Deref)]
struct Cec(CecConnection);

impl Job {
    pub fn spawn(cancel_token: CancellationToken) -> (JoinHandle<Result<()>>, Self) {
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<Command>(8);

        debug!("spawning cec job...");
        let handle = thread::spawn(move || {
            debug!("cec job started!");

            let cancel_token = cancel_token;
            let cec = Cec::new()?;
            let mut last_cmd_times = HashMap::<Command, Instant>::new();

            debug!("cec job ready!");

            loop {
                if cancel_token.is_cancelled() {
                    debug!("stopping cec job...");
                    break;
                }

                if let Ok(cmd) = cmd_rx.try_recv() {
                    let time = Instant::now();

                    if let Some(last_cmd_time) = last_cmd_times.get_mut(&cmd) {
                        let delta_time = time.duration_since(*last_cmd_time);
                        // Volume up/down events fire continuously if the button is held.
                        // Debouncing prevents the channel and CEC bus from getting congested.
                        if (cmd == Command::VolumeDown || cmd == Command::VolumeUp)
                            && delta_time <= Duration::from_millis(200)
                        {
                            debug!("debouncing cmd {cmd}, delta: {delta_time:?}");
                            continue;
                        } else {
                            *last_cmd_time = time;
                        }
                    } else {
                        last_cmd_times.insert(cmd, time);
                    }

                    debug!("sending command: {cmd}");
                    match cmd {
                        Command::PowerOn => cec.set_active_source(CecDeviceType::PlaybackDevice)?,
                        Command::PowerOff => cec.send_standby_devices(CecLogicalAddress::Tv)?,
                        Command::VolumeUp => cec.volume_up(true)?,
                        Command::VolumeDown => cec.volume_down(true)?,
                        Command::VolumeMute => cec.audio_toggle_mute()?,
                    }
                }

                std::thread::sleep(Duration::from_micros(100));
            }

            Ok(())
        });

        (handle, Self { cmd_tx })
    }

    pub async fn send_cmd(&self, cmd: Command) -> Result<()> {
        Ok(self.cmd_tx.send(cmd).await?)
    }
}

impl Cec {
    pub fn new() -> Result<Self> {
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

        debug!("connecting to cec...");
        let cec = cfg.autodetect().context("failed to connect to cec")?;
        info!("connected to cec!");

        Ok(Self(cec))
    }

    pub fn print_devices(&self) -> Result<()> {
        let connection = self.1;
        let devices = unsafe { libcec_sys::libcec_get_active_devices(connection) };

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
            let vendor_id =
                unsafe { libcec_sys::libcec_get_device_vendor_id(connection, logical_addr) };
            unsafe {
                libcec_sys::libcec_vendor_id_to_string(
                    vendor_id,
                    &mut vendor_buf as _,
                    vendor_buf.len(),
                )
            };
            let physical_addr =
                unsafe { libcec_sys::libcec_get_device_physical_address(connection, logical_addr) };
            let is_active =
                unsafe { libcec_sys::libcec_is_active_source(connection, logical_addr) } != 0;
            let cec_version = CecVersion::try_from(unsafe {
                libcec_sys::libcec_get_device_cec_version(connection, logical_addr)
            })
            .map_err(|e| eyre!("failed to parse cec version: {e:?}"))?;
            let power_status = CecPowerStatus::try_from(unsafe {
                libcec_sys::libcec_get_device_power_status(connection, logical_addr)
            })
            .map_err(|e| eyre!("failed to parse power status: {e:?}"))?;
            unsafe {
                libcec_sys::libcec_get_device_osd_name(connection, logical_addr, &mut name_buf as _)
            };

            let vendor = unsafe { CStr::from_ptr(vendor_buf.as_ptr()).to_owned() };
            let name = unsafe { CStr::from_ptr(name_buf.as_ptr()).to_owned() };

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
        Ok(())
    }
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
    trace!("got: {:?}", keypress);
}

pub fn on_command_received(command: CecCommand) {
    trace!("got: {:?}", command);
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
