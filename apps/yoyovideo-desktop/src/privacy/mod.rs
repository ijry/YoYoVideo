mod credentials;
mod store;
pub use credentials::PinCredential;
pub use store::{PrivacyDocument, PrivacyStore};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrivacyError {
    InvalidPin,
    ConfirmationMismatch,
    IncorrectPin,
    Unconfigured,
    Locked,
    Busy,
    Stale,
    Cooldown(u32),
    CorruptConfig,
    Persistence,
    InvalidSchedule,
    InvalidMedia,
    WindowUnavailable,
}
impl std::fmt::Display for PrivacyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidPin => "PIN must contain exactly four ASCII digits",
            Self::ConfirmationMismatch => "PIN confirmation does not match",
            Self::IncorrectPin => "Incorrect PIN",
            Self::Unconfigured => "Set a privacy PIN first",
            Self::Locked => "Privacy authorization required",
            Self::Busy => "Privacy verification is busy",
            Self::Stale => "Privacy authorization expired",
            Self::Cooldown(_) => "Wait before trying the PIN again",
            Self::CorruptConfig => {
                "Privacy configuration cannot be read; protection remains enabled"
            }
            Self::Persistence => "Privacy configuration could not be saved",
            Self::InvalidSchedule => "Invalid privacy period",
            Self::InvalidMedia => "Invalid media identity",
            Self::WindowUnavailable => "Privacy window is unavailable",
        })
    }
}
impl std::error::Error for PrivacyError {}

mod service;
pub use service::{
    AuthGrant, AuthPurpose, AuthTicket, PrivacyService, PrivacySettings, PrivacySnapshot,
};

pub mod view;
pub(crate) mod window;
pub use window::bind_pin_validation;

#[cfg(feature = "privacy-qa")]
pub(crate) use service::PrivacyClock;
