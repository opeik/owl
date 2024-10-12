use std::ptr;

mod win32 {
    pub use windows::{
        core::GUID,
        Win32::{
            Foundation::LPARAM,
            System::{Power::POWERBROADCAST_SETTING, SystemServices::MONITOR_DISPLAY_STATE},
        },
    };
}

/// See: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-powerbroadcast_setting>
#[derive(Debug, Clone, Copy, derive_more::Deref)]
pub struct Event(pub win32::POWERBROADCAST_SETTING);

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to parse power event")]
    ParseError(#[from] ParseError),
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("power settings are null")]
    NullPowerSettings,
}

impl Event {
    /// See: <https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/ne-wdm-_monitor_display_state>
    pub fn state(&self) -> win32::MONITOR_DISPLAY_STATE {
        win32::MONITOR_DISPLAY_STATE(i32::from(self.Data[0]))
    }

    /// See: <https://learn.microsoft.com/en-us/windows/win32/power/power-setting-guids>
    pub fn target(&self) -> win32::GUID {
        self.PowerSetting
    }
}

impl TryFrom<win32::LPARAM> for Event {
    type Error = ParseError;

    fn try_from(value: win32::LPARAM) -> Result<Self, ParseError> {
        #[allow(clippy::cast_sign_loss)]
        let power_settings =
            ptr::with_exposed_provenance::<win32::POWERBROADCAST_SETTING>(value.0 as usize);

        if !power_settings.is_null() {
            return Err(ParseError::NullPowerSettings);
        }

        Ok(Self(unsafe { *power_settings }))
    }
}
