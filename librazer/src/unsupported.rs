//! Optional settings a laptop turned out not to have.
//!
//! The generic profile for a model not in the supported list offers every feature,
//! because nothing says which ones its EC implements. Issue #2 (Blade 15 2018, `0x023b`)
//! showed the cost: one getter answered "not supported" (0x05), `DeviceState::read`
//! failed as a whole, and the tray could change nothing but the screen refresh rate.
//!
//! For such a model the device records each optional setting whose command the EC
//! answers with "not supported". Reads then fill that field with a placeholder, writes
//! skip it, drift checks ignore it and the tray hides its menu. The answer is a property
//! of the firmware, so it is never cleared while the process runs.
//!
//! Only optional settings are learned. Performance and fan mode share one command
//! (`0x0d82`), and without it the tray has nothing to show, so that failure still ends
//! the read. Supported models never learn: a 0x05 there stays an error, as before.

use std::sync::atomic::{AtomicU8, Ordering};

/// A setting the tray and CLI can work without.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Optional {
    Logo,
    KeyboardBrightness,
    BatteryCare,
}

impl Optional {
    pub const ALL: [Optional; 3] = [
        Optional::Logo,
        Optional::KeyboardBrightness,
        Optional::BatteryCare,
    ];

    fn bit(self) -> u8 {
        match self {
            Optional::Logo => 1,
            Optional::KeyboardBrightness => 2,
            Optional::BatteryCare => 4,
        }
    }

    /// The name `razer-cli` and the log use for it.
    pub fn name(self) -> &'static str {
        match self {
            Optional::Logo => "logo",
            Optional::KeyboardBrightness => "keyboard brightness",
            Optional::BatteryCare => "battery care",
        }
    }
}

/// A set of [`Optional`] settings, by value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Unsupported(u8);

impl Unsupported {
    pub fn contains(self, feature: Optional) -> bool {
        self.0 & feature.bit() != 0
    }

    pub fn with(self, feature: Optional) -> Self {
        Self(self.0 | feature.bit())
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn iter(self) -> impl Iterator<Item = Optional> {
        Optional::ALL.into_iter().filter(move |f| self.contains(*f))
    }
}

/// The record a device keeps while it runs. Atomic so a `Device` stays `Sync`.
#[derive(Debug, Default)]
pub struct UnsupportedLog(AtomicU8);

impl UnsupportedLog {
    pub fn get(&self) -> Unsupported {
        Unsupported(self.0.load(Ordering::Relaxed))
    }

    /// Record `feature`; true the first time, so the caller logs it once.
    pub fn mark(&self, feature: Optional) -> bool {
        self.0.fetch_or(feature.bit(), Ordering::Relaxed) & feature.bit() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_set_holds_what_was_added() {
        let s = Unsupported::default().with(Optional::BatteryCare);
        assert!(s.contains(Optional::BatteryCare));
        assert!(!s.contains(Optional::Logo));
        assert_eq!(s.iter().collect::<Vec<_>>(), vec![Optional::BatteryCare]);
        assert!(Unsupported::default().is_empty());
    }

    #[test]
    fn the_log_reports_a_feature_new_only_once() {
        let log = UnsupportedLog::default();
        assert!(log.mark(Optional::Logo));
        assert!(!log.mark(Optional::Logo));
        assert!(log.mark(Optional::KeyboardBrightness));
        assert_eq!(
            log.get(),
            Unsupported::default()
                .with(Optional::Logo)
                .with(Optional::KeyboardBrightness)
        );
    }
}
