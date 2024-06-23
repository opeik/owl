#![feature(let_chains)]

pub(crate) mod callback;
pub(crate) mod convert;
pub(crate) mod types;

use std::{
    collections::HashSet,
    convert::{TryFrom, TryInto},
    ffi::{c_int, CStr, CString},
    fmt::{self, Display},
    pin::Pin,
    ptr::addr_of_mut,
    result,
    time::Duration,
};

use arrayvec::ArrayVec;
use cec_sys::*;
use derive_builder::{Builder, UninitializedFieldError};

pub use crate::types::*;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("failed to convert cmd: {0}")]
    TryFromCmdError(#[from] TryFromCmdError),
    #[error("failed to convert log msg: {0}")]
    TryFromLogMsgError(#[from] TryFromLogMsgError),
    #[error("failed to convert logical address: {0}")]
    TryFromLogicalAddressesError(#[from] TryFromLogicalAddressesError),
    #[error("failed to convert keypress: {0}")]
    TryFromKeypressError(#[from] TryFromKeypressError),
    #[error("failed to convert alert: {0}")]
    TryFromAlertError(#[from] TryFromAlertError),
    #[error("failed to convert menu state: {0}")]
    TryFromMenuStateError(#[from] TryFromMenuStateError),
    #[error("failed to connect: {0}")]
    ConnectionError(#[from] ConnectionError),
    #[error("builder error: {0}")]
    BuilderError(#[from] CfgBuilderError),
    #[error("nul byte found: {0}")]
    NulError(#[from] std::ffi::NulError),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConnectionError {
    #[error("initialization failed")]
    InitFailed,
    #[error("no adapter found")]
    NoAdapterFound,
    #[error("failed to open adapter")]
    AdapterOpenFailed,
    #[error("callback registration failed")]
    CallbackRegistrationFailed,
    #[error("transmit failed")]
    TransmitFailed,
    #[error("device missing")]
    DeviceMissing,
    #[error("ffi error: {0}")]
    FfiError(#[from] std::ffi::NulError),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TryFromCmdError {
    #[error("unknown opcode")]
    UnknownOpcode,
    #[error("unknown initiator")]
    UnknownInitiator,
    #[error("unknown destination")]
    UnknownDestination,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TryFromLogMsgError {
    #[error("message parse error")]
    MessageParseError,
    #[error("log level parse error")]
    LogLevelParseError,
    #[error("timestamp parse error")]
    TimestampParseError,
    #[error("unknown log level")]
    UnknownLogLevel,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TryFromLogicalAddressesError {
    #[error("unknown primary address")]
    UnknownPrimaryAddress,
    #[error("invalid primary address")]
    InvalidPrimaryAddress,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TryFromKeypressError {
    #[error("unknown keycode")]
    UnknownKeycode,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TryFromAlertError {
    #[error("unknown alert")]
    UnknownAlert,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TryFromMenuStateError {
    #[error("unknown menu state")]
    UnknownMenuState,
}

#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum CfgBuilderError {
    #[error("uninitialized field: {0}")]
    UninitializedField(&'static str),
    #[error("validation error: {0}")]
    ValidationError(String),
}

/// CecLogicalAddress which does not allow Unknown variant
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct KnownLogicalAddress(types::LogicalAddress);

/// CecLogicalAddress which does not allow Unknown and Unregistered variants
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct RegisteredLogicalAddress(LogicalAddress);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct UnregisteredLogicalAddress {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataPacket(pub ArrayVec<u8, 64>);

#[derive(Debug, Clone)]
pub struct Cmd {
    /// The logical address of the initiator of this message.
    pub initiator: LogicalAddress,
    /// The logical address of the destination of this message.
    pub destination: LogicalAddress,
    /// 1 when the ACK bit is set, 0 otherwise.
    pub ack: bool,
    /// 1 when the EOM bit is set, 0 otherwise.
    pub eom: bool,
    /// The opcode of this message.
    pub opcode: Opcode,
    /// The parameters attached to this message.
    pub parameters: DataPacket,
    /// 1 when an opcode is set, 0 otherwise (POLL message).
    pub opcode_set: bool,
    /// The timeout to use in ms.
    pub transmit_timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct LogMsg {
    /// The actual message.
    pub message: String,
    /// Log level of the message.
    pub level: LogLevel,
    /// Duration since connection was established.
    pub time: Duration,
}

/// Collection of logical addresses, with one primary address
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalAddresses {
    pub primary: KnownLogicalAddress,
    pub addresses: HashSet<RegisteredLogicalAddress>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Keypress {
    /// The keycode.
    pub keycode: UserControlCode,
    /// The duration of the keypress.
    pub duration: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceKinds(pub ArrayVec<DeviceKind, 5>);

#[derive(derive_more::Debug)]
pub struct Callbacks {
    #[debug(skip)]
    pub on_key_press: Option<Box<OnKeyPress>>,

    #[debug(skip)]
    pub on_cmd_received: Option<Box<OnCmd>>,

    #[debug(skip)]
    pub on_log_msg: Option<Box<OnLogMsg>>,

    #[debug(skip)]
    pub on_cfg_changed: Option<Box<OnCfgChanged>>,

    #[debug(skip)]
    pub on_alert: Option<Box<OnAlert>>,

    #[debug(skip)]
    pub on_menu_state_changed: Option<Box<OnMenuStateChanged>>,

    #[debug(skip)]
    pub on_source_activated: Option<Box<OnSourceActivated>>,
}

pub type OnKeyPress = dyn FnMut(Keypress) + Send;
pub type OnCmd = dyn FnMut(Cmd) + Send;
pub type OnLogMsg = dyn FnMut(LogMsg) + Send;
pub type OnSourceActivated = dyn FnMut(KnownLogicalAddress, bool) + Send;
pub type OnCfgChanged = dyn FnMut(Cfg) + Send;
pub type OnAlert = dyn FnMut(Alert) + Send;
pub type OnMenuStateChanged = dyn FnMut(MenuState) + Send;

static mut CALLBACKS: ICECCallbacks = ICECCallbacks {
    logMessage: Some(callback::on_log_msg),
    keyPress: Some(callback::on_key_press),
    commandReceived: Some(callback::on_cmd_received),
    configurationChanged: Some(callback::on_config_changed),
    alert: Some(callback::on_alert),
    menuStateChanged: Some(callback::on_menu_changed),
    sourceActivated: Some(callback::on_source_activated),
};

#[derive(Builder, derive_more::Debug)]
#[builder(
    pattern = "owned",
    build_fn(private, name = "build", error = "CfgBuilderError")
)]
pub struct Cfg {
    #[debug(skip)]
    #[builder(default, setter(strip_option), pattern = "owned")]
    on_key_press: Option<Box<OnKeyPress>>,

    #[debug(skip)]
    #[builder(default, setter(strip_option), pattern = "owned")]
    on_command_received: Option<Box<OnCmd>>,

    #[debug(skip)]
    #[builder(default, setter(strip_option), pattern = "owned")]
    on_log_message: Option<Box<OnLogMsg>>,

    #[debug(skip)]
    #[builder(default, setter(strip_option), pattern = "owned")]
    on_cfg_changed: Option<Box<OnCfgChanged>>,

    #[debug(skip)]
    #[builder(default, setter(strip_option), pattern = "owned")]
    on_alert: Option<Box<OnAlert>>,

    #[debug(skip)]
    #[builder(default, setter(strip_option), pattern = "owned")]
    on_menu_state_change: Option<Box<OnMenuStateChanged>>,

    #[debug(skip)]
    #[builder(default, setter(strip_option), pattern = "owned")]
    on_source_activated: Option<Box<OnSourceActivated>>,

    #[builder(default)]
    device: Option<String>,

    #[builder(default, setter(strip_option))]
    detect_device: Option<bool>,

    #[builder(default = "Duration::from_secs(5)")]
    timeout: Duration,

    //
    // cec_configuration items follow up
    //
    name: String,

    ///< the device type(s) to use on the CEC bus for libCEC.
    kind: DeviceKind,

    // optional cec_configuration items follow
    ///< the physical address of the CEC adapter.
    #[builder(default, setter(strip_option))]
    physical_address: Option<u16>,

    ///< the logical address of the device to which the adapter is connected. only used when iPhysicalAddress = 0 or when the adapter doesn't support autodetection.
    #[builder(default, setter(strip_option))]
    base_device: Option<LogicalAddress>,

    ///< the HDMI port to which the adapter is connected. only used when iPhysicalAddress = 0 or when the adapter doesn't support autodetection.
    #[builder(default, setter(strip_option))]
    hdmi_port: Option<u8>,

    ///< override the vendor ID of the TV. leave this untouched to autodetect.
    #[builder(default, setter(strip_option))]
    tv_vendor: Option<u32>,

    ///< list of devices to wake when initialising libCEC or when calling PowerOnDevices() without any parameter..
    #[builder(default, setter(strip_option))]
    wake_devices: Option<LogicalAddresses>,

    /// List of devices to power off when calling StandbyDevices() without any parameter.
    #[builder(default, setter(strip_option))]
    power_off_devices: Option<LogicalAddresses>,

    /// True to get the settings from the ROM (if set, and a v2 ROM is present), false to use these settings.
    #[builder(default, setter(strip_option))]
    settings_from_rom: Option<bool>,

    /// Make libCEC the active source on the bus when starting the player application.
    #[builder(default, setter(strip_option))]
    activate_source: Option<bool>,

    /// Put this PC in standby mode when the TV is switched off.
    /// Only used when `bShutdownOnStandby` = 0.
    #[builder(default, setter(strip_option))]
    power_off_on_standby: Option<bool>,

    /// The menu language used by the client. 3 character ISO 639-2 country code. see http://http://www.loc.gov/standards/iso639-2/ added in 1.6.2.
    #[builder(default, setter(strip_option))]
    language: Option<String>,

    /// Won't allocate a CCECClient when starting the connection when set (same as monitor mode). added in 1.6.3.
    #[builder(default, setter(strip_option))]
    monitor_only: Option<bool>,

    /// Type of the CEC adapter that we're connected to. added in 1.8.2.
    #[builder(default, setter(strip_option))]
    adapter_type: Option<AdapterType>,

    /// key code that initiates combo keys. defaults to CEC_USER_CONTROL_CODE_F1_BLUE. CEC_USER_CONTROL_CODE_UNKNOWN to disable. added in 2.0.5.
    #[builder(default, setter(strip_option))]
    combo_key: Option<UserControlCode>,

    /// Timeout until the combo key is sent as normal keypress.
    #[builder(default, setter(strip_option))]
    combo_key_timeout: Option<Duration>,

    /// Rate at which buttons autorepeat. 0 means rely on CEC device.
    #[builder(default, setter(strip_option))]
    button_repeat_rate: Option<Duration>,

    /// Duration after last update until a button is considered released.
    #[builder(default, setter(strip_option))]
    button_release_delay: Option<Duration>,

    /// Prevent double taps within this timeout. defaults to 200ms. added in 4.0.0.
    #[builder(default, setter(strip_option))]
    double_tap_timeout: Option<Duration>,

    /// Set to 1 to automatically waking an AVR when the source is activated. added in 4.0.0.
    #[builder(default, setter(strip_option))]
    autowake_avr: Option<bool>,
}

impl CfgBuilder {
    pub fn connect(self) -> Result<Connection> {
        let cfg = self.build()?;
        cfg.connect()
    }
}

#[derive(Debug)]
pub struct Connection(pub Cfg, pub libcec_connection_t, pub Pin<Box<Callbacks>>);
unsafe impl Send for Connection {}

impl Connection {
    pub fn builder() -> CfgBuilder {
        CfgBuilder::default()
    }

    pub fn transmit(&self, command: Cmd) -> Result<()> {
        if unsafe { libcec_transmit(self.1, &command.into()) } == 0 {
            Err(ConnectionError::TransmitFailed.into())
        } else {
            Ok(())
        }
    }
    pub fn send_power_on_devices(&self, address: LogicalAddress) -> Result<()> {
        if unsafe { libcec_power_on_devices(self.1, address.repr()) } == 0 {
            Err(ConnectionError::TransmitFailed.into())
        } else {
            Ok(())
        }
    }
    pub fn send_standby_devices(&self, address: LogicalAddress) -> Result<()> {
        if unsafe { libcec_standby_devices(self.1, address.repr()) } == 0 {
            Err(ConnectionError::TransmitFailed.into())
        } else {
            Ok(())
        }
    }

    pub fn set_active_source(&self, device_type: DeviceKind) -> Result<()> {
        if unsafe { libcec_set_active_source(self.1, device_type.repr()) } == 0 {
            Err(ConnectionError::TransmitFailed.into())
        } else {
            Ok(())
        }
    }

    pub fn get_active_source(&self) -> LogicalAddress {
        let active_raw: cec_logical_address = unsafe { libcec_get_active_source(self.1) };
        LogicalAddress::from_repr(active_raw).unwrap()
    }

    pub fn is_active_source(&self, address: LogicalAddress) -> Result<()> {
        if unsafe { libcec_is_active_source(self.1, address.repr()) } == 0 {
            Err(ConnectionError::TransmitFailed.into())
        } else {
            Ok(())
        }
    }

    pub fn get_device_power_status(&self, address: LogicalAddress) -> PowerStatus {
        let status_raw: cec_power_status =
            unsafe { libcec_get_device_power_status(self.1, address.repr()) };

        PowerStatus::from_repr(status_raw).unwrap()
    }

    pub fn send_keypress(
        &self,
        address: LogicalAddress,
        key: UserControlCode,
        wait: bool,
    ) -> Result<()> {
        if unsafe { libcec_send_keypress(self.1, address.repr(), key.repr(), wait.into()) } == 0 {
            Err(ConnectionError::TransmitFailed.into())
        } else {
            Ok(())
        }
    }

    pub fn send_key_release(&self, address: LogicalAddress, wait: bool) -> Result<()> {
        if unsafe { libcec_send_key_release(self.1, address.repr(), wait.into()) } == 0 {
            Err(ConnectionError::TransmitFailed.into())
        } else {
            Ok(())
        }
    }

    pub fn volume_up(&self, send_release: bool) -> Result<()> {
        if unsafe { libcec_volume_up(self.1, send_release.into()) } == 0 {
            Err(ConnectionError::TransmitFailed.into())
        } else {
            Ok(())
        }
    }

    pub fn volume_down(&self, send_release: bool) -> Result<()> {
        if unsafe { libcec_volume_down(self.1, send_release.into()) } == 0 {
            Err(ConnectionError::TransmitFailed.into())
        } else {
            Ok(())
        }
    }

    pub fn mute_audio(&self, send_release: bool) -> Result<()> {
        if unsafe { libcec_mute_audio(self.1, send_release.into()) } == 0 {
            Err(ConnectionError::TransmitFailed.into())
        } else {
            Ok(())
        }
    }

    pub fn audio_toggle_mute(&self) -> Result<()> {
        if unsafe { libcec_audio_toggle_mute(self.1) } == 0 {
            Err(ConnectionError::TransmitFailed.into())
        } else {
            Ok(())
        }
    }

    pub fn audio_mute(&self) -> Result<()> {
        if unsafe { libcec_audio_mute(self.1) } == 0 {
            Err(ConnectionError::TransmitFailed.into())
        } else {
            Ok(())
        }
    }

    pub fn audio_unmute(&self) -> Result<()> {
        if unsafe { libcec_audio_unmute(self.1) } == 0 {
            Err(ConnectionError::TransmitFailed.into())
        } else {
            Ok(())
        }
    }

    pub fn audio_get_status(&self) -> Result<()> {
        if unsafe { libcec_audio_get_status(self.1) } == 0 {
            Err(ConnectionError::TransmitFailed.into())
        } else {
            Ok(())
        }
    }

    pub fn set_inactive_view(&self) -> Result<()> {
        if unsafe { libcec_set_inactive_view(self.1) } == 0 {
            Err(ConnectionError::TransmitFailed.into())
        } else {
            Ok(())
        }
    }

    pub fn set_logical_address(&self, address: LogicalAddress) -> Result<()> {
        if unsafe { libcec_set_logical_address(self.1, address.repr()) } == 0 {
            Err(ConnectionError::TransmitFailed.into())
        } else {
            Ok(())
        }
    }

    pub fn switch_monitoring(&self, enable: bool) -> Result<()> {
        if unsafe { libcec_switch_monitoring(self.1, enable.into()) } == 0 {
            Err(ConnectionError::TransmitFailed.into())
        } else {
            Ok(())
        }
    }

    pub fn get_logical_addresses(&self) -> Result<LogicalAddresses> {
        LogicalAddresses::try_from(unsafe { libcec_get_logical_addresses(self.1) })
    }

    // Unimplemented:
    // extern DECLSPEC int libcec_set_physical_address(libcec_connection_t connection, uint16_t iPhysicalAddress);
    // extern DECLSPEC int libcec_set_deck_control_mode(libcec_connection_t connection, CEC_NAMESPACE cec_deck_control_mode mode, int bSendUpdate);
    // extern DECLSPEC int libcec_set_deck_info(libcec_connection_t connection, CEC_NAMESPACE cec_deck_info info, int bSendUpdate);
    // extern DECLSPEC int libcec_set_menu_state(libcec_connection_t connection, CEC_NAMESPACE cec_menu_state state, int bSendUpdate);
    // extern DECLSPEC int libcec_set_osd_string(libcec_connection_t connection, CEC_NAMESPACE cec_logical_address iLogicalAddress, CEC_NAMESPACE cec_display_control duration, const char* strMessage);
    // extern DECLSPEC CEC_NAMESPACE cec_version libcec_get_device_cec_version(libcec_connection_t connection, CEC_NAMESPACE cec_logical_address iLogicalAddress);
    // extern DECLSPEC int libcec_get_device_menu_language(libcec_connection_t connection, CEC_NAMESPACE cec_logical_address iLogicalAddress, CEC_NAMESPACE cec_menu_language language);
    // extern DECLSPEC uint32_t libcec_get_device_vendor_id(libcec_connection_t connection, CEC_NAMESPACE cec_logical_address iLogicalAddress);
    // extern DECLSPEC uint16_t libcec_get_device_physical_address(libcec_connection_t connection, CEC_NAMESPACE cec_logical_address iLogicalAddress);
    // extern DECLSPEC int libcec_poll_device(libcec_connection_t connection, CEC_NAMESPACE cec_logical_address iLogicalAddress);
    // extern DECLSPEC CEC_NAMESPACE cec_logical_addresses libcec_get_active_devices(libcec_connection_t connection);
    // extern DECLSPEC int libcec_is_active_device(libcec_connection_t connection, CEC_NAMESPACE cec_logical_address address);
    // extern DECLSPEC int libcec_is_active_device_type(libcec_connection_t connection, CEC_NAMESPACE cec_device_type type);
    // extern DECLSPEC int libcec_set_hdmi_port(libcec_connection_t connection, CEC_NAMESPACE cec_logical_address baseDevice, uint8_t iPort);
    // extern DECLSPEC int libcec_get_device_osd_name(libcec_connection_t connection, CEC_NAMESPACE cec_logical_address iAddress, CEC_NAMESPACE cec_osd_name name);
    // extern DECLSPEC int libcec_set_stream_path_logical(libcec_connection_t connection, CEC_NAMESPACE cec_logical_address iAddress);
    // extern DECLSPEC int libcec_set_stream_path_physical(libcec_connection_t connection, uint16_t iPhysicalAddress);
    // extern DECLSPEC int libcec_get_current_configuration(libcec_connection_t connection, CEC_NAMESPACE libcec_configuration* configuration);
    // extern DECLSPEC int libcec_can_persist_configuration(libcec_connection_t connection);
    // extern DECLSPEC int libcec_persist_configuration(libcec_connection_t connection, CEC_NAMESPACE libcec_configuration* configuration);
    // extern DECLSPEC int libcec_set_configuration(libcec_connection_t connection, const CEC_NAMESPACE libcec_configuration* configuration);
    // extern DECLSPEC void libcec_rescan_devices(libcec_connection_t connection);
    // extern DECLSPEC int libcec_is_libcec_active_source(libcec_connection_t connection);
    // extern DECLSPEC int libcec_get_device_information(libcec_connection_t connection, const char* strPort, CEC_NAMESPACE libcec_configuration* config, uint32_t iTimeoutMs);
    // extern DECLSPEC const char* libcec_get_lib_info(libcec_connection_t connection);
    // extern DECLSPEC void libcec_init_video_standalone(libcec_connection_t connection);
    // extern DECLSPEC uint16_t libcec_get_adapter_vendor_id(libcec_connection_t connection);
    // extern DECLSPEC uint16_t libcec_get_adapter_product_id(libcec_connection_t connection);
    // extern DECLSPEC int8_t libcec_detect_adapters(libcec_connection_t connection, CEC_NAMESPACE cec_adapter_descriptor* deviceList, uint8_t iBufSize, const char* strDevicePath, int bQuickScan);
}

impl Cfg {
    /// Open connection to configuration represented by this object
    ///
    ///
    /// # Errors
    ///
    /// Error is returned in following cases
    /// - LibInitFailed: cec_sys::libcec_initialise fails
    /// - AdapterOpenFailed: cec_sys::libcec_open fails
    /// - CallbackRegistrationFailed: cec_sys::libcec_enable_callbacks fails
    pub fn connect(mut self) -> Result<Connection> {
        let mut cfg: libcec_configuration = (&self).into();
        // Consume self.*_callback and build CecCallbacks from those
        let pinned_callbacks = Box::pin(Callbacks {
            on_key_press: self.on_key_press.take(),
            on_cmd_received: self.on_command_received.take(),
            on_log_msg: self.on_log_message.take(),
            on_cfg_changed: self.on_cfg_changed.take(),
            on_alert: self.on_alert.take(),
            on_menu_state_changed: self.on_menu_state_change.take(),
            on_source_activated: self.on_source_activated.take(),
        });
        let rust_callbacks_as_void_ptr = &*pinned_callbacks as *const _ as *mut _;
        let detect_device = self.detect_device.unwrap_or(false);
        let device = self.device.clone();
        let open_timeout = self.timeout.as_millis() as u32;

        let connection = Connection(
            self,
            unsafe { libcec_initialise(&mut cfg) },
            pinned_callbacks,
        );

        if connection.1.is_null() {
            return Err(ConnectionError::InitFailed.into());
        }

        let resolved_device = match detect_device {
            true => match Self::detect_device(&connection) {
                Ok(x) => x,
                Err(e) => return Err(e),
            },
            false => match device {
                Some(x) => CString::new(x)?,
                None => return Err(ConnectionError::DeviceMissing.into()),
            },
        };

        if unsafe { libcec_open(connection.1, resolved_device.as_ptr(), open_timeout) } == 0 {
            return Err(ConnectionError::AdapterOpenFailed.into());
        }

        let callback_ret = unsafe {
            cec_sys::libcec_set_callbacks(
                connection.1,
                addr_of_mut!(CALLBACKS),
                rust_callbacks_as_void_ptr,
            )
        };
        if callback_ret == 0 {
            return Err(ConnectionError::CallbackRegistrationFailed.into());
        }

        Ok(connection)
    }

    fn detect_device(connection: &Connection) -> Result<CString> {
        let mut devices: [cec_sys::cec_adapter_descriptor; 10] = unsafe { std::mem::zeroed() };
        let num_devices = unsafe {
            cec_sys::libcec_detect_adapters(
                connection.1,
                &mut devices as _,
                10,
                std::ptr::null(),
                true as i32,
            )
        };

        if num_devices < 0 {
            Err(ConnectionError::NoAdapterFound.into())
        } else {
            let device = devices[0]
                .strComName
                .into_iter()
                .flat_map(u8::try_from)
                .filter(|x| *x != 0)
                .collect::<Vec<u8>>();
            Ok(CString::new(device)?)
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        unsafe {
            libcec_close(self.1);
            libcec_destroy(self.1);
        }
    }
}

impl KnownLogicalAddress {
    pub fn new(address: LogicalAddress) -> Option<Self> {
        match address {
            LogicalAddress::Unknown => None,
            valid_address => Some(Self(valid_address)),
        }
    }
}

impl RegisteredLogicalAddress {
    pub fn new(address: LogicalAddress) -> Option<Self> {
        match address {
            LogicalAddress::Unknown | LogicalAddress::Unregistered => None,
            valid_address => Some(Self(valid_address)),
        }
    }
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Error => write!(f, "Error"),
            LogLevel::Warning => write!(f, "Warning"),
            LogLevel::Notice => write!(f, "Notice"),
            LogLevel::Traffic => write!(f, "Traffic"),
            LogLevel::Debug => write!(f, "Debug"),
            LogLevel::All => write!(f, "All"),
        }
    }
}

impl LogicalAddresses {
    pub fn with_only_primary(primary: &KnownLogicalAddress) -> LogicalAddresses {
        LogicalAddresses {
            primary: *primary,
            addresses: HashSet::new(),
        }
    }
    /// Create CecLogicalAddresses from primary address and secondary addresses
    ///
    /// # Arguments
    ///
    /// * `primary` - Primary address to use
    /// * `addresses` - other addresses to use. Primary is added to the set if not yet present
    ///
    /// Returns `None` in the following cases
    /// * when primary is `Unregistered` and `addresses` is non-empty
    ///
    pub fn with_primary_and_addresses(
        primary: &KnownLogicalAddress,
        addresses: &HashSet<RegisteredLogicalAddress>,
    ) -> Option<LogicalAddresses> {
        match (*primary).into() {
            // Invalid: Primary must be set if there are addresses
            LogicalAddress::Unregistered if !addresses.is_empty() => None,
            // Empty
            LogicalAddress::Unregistered => Some(LogicalAddresses::default()),
            // Non-empty
            _ => {
                let mut cloned_addresses = addresses.clone();
                // Following cannot panic since primary is not representing Unregistered
                let registered_address: RegisteredLogicalAddress = (*primary).try_into().unwrap();
                // We ensure that addresses always contains the primary
                cloned_addresses.insert(registered_address);
                Some(LogicalAddresses {
                    primary: *primary,
                    addresses: cloned_addresses,
                })
            }
        }
    }
}

impl DeviceKinds {
    pub fn new(value: DeviceKind) -> DeviceKinds {
        let mut inner = ArrayVec::<_, 5>::new();
        inner.push(value);
        DeviceKinds(inner)
    }
}

impl Default for LogicalAddresses {
    fn default() -> Self {
        LogicalAddresses {
            primary: KnownLogicalAddress::new(LogicalAddress::Unregistered).unwrap(),
            addresses: HashSet::new(),
        }
    }
}

fn first_n<const N: usize>(string: &str) -> [::std::os::raw::c_char; N] {
    let mut data: [::std::os::raw::c_char; N] = [0; N];
    let bytes = string.as_bytes();
    for (dst, src) in data.iter_mut().zip(bytes) {
        // c_char is either u8 or i8. We use simple casting to convert u8 accordingly
        *dst = *src as _;
    }
    data
}
