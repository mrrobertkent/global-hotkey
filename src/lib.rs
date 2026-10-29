// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

#![allow(clippy::uninlined_format_args)]

//! global_hotkey lets you register Global HotKeys for Desktop Applications.
//!
//! ## Platforms-supported:
//!
//! - Windows
//! - macOS
//! - Linux (X11 Only)
//!
//! ## Platform-specific notes:
//!
//! - On Windows a win32 event loop must be running on the thread. It doesn't need to be the main thread but you have to create the global hotkey manager on the same thread as the event loop.
//! - On macOS, an event loop must be running on the main thread so you also need to create the global hotkey manager on the main thread.
//! - On macOS, a hotkey registered with [`GlobalHotKeyManager::register`] succeeds even when another app holds it exclusively, and then doesn't fire while that app holds it. Use [`GlobalHotKeyManager::register_with_options`] with [`RegisterOptions::exclusive`] to get [`Error::AlreadyRegistered`] instead. The catch is that other apps can no longer receive that hotkey. Windows and X11 always refuse a hotkey that is already taken, so the option has no effect there.
//!
//! # Example
//!
//! ```no_run
//! use global_hotkey::{GlobalHotKeyManager, hotkey::{HotKey, Modifiers, Code}};
//!
//! // initialize the hotkeys manager
//! let manager = GlobalHotKeyManager::new().unwrap();
//!
//! // construct the hotkey
//! let hotkey = HotKey::new(Some(Modifiers::SHIFT), Code::KeyD);
//!
//! // register it
//! manager.register(hotkey);
//! ```
//!
//!
//! # Processing global hotkey events
//!
//! You can also listen for the menu events using [`GlobalHotKeyEvent::receiver`] to get events for the hotkey pressed events.
//! ```no_run
//! use global_hotkey::GlobalHotKeyEvent;
//!
//! if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
//!     println!("{:?}", event);
//! }
//! ```
//!
//! # Platforms-supported:
//!
//! - Windows
//! - macOS
//! - Linux (X11 Only)

use crossbeam_channel::{unbounded, Receiver, Sender};
use once_cell::sync::{Lazy, OnceCell};

mod error;
pub mod hotkey;
mod platform_impl;

pub use self::error::*;
use hotkey::HotKey;

/// Describes the state of the [`HotKey`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum HotKeyState {
    /// The [`HotKey`] is pressed (the key is down).
    Pressed,
    /// The [`HotKey`] is released (the key is up).
    Released,
}

/// Describes a global hotkey event emitted when a [`HotKey`] is pressed or released.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct GlobalHotKeyEvent {
    /// Id of the associated [`HotKey`].
    pub id: u32,
    /// State of the associated [`HotKey`].
    pub state: HotKeyState,
}

/// A reciever that could be used to listen to global hotkey events.
pub type GlobalHotKeyEventReceiver = Receiver<GlobalHotKeyEvent>;
type GlobalHotKeyEventHandler = Box<dyn Fn(GlobalHotKeyEvent) + Send + Sync + 'static>;

static GLOBAL_HOTKEY_CHANNEL: Lazy<(Sender<GlobalHotKeyEvent>, GlobalHotKeyEventReceiver)> =
    Lazy::new(unbounded);
static GLOBAL_HOTKEY_EVENT_HANDLER: OnceCell<Option<GlobalHotKeyEventHandler>> = OnceCell::new();

impl GlobalHotKeyEvent {
    /// Returns the id of the associated [`HotKey`].
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Returns the state of the associated [`HotKey`].
    pub fn state(&self) -> HotKeyState {
        self.state
    }

    /// Gets a reference to the event channel's [`GlobalHotKeyEventReceiver`]
    /// which can be used to listen for global hotkey events.
    ///
    /// ## Note
    ///
    /// This will not receive any events if [`GlobalHotKeyEvent::set_event_handler`] has been called with a `Some` value.
    pub fn receiver<'a>() -> &'a GlobalHotKeyEventReceiver {
        &GLOBAL_HOTKEY_CHANNEL.1
    }

