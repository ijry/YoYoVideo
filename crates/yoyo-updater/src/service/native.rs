use super::{Backend, ServiceConfig};
use crate::{Platform, SignedSource, UpdateError, VerifiedManifest};
use semver::Version;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use velopack::locator::{LocationContext, auto_locate_app_manifest};
use velopack::{UpdateCheck, UpdateInfo, UpdateManager, UpdateOptions};

pub(super) struct NativeBackend {
    manager: UpdateManager,
    source: SignedSource,
    version: Version,
    packages: PathBuf,
    binaries: PathBuf,
}
impl NativeBackend {
    pub(super) fn new(config: &ServiceConfig) -> Result<Self, UpdateError> {
        if Platform::current() != Some(config.platform) {
            return Err(UpdateError::Unsupported);
        }
        let locator = auto_locate_app_manifest(LocationContext::FromCurrentExe)
            .map_err(|_| UpdateError::Unsupported)?;
        let manifest = locator.get_manifest();
        if manifest.id != "YoYoVideo"
            || manifest.channel != config.platform.channel()
            || manifest.version.to_string() != env!("CARGO_PKG_VERSION")
        {
            return Err(UpdateError::Unsupported);
        }
        #[cfg(not(feature = "qa-fixture"))]
        let source = SignedSource::new(config.platform, config.public_key.clone());
        #[cfg(feature = "qa-fixture")]
        let source = SignedSource::from_qa_fixture(
            config.platform,
            config.public_key.clone(),
            crate::QaFixture::from_env()?,
        );
        let options = UpdateOptions {
            AllowVersionDowngrade: false,
            MaximumDeltasBeforeFallback: -1,
            ..Default::default()
        };
        #[cfg(feature = "qa-fixture")]
        let override_locator = Some(crate::QaFixture::from_env()?.native_locator()?);
        #[cfg(not(feature = "qa-fixture"))]
        let override_locator = None;
        let packages = override_locator
            .as_ref()
            .map(|l: &velopack::locator::VelopackLocatorConfig| l.PackagesDir.clone())
            .unwrap_or_else(|| locator.get_packages_dir());
        let manager = UpdateManager::new(source.clone(), Some(options), override_locator)
            .map_err(|_| UpdateError::Unsupported)?;
        Ok(Self {
            manager,
            source,
            version: manifest.version,
            packages,
            binaries: locator.get_current_bin_dir(),
        })
    }
}
impl Backend for NativeBackend {
    fn current_version(&self) -> &Version {
        &self.version
    }
    fn packages_dir(&self) -> &Path {
        &self.packages
    }
    fn check(&mut self) -> Result<Option<VerifiedManifest>, UpdateError> {
        match self.manager.check_for_updates().map_err(|e| UpdateError::Backend(e.to_string()))? {
            UpdateCheck::UpdateAvailable(info) => {
                let candidate =
                    self.source.snapshot().ok_or(UpdateError::State("no authenticated feed"))?;
                if serde_json::to_vec(&info.TargetFullRelease).ok()
                    != serde_json::to_vec(candidate.asset()).ok()
                {
                    return Err(UpdateError::State("SDK candidate differs from signed feed"));
                }
                Ok(Some(candidate))
            }
            _ => Ok(None),
        }
    }
    fn download(
        &mut self,
        candidate: &VerifiedManifest,
        progress: Sender<i16>,
    ) -> Result<(), UpdateError> {
        let info =
            UpdateInfo { TargetFullRelease: candidate.asset().clone(), ..Default::default() };
        self.manager
            .download_updates(&info, Some(progress))
            .map_err(|e| UpdateError::Backend(e.to_string()))
    }
    fn launch(&mut self, candidate: &VerifiedManifest) -> Result<(), UpdateError> {
        crate::process_guard::ensure_exclusive_installation(&self.binaries)?;
        self.manager
            .wait_exit_then_apply_updates(candidate.asset(), false, true, Vec::<String>::new())
            .map_err(|e| UpdateError::Install(e.to_string()))
    }
}
