use std::{convert::TryInto, ffi::c_int, os::raw::c_void};

use cec_sys::*;
use log::trace;

use crate::Callbacks;

pub extern "C" fn on_key_press(callbacks: *mut c_void, keypress: *const cec_keypress) {
    trace!("on_key_press: {keypress:?}");

    let callbacks: *mut Callbacks = callbacks.cast();
    if let Some(rust_callbacks) = unsafe { callbacks.as_mut() }
        && let Some(keypress) = unsafe { keypress.as_ref() }
        && let Some(callback) = &mut rust_callbacks.on_key_press
        && let Ok(keypress) = (*keypress).try_into()
    {
        callback(keypress);
    }
}

pub extern "C" fn on_cmd_received(callback: *mut c_void, cmd: *const cec_command) {
    trace!("on_cmd_received: {cmd:?}");

    let callbacks: *mut Callbacks = callback.cast();
    if let Some(callbacks) = unsafe { callbacks.as_mut() }
        && let Some(command) = unsafe { cmd.as_ref() }
        && let Some(callback) = &mut callbacks.on_cmd_received
        && let Ok(command) = (*command).try_into()
    {
        callback(command);
    }
}

pub extern "C" fn on_log_msg(callbacks: *mut c_void, log_msg: *const cec_log_message) {
    trace!("on_log_msg: {:?}", unsafe { *log_msg });

    let callbacks: *mut Callbacks = callbacks.cast();
    if let Some(callbacks) = unsafe { callbacks.as_mut() }
        && let Some(log_message) = unsafe { log_msg.as_ref() }
        && let Some(callback) = &mut callbacks.on_log_msg
        && let Ok(log_message) = (*log_message).try_into()
    {
        callback(log_message);
    }
}

pub unsafe extern "C" fn on_config_changed(
    callbacks: *mut c_void,
    config: *const libcec_configuration,
) {
    trace!("on_config_changed: {:?}", *config);

    let callbacks: *mut Callbacks = callbacks.cast();
    if let Some(callbacks) = unsafe { callbacks.as_mut() }
        && let Some(config) = unsafe { config.as_ref() }
        && let Some(callback) = &mut callbacks.on_cfg_changed
        && let Ok(config) = (*config).try_into()
    {
        callback(config);
    }
}

pub unsafe extern "C" fn on_alert(
    callbacks: *mut c_void,
    alert: libcec_alert,
    param: libcec_parameter,
) {
    trace!("on_alert: {alert:?}, {param:?}");

    let callbacks: *mut Callbacks = callbacks.cast();
    if let Some(callbacks) = unsafe { callbacks.as_mut() }
        && let Some(callback) = &mut callbacks.on_alert
        && let Ok(alert) = alert.try_into()
    {
        callback(alert);
    }
}

pub unsafe extern "C" fn on_menu_changed(
    callbacks: *mut ::std::os::raw::c_void,
    menu_state: cec_menu_state,
) -> c_int {
    trace!("on_menu_changed: {menu_state:?}");

    let callbacks: *mut Callbacks = callbacks.cast();
    if let Some(callbacks) = unsafe { callbacks.as_mut() }
        && let Some(callback) = &mut callbacks.on_menu_state_changed
        && let Ok(menu_state) = menu_state.try_into()
    {
        callback(menu_state);
    }

    0
}

pub unsafe extern "C" fn on_source_activated(
    callbacks: *mut c_void,
    logical_address: cec_logical_address,
    is_activated: u8,
) {
    trace!("on_source_activated: {logical_address:?}, {is_activated}");

    let callbacks: *mut Callbacks = callbacks.cast();
    if let Some(callbacks) = unsafe { callbacks.as_mut() }
        && let Some(callback) = &mut callbacks.on_source_activated
        && let Ok(logical_address) = logical_address.try_into()
    {
        callback(logical_address, is_activated != 0);
    }
}