    /// Set a handler to be called for new events. Useful for implementing custom event sender.
    ///
    /// ## Note
    ///
    /// Calling this function with a `Some` value,
    /// will not send new events to the channel associated with [`GlobalHotKeyEvent::receiver`]
    pub fn set_event_handler<F: Fn(GlobalHotKeyEvent) + Send + Sync + 'static>(f: Option<F>) {
        if let Some(f) = f {
            let _ = GLOBAL_HOTKEY_EVENT_HANDLER.set(Some(Box::new(f)));
        } else {
            let _ = GLOBAL_HOTKEY_EVENT_HANDLER.set(None);
        }
    }

    pub(crate) fn send(event: GlobalHotKeyEvent) {
        if let Some(handler) = GLOBAL_HOTKEY_EVENT_HANDLER.get_or_init(|| None) {
            handler(event);
        } else {
            let _ = GLOBAL_HOTKEY_CHANNEL.0.send(event);
        }
    }
}

/// Options for [`GlobalHotKeyManager::register_with_options`].
///
/// ```
/// use global_hotkey::RegisterOptions;
///
/// let options = RegisterOptions::new().exclusive(true);
/// assert!(options.exclusive);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RegisterOptions {
    /// Claim the hotkey for this process alone.
    ///
    /// Defaults to `false`, which is how [`GlobalHotKeyManager::register`] behaves.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** registers with `kEventHotKeyExclusive`. If another process already holds the
    ///   hotkey exclusively, registering fails with [`Error::AlreadyRegistered`]. Without this
    ///   option the registration succeeds even then, but this process doesn't receive the hotkey
    ///   until the exclusive holder releases it.
    ///   The trade-off is that an exclusive registration also stops every non-exclusive
    ///   registration of the same hotkey, in any process, from receiving events.
    ///   Media keys are not registered with Carbon, so this has no effect on them.
    /// - **Windows / X11:** no effect. The hotkey is always claimed exclusively, and registering
    ///   one that is already taken fails with [`Error::AlreadyRegistered`].
    pub exclusive: bool,
}

impl RegisterOptions {
    /// Creates the default options, which are not exclusive.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`RegisterOptions::exclusive`].
    pub fn exclusive(mut self, exclusive: bool) -> Self {
        self.exclusive = exclusive;
        self
    }
}

pub struct GlobalHotKeyManager {
    platform_impl: platform_impl::GlobalHotKeyManager,
}

impl GlobalHotKeyManager {
    pub fn new() -> crate::Result<Self> {
        Ok(Self {
            platform_impl: platform_impl::GlobalHotKeyManager::new()?,
        })
    }

    pub fn register(&self, hotkey: HotKey) -> crate::Result<()> {
        self.platform_impl.register(hotkey)
    }

    /// Registers `hotkey` with the given [`RegisterOptions`].
    ///
    /// [`GlobalHotKeyManager::register`] is the same as passing [`RegisterOptions::default`].
    ///
    /// Returns [`Error::AlreadyRegistered`] when the hotkey is taken. Which registrations count
    /// as taken depends on the platform, see [`RegisterOptions::exclusive`].
    ///
    /// ```no_run
    /// use global_hotkey::{GlobalHotKeyManager, RegisterOptions, hotkey::{HotKey, Modifiers, Code}};
    ///
    /// let manager = GlobalHotKeyManager::new().unwrap();
    /// let hotkey = HotKey::new(Some(Modifiers::SHIFT), Code::KeyD);
    ///
    /// manager
    ///     .register_with_options(hotkey, RegisterOptions::new().exclusive(true))
    ///     .unwrap();
    /// ```
    pub fn register_with_options(
        &self,
        hotkey: HotKey,
        options: RegisterOptions,
    ) -> crate::Result<()> {
        self.platform_impl.register_with_options(hotkey, options)
    }

    pub fn unregister(&self, hotkey: HotKey) -> crate::Result<()> {
        self.platform_impl.unregister(hotkey)
    }

    pub fn register_all(&self, hotkeys: &[HotKey]) -> crate::Result<()> {
        self.platform_impl.register_all(hotkeys)?;
        Ok(())
    }

    pub fn unregister_all(&self, hotkeys: &[HotKey]) -> crate::Result<()> {
        self.platform_impl.unregister_all(hotkeys)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_options_default_to_not_exclusive() {
        assert!(!RegisterOptions::default().exclusive);
        assert_eq!(RegisterOptions::new(), RegisterOptions::default());
    }

    #[test]
    fn register_options_builder_sets_exclusive() {
        assert!(RegisterOptions::new().exclusive(true).exclusive);
        assert!(
            !RegisterOptions::new()
                .exclusive(true)
                .exclusive(false)
                .exclusive
        );
    }
}
