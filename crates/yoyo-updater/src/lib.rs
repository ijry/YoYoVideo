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
