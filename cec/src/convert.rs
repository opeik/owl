use std::mem;

use arrayvec::ArrayVec;
use num_traits::ToPrimitive;

pub use crate::*;

impl From<KnownLogicalAddress> for LogicalAddress {
    fn from(address: KnownLogicalAddress) -> Self {
        address.0
    }
}

impl From<KnownLogicalAddress> for cec_logical_address {
    fn from(address: KnownLogicalAddress) -> Self {
        address.0.repr()
    }
}

impl From<RegisteredLogicalAddress> for LogicalAddress {
    fn from(address: RegisteredLogicalAddress) -> Self {
        address.0
    }
}

impl From<RegisteredLogicalAddress> for cec_logical_address {
    fn from(address: RegisteredLogicalAddress) -> Self {
        address.0.repr()
    }
}

impl From<DataPacket> for cec_datapacket {
    fn from(datapacket: DataPacket) -> Self {
        let mut data = [0u8; 64];
        data[..datapacket.0.len()].clone_from_slice(datapacket.0.as_slice());
        Self {
            data,
            size: datapacket.0.len() as u8,
        }
    }
}

impl From<cec_datapacket> for DataPacket {
    fn from(datapacket: cec_datapacket) -> Self {
        let end = datapacket.size as usize;
        let mut packet = Self(ArrayVec::new());
        packet
            .0
            .try_extend_from_slice(&datapacket.data[..end])
            .unwrap();
        packet
    }
}

impl From<Cmd> for cec_command {
    fn from(command: Cmd) -> Self {
        Self {
            initiator: command.initiator.repr(),
            destination: command.destination.repr(),
            ack: command.ack.into(),
            eom: command.eom.into(),
            opcode: command.opcode.repr(),
            parameters: command.parameters.into(),
            opcode_set: command.opcode_set.into(),
            transmit_timeout: command.transmit_timeout.as_millis() as i32,
        }
    }
}

impl From<LogicalAddresses> for cec_logical_addresses {
    fn from(addresses: LogicalAddresses) -> Self {
        // cec_logical_addresses.addresses is a 'mask'
        // cec_logical_addresses.addresses[logical_address value] = 1 when mask contains
        // the address
        let mut data = Self {
            primary: addresses.primary.into(),
            addresses: [0; 16],
        };
        for known_address in addresses.addresses {
            let address: LogicalAddress = known_address.into();
            let address_mask_position = address.repr();
            data.addresses[address_mask_position as usize] = 1;
        }
        data
    }
}

impl From<DeviceKinds> for cec_device_type_list {
    fn from(device_types: DeviceKinds) -> Self {
        let mut devices = Self {
            types: [DeviceKind::Reserved.repr(); 5],
        };
        for (i, type_id) in device_types.0.iter().enumerate() {
            devices.types[i] = (*type_id).repr();
        }
        devices
    }
}

impl From<&Cfg> for libcec_configuration {
    fn from(config: &Cfg) -> Self {
        let mut cfg: Self;
        unsafe {
            cfg = mem::zeroed::<Self>();
            libcec_clear_configuration(&mut cfg);
        }
        cfg.clientVersion = libcec_version::CURRENT as _;
        cfg.strDeviceName = first_n::<{ LIBCEC_OSD_NAME_SIZE as usize }>(&config.name);
        cfg.deviceTypes = DeviceKinds::new(config.kind).into();
        if let Some(v) = config.physical_address {
            cfg.iPhysicalAddress = v;
        }
        if let Some(v) = config.base_device {
            cfg.baseDevice = v.repr();
        }
        if let Some(v) = config.hdmi_port {
            cfg.iHDMIPort = v;
        }
        if let Some(v) = config.tv_vendor {
            cfg.tvVendor = v;
        }
        if let Some(v) = config.wake_devices.clone() {
            cfg.wakeDevices = v.into();
        }
        if let Some(v) = config.power_off_devices.clone() {
            cfg.powerOffDevices = v.into();
        }
        if let Some(v) = config.settings_from_rom {
            cfg.bGetSettingsFromROM = v.into();
        }
        if let Some(v) = config.activate_source {
            cfg.bActivateSource = v.into();
        }
        if let Some(v) = config.power_off_on_standby {
            cfg.bPowerOffOnStandby = v.into();
        }
        if let Some(v) = config.language.clone() {
            cfg.strDeviceLanguage = first_n::<3>(&v);
        }
        if let Some(v) = config.monitor_only {
            cfg.bMonitorOnly = v.into();
        }
        if let Some(v) = config.adapter_type {
            cfg.adapterType = v.repr();
        }
        if let Some(v) = config.combo_key {
            cfg.comboKey = v.repr();
        }
        if let Some(v) = config.combo_key_timeout {
            cfg.iComboKeyTimeoutMs = v.as_millis().to_u32().unwrap();
        }
        if let Some(v) = config.button_repeat_rate {
            cfg.iButtonRepeatRateMs = v.as_millis().to_u32().unwrap();
        }
        if let Some(v) = config.button_release_delay {
            cfg.iButtonReleaseDelayMs = v.as_millis().to_u32().unwrap();
        }
        if let Some(v) = config.double_tap_timeout {
            cfg.iDoubleTapTimeoutMs = v.as_millis().to_u32().unwrap();
        }
        if let Some(v) = config.autowake_avr {
            cfg.bAutoWakeAVR = v.into();
        }
        cfg
    }
}

