// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! The parts of Carbon's `RegisterEventHotKey` contract that don't need Carbon.
//!
//! Kept out of the macOS backend so they compile, and are tested, on every platform.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use crate::{hotkey::HotKey, RegisterOptions};

// Declared like the other Carbon constants in `platform_impl/macos/ffi.rs`, which re-exports them.
// Values from CarbonEvents.h (`kEventHotKeyNoOptions`, `kEventHotKeyExclusive`)
// and CarbonEventsCore.h (`eventHotKeyExistsErr`).
pub type _bindgen_ty_1 = ::std::os::raw::c_uint;
pub const kEventHotKeyNoOptions: _bindgen_ty_1 = 0;
pub const kEventHotKeyExclusive: _bindgen_ty_1 = 1;
pub type _bindgen_ty_2 = ::std::os::raw::c_int;
pub const eventHotKeyExistsErr: _bindgen_ty_2 = -9878;

/// The `inOptions` argument of `RegisterEventHotKey` for `options`.
pub fn event_hotkey_options(options: RegisterOptions) -> u32 {
    if options.exclusive {
        kEventHotKeyExclusive
    } else {
        kEventHotKeyNoOptions
    }
}

/// Turns the `OSStatus` returned by `RegisterEventHotKey` into a result.
pub fn check_register_status(status: i32, hotkey: HotKey) -> crate::Result<()> {
    match status {
        0 => Ok(()),
        eventHotKeyExistsErr => Err(crate::Error::AlreadyRegistered(hotkey)),
        status => Err(crate::Error::FailedToRegister(format!(
            "RegisterEventHotKey failed for {} (OSStatus {})",
            hotkey.key, status
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hotkey::Code, Error};

    fn hotkey() -> HotKey {
        HotKey::new(None, Code::KeyD)
    }

    #[test]
    fn default_options_are_not_exclusive() {
        assert_eq!(event_hotkey_options(RegisterOptions::default()), 0);
    }

    #[test]
    fn exclusive_option_sets_the_exclusive_flag() {
        let options = RegisterOptions::new().exclusive(true);
        assert_eq!(event_hotkey_options(options), 1);
    }

    #[test]
    fn carbon_constants_match_the_sdk_headers() {
        assert_eq!(kEventHotKeyNoOptions, 0);
        assert_eq!(kEventHotKeyExclusive, 1 << 0);
        assert_eq!(eventHotKeyExistsErr, -9878);
    }

    #[test]
    fn no_err_status_registers() {
        assert!(check_register_status(0, hotkey()).is_ok());
    }

    #[test]
    fn hotkey_exists_status_is_already_registered() {
        let err = check_register_status(-9878, hotkey()).unwrap_err();
        assert!(matches!(err, Error::AlreadyRegistered(h) if h == hotkey()));
    }

    #[test]
    fn other_statuses_fail_with_the_os_status_in_the_message() {
        let err = check_register_status(-50, hotkey()).unwrap_err();
        match err {
            Error::FailedToRegister(msg) => {
                assert!(msg.contains("-50"), "{msg}");
                assert!(msg.contains("KeyD"), "{msg}");
            }
            other => panic!("expected FailedToRegister, got {other:?}"),
        }
    }
}
