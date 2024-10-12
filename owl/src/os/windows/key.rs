use std::ptr;

use crate::os;

mod win32 {
    pub use windows::Win32::{
        Foundation::{LPARAM, WPARAM},
        UI::{
            Input::KeyboardAndMouse::{self, VIRTUAL_KEY},
            WindowsAndMessaging::{self, KBDLLHOOKSTRUCT},
        },
    };
}

/// See: <https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes>
#[derive(Debug, Clone, Copy, derive_more::Deref)]
pub struct Code(pub win32::VIRTUAL_KEY);

/// See: [`WM_KEYDOWN`] and [`WM_KEYUP`].
///
/// [`WM_KEYDOWN`]: https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-keydown
/// [`WM_KEYUP`]: https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-keyup
#[derive(Debug, Clone, Copy, derive_more::Deref)]
pub struct EventKind(pub u32);

/// See: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-kbdllhookstruct>
#[derive(Debug, Clone, Copy, derive_more::Deref)]
pub struct EventContext(pub win32::KBDLLHOOKSTRUCT);

#[derive(Debug, Clone, Copy)]
pub struct Event {
    #[allow(dead_code)]
    pub context: EventContext,
    pub kind: EventKind,
    pub code: Code,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to parse power event")]
    ParseError(#[from] ParseError),
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("key code is out of range")]
    KeyCodeOutOfRange,
    #[error("key state is out of range")]
    KeyStateOutOfRange,
    #[error("key event is null")]
    NullKeyEvent,
}

impl TryFrom<(win32::WPARAM, win32::LPARAM)> for Event {
    type Error = Error;

    fn try_from(value: (win32::WPARAM, win32::LPARAM)) -> Result<Self, Error> {
        let wparam = value.0;
        let lparam = value.1;

        let context = EventContext::try_from(lparam)?;
        let kind = EventKind::try_from(wparam)?;
        let code = context.key_code()?;

        Ok(Self {
            context,
            kind,
            code,
        })
    }
}

impl Event {
    pub fn to_owl_event(self) -> Option<os::Event> {
        let owl_event = match *self.kind {
            win32::WindowsAndMessaging::WM_KEYDOWN => os::Event::Press,
            win32::WindowsAndMessaging::WM_KEYUP => os::Event::Release,
            _ => return None,
        };

        let result = match *self.code {
            win32::KeyboardAndMouse::VK_VOLUME_DOWN => owl_event(os::Key::VolumeDown),
            win32::KeyboardAndMouse::VK_VOLUME_UP => owl_event(os::Key::VolumeUp),
            win32::KeyboardAndMouse::VK_VOLUME_MUTE => owl_event(os::Key::VolumeMute),
            _ => os::Event::Focus,
        };

        Some(result)
    }
}

impl EventContext {
    pub fn key_code(&self) -> Result<Code, Error> {
        let inner = win32::VIRTUAL_KEY(
            u16::try_from(self.vkCode).map_err(|_| ParseError::KeyCodeOutOfRange)?,
        );

        Ok(Code(inner))
    }
}

impl TryFrom<win32::LPARAM> for EventContext {
    type Error = ParseError;

    fn try_from(value: win32::LPARAM) -> Result<Self, ParseError> {
        #[allow(clippy::cast_sign_loss)]
        let event = ptr::with_exposed_provenance::<win32::KBDLLHOOKSTRUCT>(value.0 as usize);
        if event.is_null() {
            return Err(ParseError::NullKeyEvent);
        }

        Ok(Self(unsafe { *event }))
    }
}

impl TryFrom<win32::WPARAM> for EventKind {
    type Error = ParseError;

    fn try_from(value: win32::WPARAM) -> Result<Self, ParseError> {
        match u32::try_from(value.0) {
            Ok(x) => Ok(Self(x)),
            Err(_) => Err(ParseError::KeyStateOutOfRange),
        }
    }
}
