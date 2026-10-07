//! Authenticated update protocol and application update policy, independent of the UI.
mod error;
mod manifest;
mod policy;

pub use error::UpdateError;
pub use manifest::{
    MAX_MANIFEST_BYTES, MAX_PACKAGE_BYTES, MAX_SIGNATURE_BYTES, UpdateManifest, VerifiedManifest,
};
pub use policy::Platform;
