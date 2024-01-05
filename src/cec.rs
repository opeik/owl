use std::{
    collections::HashMap,
    thread,
    time::{Duration, Instant},
};

use cec::{DeviceKind, LogicalAddress};
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
    cec_version: cec::Version,
    power_status: cec::PowerStatus,
    is_active: bool,
}

#[derive(Debug, derive_more::Deref)]
struct Cec(cec::Connection);

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
    async fn spawn(run_token: CancellationToken) -> SpawnResult<Self> {
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<Command>(8);

        debug!("spawning cec job...");
        let handle = thread::spawn(move || {
            debug!("cec job started!");

            let run_token = run_token;
            let cec = Cec::new()?;
            let mut last_cmd = LastCmd::new();
            debug!("cec job ready!");

            loop {
                if run_token.is_cancelled() {
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
            Command::PowerOn => cec.set_active_source(DeviceKind::PlaybackDevice),
            Command::PowerOff => cec.send_standby_devices(LogicalAddress::Tv),
            Command::VolumeUp => cec.volume_up(true),
            Command::VolumeDown => cec.volume_down(true),
            Command::VolumeMute => cec.audio_toggle_mute(),
            Command::Focus => cec.set_active_source(DeviceKind::PlaybackDevice),
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
        info!("connected to cec...");

        let connection = cec::Connection::builder()
            .detect_device(true)
            .name("owl".to_owned())
            .kind(DeviceKind::RecordingDevice)
            .activate_source(false)
            .on_key_press(Box::new(on_key_press))
            .on_command_received(Box::new(on_command_received))
            .on_log_message(Box::new(on_log_level))
            .hdmi_port(2)
            .connect()
            .context("failed to connect to cec")?;

        info!("connected to cec!");
        Ok(Self(connection))
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

pub fn on_key_press(keypress: cec::Keypress) {
    trace!("got: {:?}", keypress);
}

pub fn on_command_received(command: cec::Cmd) {
    trace!("got: {:?}", command);
}

pub fn on_log_level(log: cec::LogMsg) {
    match log.level {
        cec::LogLevel::Error => error!("{}", log.message),
        cec::LogLevel::Warning => warn!("{}", log.message),
        cec::LogLevel::Notice => trace!("{}", log.message),
        cec::LogLevel::Traffic => trace!("{}", log.message),
        cec::LogLevel::Debug => debug!("{}", log.message),
        cec::LogLevel::All => trace!("{}", log.message),
    }
}
