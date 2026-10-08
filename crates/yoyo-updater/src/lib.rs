//! Authenticated update protocol and application update policy, independent of the UI.
mod error;
mod manifest;
mod policy;

pub use error::UpdateError;
pub use manifest::{
    MAX_MANIFEST_BYTES, MAX_PACKAGE_BYTES, MAX_SIGNATURE_BYTES, UpdateManifest, VerifiedManifest,
    verify_package_file,
};
pub use policy::Platform;

mod preferences;
mod source;
pub use preferences::UpdatePreferences;
pub use source::SignedSource;

mod service;
pub use service::{ServiceConfig, UpdatePhase, UpdateService, UpdateSnapshot};

mod pending;
#[cfg(windows)]
mod process_guard;

mod worker;
pub use worker::{
    UpdateCommand, UpdateEvent, UpdateMessage, UpdateRequest, UpdateWorker, spawn_worker,
};

mod control;
pub use control::{ControlEffect, UpdateControl};

#[cfg(feature = "qa-fixture")]
mod qa_fixture;
#[cfg(feature = "qa-fixture")]
pub use qa_fixture::QaFixture;

pub const fn qa_fixture_enabled() -> bool {
    cfg!(feature = "qa-fixture")
}
