use windows::Win32::{
    System::Power::POWERBROADCAST_SETTING, UI::Input::KeyboardAndMouse::VIRTUAL_KEY,
};

#[derive(Debug, Clone, Copy, derive_more::Display)]
pub enum RawEvent {
    Resume,
    Suspend,
    #[display("PowerSettingChange")]
    PowerSettingChange(POWERBROADCAST_SETTING),
    #[display("KeyDown")]
    KeyDown(VIRTUAL_KEY),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookResult {
    Forward,
    Suppress,
}