impl TryFrom<libcec_configuration> for Cfg {
    type Error = Error;

    fn try_from(_value: libcec_configuration) -> Result<Self> {
        todo!()
        // Ok(Self {
        //     on_key_press: todo!(),
        //     on_command_received: todo!(),
        //     on_log_message: todo!(),
        //     on_cfg_changed: todo!(),
        //     on_alert: todo!(),
        //     on_menu_state_change: todo!(),
        //     on_source_activated: todo!(),
        //     device: todo!(),
        //     detect_device: todo!(),
        //     timeout: todo!(),
        //     name: todo!(),
        //     kind: todo!(),
        //     physical_address: todo!(),
        //     base_device: todo!(),
        //     hdmi_port: todo!(),
        //     tv_vendor: todo!(),
        //     wake_devices: todo!(),
        //     power_off_devices: todo!(),
        //     settings_from_rom: todo!(),
        //     activate_source: todo!(),
        //     power_off_on_standby: todo!(),
        //     language: todo!(),
        //     monitor_only: todo!(),
        //     adapter_type: todo!(),
        //     combo_key: todo!(),
        //     combo_key_timeout: todo!(),
        //     button_repeat_rate: todo!(),
        //     button_release_delay: todo!(),
        //     double_tap_timeout: todo!(),
        //     autowake_avr: todo!(),
        // })
    }
}

impl From<String> for CfgBuilderError {
    fn from(s: String) -> Self {
        Self::ValidationError(s)
    }
}

impl From<UninitializedFieldError> for CfgBuilderError {
    fn from(e: UninitializedFieldError) -> Self {
        Self::UninitializedField(e.field_name())
    }
}

impl TryFrom<KnownLogicalAddress> for RegisteredLogicalAddress {
    type Error = Error;

    fn try_from(address: KnownLogicalAddress) -> Result<Self> {
        let unchecked_address = address.0;
        Ok(Self::new(unchecked_address)
            .ok_or(TryFromLogicalAddressesError::InvalidPrimaryAddress)?)
    }
}

impl TryFrom<cec_command> for Cmd {
    type Error = Error;

    fn try_from(command: cec_command) -> Result<Self> {
        let opcode = Opcode::from_repr(command.opcode).ok_or(TryFromCmdError::UnknownOpcode)?;
        let initiator = LogicalAddress::from_repr(command.initiator)
            .ok_or(TryFromCmdError::UnknownInitiator)?;
        let destination = LogicalAddress::from_repr(command.destination)
            .ok_or(TryFromCmdError::UnknownDestination)?;
        let parameters = command.parameters.into();
        let transmit_timeout = Duration::from_millis(if command.transmit_timeout < 0 {
            0
        } else {
            command.transmit_timeout.try_into().unwrap()
        });
        Ok(Cmd {
            initiator,
            destination,
            ack: command.ack != 0,
            eom: command.eom != 0,
            opcode,
            parameters,
            opcode_set: command.opcode_set != 0,
            transmit_timeout,
        })
    }
}

impl TryFrom<cec_log_message> for LogMsg {
    type Error = Error;

