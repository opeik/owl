use std::{
    collections::HashMap,
    thread,
    time::{Duration, Instant},
};

use cec::{DeviceKind, LogicalAddress, UserControlCode};
use color_eyre::eyre::{Context, Result};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, trace, warn};

use crate::{
    job::{self, SpawnResult},
    os::{Event, Key},
    Spawn,
};

pub type CommandTx = mpsc::Sender<Command>;
pub type CommandRx = mpsc::Receiver<Command>;
type LastCmd = HashMap<Command, Instant>;

/// Represents a HDMI-CEC remote control button.
///
/// See: HDMI-CEC 1.3 Supplement 1, page 47.
/// <https://engineering.purdue.edu/ece477/Archive/2012/Spring/S12-Grp10/Datasheets/CEC_HDMI_Specification.pdf>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Button {
    VolumeUp,
    VolumeDown,
    VolumeMute,
}

/// Represents a HDMI-CEC command.
///
/// See: HDMI-CEC 1.3 Supplement 1, page 65.
/// <https://engineering.purdue.edu/ece477/Archive/2012/Spring/S12-Grp10/Datasheets/CEC_HDMI_Specification.pdf>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    PowerOn,
    PowerOff,
    Focus,
    Press(Button),
    Release(Button),
}

/// Represents a HDMI-CEC job, responsible for communicating with the HDMI-CEC bus.
/// libcec only works on a single thread, so we can't use an async task.
pub struct Job {
    cmd_tx: CommandTx,
}

#[derive(Debug, derive_more::Deref)]
struct Cec(cec::Connection);

impl Command {
    const fn debounce_duration(self) -> Option<Duration> {
        match self {
            Command::Press(_) | Command::Release(_) => Some(Duration::from_millis(200)),
            Command::Focus => Some(Duration::from_secs(3)),
            _ => None,
        }
    }
}

impl Spawn for Job {
    /// Spawns a new HDMI-CEC job. The job runs on a thread.
    async fn spawn(run_token: CancellationToken) -> SpawnResult<Self> {
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<Command>(8);
        let (ready_tx, ready_rx) = oneshot::channel::<Result<()>>();

        trace!("spawning cec job...");
        let handle = thread::spawn(move || {
            debug!("cec job starting...");

            let mut last_cmd = LastCmd::new();
            let run_token = run_token;
            let cec = job::send_ready_status(ready_tx, Cec::new)?;

            loop {
                if run_token.is_cancelled() {
                    trace!("stopping cec job...");
                    break;
                }

                handle_cmd(&cec, &mut cmd_rx, &mut last_cmd);
                std::thread::sleep(Duration::from_millis(1));
            }

            Ok(())
        });

        ready_rx
            .await
            .context("failed to read job status")?
            .context("job failed to start")?;
        debug!("cec job ready!");

        Ok((handle, Self { cmd_tx }))
    }
}

impl job::Send<Command> for Job {
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
        debug!("sending command: {cmd:?}");
        let result = match cmd {
            Command::PowerOn | Command::Focus => cec.set_active_source(DeviceKind::PlaybackDevice),
            Command::PowerOff => cec.send_standby_devices(LogicalAddress::Tv),
            Command::Press(button) => match button {
                Button::VolumeUp => cec.send_keypress(
                    LogicalAddress::Audiosystem,
                    UserControlCode::VolumeUp,
                    false,
                ),
                Button::VolumeDown => cec.send_keypress(
                    LogicalAddress::Audiosystem,
                    UserControlCode::VolumeDown,
                    false,
                ),
                Button::VolumeMute => cec.audio_toggle_mute(),
            },
            Command::Release(button) => match button {
                Button::VolumeDown | Button::VolumeUp => {
                    cec.send_key_release(LogicalAddress::Audiosystem, false)
                }
                Button::VolumeMute => Ok(()),
            },
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
        }

        *last_time = time;
    } else {
        time_by_cmd.insert(cmd, time);
    }

    Some(cmd)
}

impl Cec {
    pub fn new() -> Result<Self> {
        trace!("connecting to cec...");
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

        trace!("connected to cec!");
        Ok(Self(connection))
    }
}

impl From<Key> for Button {
    fn from(value: Key) -> Self {
        match value {
            Key::VolumeUp => Button::VolumeUp,
            Key::VolumeDown => Button::VolumeDown,
            Key::VolumeMute => Button::VolumeMute,
        }
    }
}

impl From<Event> for Command {
    fn from(value: Event) -> Self {
        match value {
            Event::Suspend => Command::PowerOff,
            Event::Resume => Command::PowerOn,
            Event::Focus => Command::Focus,
            Event::Press(key) => Command::Press(key.into()),
            Event::Release(key) => Command::Release(key.into()),
        }
    }
}

fn on_key_press(keypress: cec::Keypress) {
    trace!("got: {:?}", keypress);
}

#[allow(clippy::needless_pass_by_value)]
fn on_command_received(command: cec::Cmd) {
    trace!("got: {:?}", command);
}

#[allow(clippy::needless_pass_by_value)]
fn on_log_level(log: cec::LogMsg) {
    const TARGET: &str = "libcec";
    match log.level {
        cec::LogLevel::Error => error!(target: TARGET, "{}", log.message),
        cec::LogLevel::Warning => warn!(target: TARGET, "{}", log.message),
        cec::LogLevel::Notice => trace!(target: TARGET, "{}", log.message),
        cec::LogLevel::Traffic => trace!(target: TARGET, "{}", log.message),
        cec::LogLevel::Debug => debug!(target: TARGET, "{}", log.message),
        cec::LogLevel::All => trace!(target: TARGET, "{}", log.message),
    }
}
