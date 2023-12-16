use std::{
    collections::HashMap,
    thread,
    time::{Duration, Instant},
};

use cec_rs::{
    CecCommand, CecConnection, CecConnectionCfgBuilder, CecDeviceType, CecDeviceTypeVec,
    CecKeypress, CecLogMessage, CecLogicalAddress, CecPowerStatus, CecVersion,
};
use color_eyre::eyre::{Context, Result};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace, warn};

use crate::{
    job::{SendJob, SpawnResult},
    os::Event,
    Spawn,
};

pub type CommandTx = mpsc::Sender<Command>;
pub type CommandRx = mpsc::Receiver<Command>;
type LastCmd = HashMap<Command, Instant>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display)]
pub enum Command {
    PowerOn,
    PowerOff,
    VolumeUp,
    VolumeDown,
    VolumeMute,
    Focus,
}

pub struct Job {
    cmd_tx: CommandTx,
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

impl Command {
    const fn debounce_duration(&self) -> Option<Duration> {
        match self {
            // In my testing, 180ms was the shortest delay between repeated volume commands
            // that maintained CEC bus responsiveness.
            Command::VolumeUp | Command::VolumeDown => Some(Duration::from_millis(180)),
            Command::Focus => Some(Duration::from_secs(3)),
            _ => None,
        }
    }
}

impl Spawn for Job {
    async fn spawn(cancel_token: CancellationToken) -> SpawnResult<Self> {
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<Command>(8);

        debug!("spawning cec job...");
        let handle = thread::spawn(move || {
            debug!("cec job started!");

            let cancel_token = cancel_token;
            let cec = Cec::new()?;
            let mut last_cmd = LastCmd::new();
            debug!("cec job ready!");

            loop {
                if cancel_token.is_cancelled() {
                    debug!("stopping cec job...");
                    break;
                }

                handle_cmd(&cec, &mut cmd_rx, &mut last_cmd);
                std::thread::sleep(Duration::from_millis(1));
            }

            Ok(())
        });

        Ok((handle, Self { cmd_tx }))
    }
}

impl SendJob<Command> for Job {
    async fn send(&self, cmd: Command) -> Result<()> {
        Ok(self.cmd_tx.send(cmd).await?)
    }
}

fn handle_cmd(cec: &Cec, cmd_rx: &mut CommandRx, last_cmd: &mut LastCmd) {
    // Volume up/down events fire continuously if the button is held.
    // Debouncing prevents the channel and CEC bus from getting congested.
    if let Ok(cmd) = cmd_rx.try_recv()
        && let Some(cmd) = debounce_cmd(cmd, last_cmd)
    {
        debug!("sending command: {cmd}");
        let result = match cmd {
            Command::PowerOn => cec.set_active_source(CecDeviceType::PlaybackDevice),
            Command::PowerOff => cec.send_standby_devices(CecLogicalAddress::Tv),
            Command::VolumeUp => cec.volume_up(true),
            Command::VolumeDown => cec.volume_down(true),
            Command::VolumeMute => cec.audio_toggle_mute(),
            Command::Focus => cec.set_active_source(CecDeviceType::PlaybackDevice),
        };

        if let Err(e) = result {
            error!("failed to send cec command: {e}");
        }
    }
}

fn debounce_cmd(cmd: Command, time_by_cmd: &mut HashMap<Command, Instant>) -> Option<Command> {
    let time = Instant::now();

    if let Some(last_time) = time_by_cmd.get_mut(&cmd) {
        let delta = time.duration_since(*last_time);
        if let Some(duration) = cmd.debounce_duration()
            && delta <= duration
        {
            return None;
        } else {
            *last_time = time;
        }
    } else {
        time_by_cmd.insert(cmd, time);
    }

    Some(cmd)
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
}

impl From<Event> for Command {
    fn from(value: Event) -> Self {
        match value {
            Event::Suspend => Command::PowerOff,
            Event::Resume => Command::PowerOn,
            Event::VolumeUp => Command::VolumeUp,
            Event::VolumeDown => Command::VolumeDown,
            Event::VolumeMute => Command::VolumeMute,
            Event::Focus => Command::Focus,
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
        cec_rs::CecLogLevel::Notice => trace!("{}", log.message),
        cec_rs::CecLogLevel::Traffic => trace!("{}", log.message),
        cec_rs::CecLogLevel::Debug => debug!("{}", log.message),
        cec_rs::CecLogLevel::All => trace!("{}", log.message),
    }
}