    fn try_from(log_message: cec_log_message) -> Result<Self> {
        let c_str: &CStr = unsafe { CStr::from_ptr(log_message.message) };
        let message = c_str
            .to_str()
            .map_err(|_| TryFromLogMsgError::MessageParseError)?
            .to_owned();
        let level =
            LogLevel::from_repr(log_message.level).ok_or(TryFromLogMsgError::LogLevelParseError)?;
        let time = log_message
            .time
            .try_into()
            .map_err(|_| TryFromLogMsgError::TimestampParseError)?;

        Ok(LogMsg {
            message,
            level,
            time: Duration::from_millis(time),
        })
    }
}

impl TryFrom<cec_logical_addresses> for LogicalAddresses {
    type Error = Error;

    fn try_from(addresses: cec_logical_addresses) -> Result<Self> {
        let primary = LogicalAddress::from_repr(addresses.primary)
            .ok_or(TryFromLogicalAddressesError::InvalidPrimaryAddress)?;
        let primary = KnownLogicalAddress::new(primary)
            .ok_or(TryFromLogicalAddressesError::UnknownPrimaryAddress)?;

        let addresses = HashSet::from_iter(addresses.addresses.into_iter().enumerate().filter_map(
            |(logical_addr, addr_mask)| {
                let logical_addr = logical_addr as c_int;
                // If logical address x is in use, addresses.addresses[x] != 0.
                if addr_mask != 0 {
                    RegisteredLogicalAddress::new(LogicalAddress::try_from(logical_addr).unwrap())
                } else {
                    None
                }
            },
        ));

        Ok(Self { primary, addresses })
    }
}

impl TryFrom<cec_logical_address> for KnownLogicalAddress {
    type Error = Error;

    fn try_from(addr: cec_logical_address) -> Result<Self> {
        let addr = LogicalAddress::from_repr(addr)
            .ok_or(TryFromLogicalAddressesError::InvalidPrimaryAddress)?;
        let addr = KnownLogicalAddress::new(addr)
            .ok_or(TryFromLogicalAddressesError::UnknownPrimaryAddress)?;
        Ok(addr)
    }
}

impl TryFrom<cec_keypress> for Keypress {
    type Error = Error;

    fn try_from(keypress: cec_keypress) -> Result<Self> {
        let keycode = UserControlCode::from_repr(keypress.keycode)
            .ok_or(TryFromKeypressError::UnknownKeycode)?;
        Ok(Keypress {
            keycode,
            duration: Duration::from_millis(keypress.duration.into()),
        })
    }
}

impl TryFrom<libcec_alert> for Alert {
    type Error = Error;

    fn try_from(keypress: libcec_alert) -> Result<Self> {
        Ok(Self::from_repr(keypress).ok_or(TryFromAlertError::UnknownAlert)?)
    }
}

impl TryFrom<cec_menu_state> for MenuState {
    type Error = Error;

    fn try_from(value: cec_menu_state) -> Result<Self> {
        Ok(Self::from_repr(value).ok_or(TryFromMenuStateError::UnknownMenuState)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_version() {
        assert_eq!(CEC_LIB_VERSION_MAJOR, 6);
    }

    mod utils {
        use super::*;

        #[allow(clippy::unnecessary_cast)]
        #[test]
        fn test_first_3() {
            assert_eq!(
                [b's' as _, b'a' as _, b'm' as _] as [::std::os::raw::c_char; 3],
                first_n::<3>("sample")
            );
            assert_eq!(
                [b's' as _, b'a' as _, 0 as _] as [::std::os::raw::c_char; 3],
                first_n::<3>("sa")
            );
            assert_eq!(
                [0 as _, 0 as _, 0 as _] as [::std::os::raw::c_char; 3],
                first_n::<3>("")
            );
        }

        #[allow(clippy::unnecessary_cast)]
        #[test]
        fn test_first_7() {
            assert_eq!(
                [b's' as _, b'a' as _, b'm' as _, b'p' as _, b'l' as _, b'e' as _, 0]
                    as [::std::os::raw::c_char; 7],
                first_n::<7>("sample")
            );
        }
        #[test]
        fn test_first_0() {
            assert_eq!([] as [::std::os::raw::c_char; 0], first_n::<0>("sample"));
        }
    }

    #[cfg(test)]
    mod address {
        use super::*;

        #[test]
        fn test_known_address() {
            assert_eq!(
                Some(KnownLogicalAddress(LogicalAddress::Audiosystem)),
                KnownLogicalAddress::new(LogicalAddress::Audiosystem)
            );
            assert_eq!(
                Some(KnownLogicalAddress(LogicalAddress::Unregistered)),
                KnownLogicalAddress::new(LogicalAddress::Unregistered)
            );
            assert_eq!(None, KnownLogicalAddress::new(LogicalAddress::Unknown));
        }

        #[test]
        fn test_known_and_registered_address() {
            assert_eq!(
                Some(RegisteredLogicalAddress(LogicalAddress::Audiosystem)),
                RegisteredLogicalAddress::new(LogicalAddress::Audiosystem)
            );
            assert_eq!(
                None,
                RegisteredLogicalAddress::new(LogicalAddress::Unregistered)
            );
            assert_eq!(None, RegisteredLogicalAddress::new(LogicalAddress::Unknown));
        }

        #[test]
        fn test_to_ffi_no_address() {
            let ffi_addresses: cec_logical_addresses = LogicalAddresses::default().into();
            assert_eq!(ffi_addresses.primary, LogicalAddress::Unregistered.repr());
            assert_eq!(ffi_addresses.addresses, [0; 16]);

            // try converting back
            let rust_addresses = LogicalAddresses::try_from(ffi_addresses).unwrap();
            assert_eq!(
                rust_addresses.primary,
                KnownLogicalAddress(LogicalAddress::Unregistered)
            );
            assert!(rust_addresses.addresses.is_empty());
        }

        #[test]
        fn test_to_ffi_one_address() {
            let ffi_addresses: cec_logical_addresses = LogicalAddresses::with_only_primary(
                &KnownLogicalAddress::new(LogicalAddress::Playbackdevice1).unwrap(),
            )
            .into();
            assert_eq!(
                ffi_addresses.primary,
                LogicalAddress::Playbackdevice1.repr()
            );
            // addresses mask should be all zeros
            assert_eq!(ffi_addresses.addresses, [0; 16]);

            // try converting back
            let rust_addresses = LogicalAddresses::try_from(ffi_addresses).unwrap();
            assert_eq!(
                rust_addresses.primary,
                KnownLogicalAddress(LogicalAddress::Playbackdevice1)
            );
            assert!(rust_addresses.addresses.is_empty());
        }

        #[test]
        fn test_to_ffi_three_address() {
            let mut others = HashSet::new();
            others.insert(RegisteredLogicalAddress::new(LogicalAddress::Playbackdevice2).unwrap());
            others.insert(RegisteredLogicalAddress::new(LogicalAddress::Audiosystem).unwrap());

            let non_ffi = LogicalAddresses::with_primary_and_addresses(
                &KnownLogicalAddress::new(LogicalAddress::Playbackdevice1).unwrap(),
                &others,
            )
            .unwrap();

            let ffi_addresses: cec_logical_addresses = non_ffi.clone().into();

            assert_eq!(
                ffi_addresses.primary,
                LogicalAddress::Playbackdevice1.repr()
            );
            let ffi_secondary = ffi_addresses.addresses;
            const PRIMARY_INDEX: usize = LogicalAddress::Playbackdevice1 as usize;
            const PLAYBACKDEVICE2_INDEX: usize = LogicalAddress::Playbackdevice2 as usize;
            const AUDIOSYSTEM_INDEX: usize = LogicalAddress::Audiosystem as usize;
            for (mask_index, mask_value) in ffi_secondary.iter().enumerate() {
                match mask_index {
                    // Note: also the primary address is in the mask even though it was not provided
                    // originally
                    PLAYBACKDEVICE2_INDEX | AUDIOSYSTEM_INDEX | PRIMARY_INDEX => {
                        assert_eq!(
                            1, *mask_value,
                            "index {}, non-ffi addresses {:?}, ffi addresses {:?}",
                            mask_index, non_ffi, ffi_addresses
                        )
                    }
                    _ => assert_eq!(0, *mask_value),
                }
            }

            // try converting back
            let rust_addresses = LogicalAddresses::try_from(ffi_addresses).unwrap();
            assert_eq!(rust_addresses.primary, non_ffi.primary);
            assert_eq!(rust_addresses.addresses, non_ffi.addresses);
        }

        #[test]
        fn test_unregistered_primary_no_others() {
            let expected = Some(LogicalAddresses::with_only_primary(
                &KnownLogicalAddress::new(LogicalAddress::Unregistered).unwrap(),
            ));
            assert_eq!(
                expected,
                LogicalAddresses::with_primary_and_addresses(
                    &KnownLogicalAddress::new(LogicalAddress::Unregistered).unwrap(),
                    &HashSet::new(),
                )
            );
        }

        #[test]
        fn test_unregistered_primary_some_others() {
            let mut others = HashSet::new();
            others.insert(RegisteredLogicalAddress::new(LogicalAddress::Audiosystem).unwrap());
            // If there are others, there should be also primary
            assert_eq!(
                None,
                LogicalAddresses::with_primary_and_addresses(
                    &KnownLogicalAddress::new(LogicalAddress::Unregistered).unwrap(),
                    &others,
                )
            );
        }
    }

    #[cfg(test)]
    mod data_packet {
        use super::*;

        /// Assert that
        /// 1) sizes match
        /// 2) and that the elements of CecDatapacket match the first elements
        ///    of packet2
        fn assert_eq_packet(packet: DataPacket, packet2: cec_datapacket) {
            assert_eq!(packet.0.len(), packet2.size.into());
            assert!(packet
                .0
                .as_slice()
                .iter()
                .eq(packet2.data[..(packet2.size as usize)].iter()));
        }

        fn assert_eq_ffi_packet(packet: cec_datapacket, packet2: cec_datapacket) {
            assert_eq!(packet.size, packet2.size);
            assert!(&packet.data.iter().eq(packet2.data.iter()));
        }

        #[test]
        fn test_from_ffi_full_size() {
            let mut data_buffer = [50; 64];
            data_buffer[0] = 5;
            data_buffer[1] = 7;
            data_buffer[3] = 99;
            let ffi_packet = cec_datapacket {
                data: data_buffer,
                size: 64,
            };
            let packet: DataPacket = ffi_packet.into();
            assert_eq_packet(packet, ffi_packet);
        }

        #[test]
        fn test_from_ffi_not_full() {
            let mut data_buffer = [50; 64];
            data_buffer[0] = 5;
            data_buffer[1] = 7;
            data_buffer[3] = 99;
            let ffi_packet = cec_datapacket {
                data: data_buffer,
                size: 3,
            };
            let packet: DataPacket = ffi_packet.into();
            assert_eq!(packet.0.as_slice(), &[5, 7, 50]);
        }

        #[test]
        fn test_to_ffi_not_full() {
            let mut a = ArrayVec::new();
            a.push(2);
            a.push(50);
            let packet = DataPacket(a);
            let ffi_packet: cec_datapacket = packet.into();
            let mut expected = cec_datapacket {
                size: 2,
                data: [0; 64],
            };
            expected.data[0] = 2;
            expected.data[1] = 50;
            assert_eq_ffi_packet(ffi_packet, expected);
        }

        #[test]
        fn test_to_ffi_full() {
            let mut a = ArrayVec::from([99; 64]);
            a.as_mut_slice()[1] = 50;
            let packet = DataPacket(a);
            let ffi_packet: cec_datapacket = packet.into();
            let mut expected = cec_datapacket {
                size: 64,
                data: [99; 64],
            };
            expected.data[1] = 50;
            assert_eq_ffi_packet(ffi_packet, expected);
        }
    }

    #[cfg(test)]
    mod command {
        use super::*;

        fn assert_eq_ffi_packet(packet: cec_datapacket, packet2: cec_datapacket) {
            assert_eq!(packet.size, packet2.size);
            assert!(&packet.data.iter().eq(packet2.data.iter()));
        }

        fn assert_eq_ffi_command(actual: cec_command, expected: cec_command) {
            assert_eq!(actual.ack, expected.ack);
            assert_eq!(actual.destination, expected.destination);
            assert_eq!(actual.eom, expected.eom);
            assert_eq!(actual.initiator, expected.initiator);
            assert_eq!(actual.opcode, expected.opcode);
            assert_eq!(actual.opcode_set, expected.opcode_set);
            assert_eq_ffi_packet(actual.parameters, expected.parameters);
            assert_eq!(actual.transmit_timeout, expected.transmit_timeout);
        }

        fn assert_eq_command(actual: Cmd, expected: Cmd) {
            assert_eq!(actual.ack, expected.ack);
            assert_eq!(actual.destination, expected.destination);
            assert_eq!(actual.eom, expected.eom);
            assert_eq!(actual.initiator, expected.initiator);
            assert_eq!(actual.opcode, expected.opcode);
            assert_eq!(actual.opcode_set, expected.opcode_set);
            assert_eq!(actual.parameters.0, expected.parameters.0);
            assert_eq!(actual.transmit_timeout, expected.transmit_timeout);
        }

        #[test]
        fn test_to_ffi() {
            let mut parameters = ArrayVec::new();
            parameters.push(2);
            parameters.push(3);
            let command = Cmd {
                opcode: Opcode::ClearAnalogueTimer,
                initiator: LogicalAddress::Playbackdevice1,
                destination: LogicalAddress::Playbackdevice2,
                parameters: DataPacket(parameters.clone()),
                transmit_timeout: Duration::from_secs(65),
                ack: false,
                eom: true,
                opcode_set: true,
            };
            let ffi_command: cec_command = command.into();
            assert_eq_ffi_command(
                ffi_command,
                cec_command {
                    ack: 0,
                    destination: LogicalAddress::Playbackdevice2.repr(),
                    eom: 1,
                    initiator: LogicalAddress::Playbackdevice1.repr(),
                    opcode: Opcode::ClearAnalogueTimer.repr(),
                    opcode_set: 1,
                    parameters: DataPacket(parameters).into(), /* OK to use here, verified in
                                                                * CecDatapacket unit tests */
                    transmit_timeout: 65_000,
                },
            )
        }

        #[test]
        fn test_from_ffi() {
            let mut parameters = ArrayVec::new();
            parameters.push(2);
            parameters.push(3);
            let ffi_command = cec_command {
                ack: 0,
                destination: LogicalAddress::Playbackdevice2.repr(),
                eom: 1,
                initiator: LogicalAddress::Playbackdevice1.repr(),
                opcode: Opcode::ClearAnalogueTimer.repr(),
                opcode_set: 1,
                parameters: DataPacket(parameters.clone()).into(), /* OK to use here, verified in
                                                                    * CecDatapacket unit tests */
                transmit_timeout: 65_000,
            };
            let command: Cmd = ffi_command.try_into().unwrap();
            assert_eq_command(
                command,
                Cmd {
                    ack: false,
                    destination: LogicalAddress::Playbackdevice2,
                    eom: true,
                    initiator: LogicalAddress::Playbackdevice1,
                    opcode: Opcode::ClearAnalogueTimer,
                    opcode_set: true,
                    parameters: DataPacket(parameters),
                    transmit_timeout: Duration::from_millis(65000),
                },
            )
        }
    }

    #[cfg(test)]
    mod device {
        use super::*;

        #[test]
        fn test_to_ffi_empty() {
            let devices = ArrayVec::new();
            let ffi_devices: cec_device_type_list = DeviceKinds(devices).into();
            assert_eq!(ffi_devices.types, [DeviceKind::Reserved.repr(); 5]);
        }

        #[test]
        fn test_to_ffi_two_devices() {
            let mut devices = ArrayVec::new();
            devices.push(DeviceKind::PlaybackDevice);
            devices.push(DeviceKind::RecordingDevice);
            let ffi_devices: cec_device_type_list = DeviceKinds(devices).into();
            assert_eq!(ffi_devices.types[0], DeviceKind::PlaybackDevice.repr());
            assert_eq!(ffi_devices.types[1], DeviceKind::RecordingDevice.repr());
            assert_eq!(ffi_devices.types[2..], [DeviceKind::Reserved.repr(); 3]);
        }
    }

    #[cfg(test)]
    mod keypress {
        use super::*;

        #[test]
        fn test_keypress_from_ffi_known_code() {
            let keypress: Keypress = cec_keypress {
                keycode: cec_user_control_code::UP,
                duration: 300,
            }
            .try_into()
            .unwrap();
            assert_eq!(keypress.keycode, UserControlCode::Up);
            assert_eq!(keypress.duration, Duration::from_millis(300));
        }

        #[test]
        fn test_keypress_from_ffi_unknown_code() {
            let keypress: Result<Keypress> = cec_keypress {
                keycode: unsafe { std::mem::transmute::<i32, cec_user_control_code>(666) },
                duration: 300,
            }
            .try_into();
            assert_eq!(keypress, Err(TryFromKeypressError::UnknownKeycode.into()));
        }
    }
}
