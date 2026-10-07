use crate::{Platform, UpdateError, VerifiedManifest, pending::PendingCache};
use semver::Version;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
mod native;

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub platform: Platform,
    pub public_key: String,
    pub cache_dir: PathBuf,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum UpdatePhase {
    Unsupported,
    Idle,
    Checking,
    UpToDate,
    Available,
    Downloading,
    ReadyToInstall,
    PreparingInstall,
    Error,
}
#[derive(Debug, Clone)]
pub struct UpdateSnapshot {
    pub phase: UpdatePhase,
    pub version: String,
    pub notes: String,
    pub progress: i16,
    pub error: String,
}
impl UpdateSnapshot {
    pub fn empty(phase: UpdatePhase) -> Self {
        Self {
            phase,
            version: String::new(),
            notes: String::new(),
            progress: 0,
            error: String::new(),
        }
    }
}
trait Backend: Send {
    fn current_version(&self) -> &Version;
    fn packages_dir(&self) -> &Path;
    fn check(&mut self) -> Result<Option<VerifiedManifest>, UpdateError>;
    fn download(
        &mut self,
        candidate: &VerifiedManifest,
        progress: Sender<i16>,
    ) -> Result<(), UpdateError>;
    fn launch(&mut self, candidate: &VerifiedManifest) -> Result<(), UpdateError>;
}
pub struct UpdateService {
    config: ServiceConfig,
    backend: Box<dyn Backend>,
    cache: PendingCache,
    candidate: Option<VerifiedManifest>,
    prepared: bool,
    state: UpdateSnapshot,
}
impl UpdateService {
    pub fn new(config: ServiceConfig) -> Result<Self, UpdateError> {
        let backend = native::NativeBackend::new(&config)?;
        Ok(Self::with_backend(config, Box::new(backend)))
    }
    fn with_backend(config: ServiceConfig, backend: Box<dyn Backend>) -> Self {
        let cache = PendingCache::new(&config.cache_dir);
        let mut service = Self {
            config,
            backend,
            cache,
            candidate: None,
            prepared: false,
            state: UpdateSnapshot::empty(UpdatePhase::Idle),
        };
        match service.cache.load(
            service.config.platform,
            &service.config.public_key,
            service.backend.current_version(),
            service.backend.packages_dir(),
        ) {
            Ok(Some(candidate)) => service.set_candidate(candidate, UpdatePhase::ReadyToInstall),
            Ok(None) => {}
            Err(error) => service.set_error(&error),
        }
        service
    }
    pub fn snapshot(&self) -> UpdateSnapshot {
        self.state.clone()
    }
    pub fn check(&mut self) -> Result<UpdateSnapshot, UpdateError> {
        self.candidate = None;
        self.prepared = false;
        self.state = UpdateSnapshot::empty(UpdatePhase::Checking);
        let result = (|| {
            self.cache.clear()?;
            if let Some(candidate) = self.backend.check()? {
                let candidate = self.authenticate(&candidate)?;
                if candidate.is_newer_than(self.backend.current_version()) {
                    self.set_candidate(candidate, UpdatePhase::Available);
                    return Ok(self.snapshot());
                }
            }
            self.state = UpdateSnapshot::empty(UpdatePhase::UpToDate);
            Ok(self.snapshot())
        })();
        self.finish(result)
    }
    pub fn download(&mut self, progress: Sender<i16>) -> Result<(), UpdateError> {
        let result = (|| {
            if self.state.phase != UpdatePhase::Available {
                return Err(UpdateError::State("no available update to download"));
            }
            let candidate = self.candidate.clone().ok_or(UpdateError::State("no candidate"))?;
            self.state.phase = UpdatePhase::Downloading;
            self.state.error.clear();
            self.backend.download(&candidate, progress)?;
            let checked = self.validate_candidate()?;
            self.cache.save(&checked)?;
            self.set_candidate(checked, UpdatePhase::ReadyToInstall);
            Ok(())
        })();
        self.finish(result)
    }
    pub fn verify_for_install(&mut self) -> Result<(), UpdateError> {
        self.prepared = false;
        let result = (|| {
            if self.state.phase != UpdatePhase::ReadyToInstall {
                return Err(UpdateError::State("update is not ready"));
            }
            let checked = self.validate_candidate()?;
            self.set_candidate(checked, UpdatePhase::PreparingInstall);
            self.prepared = true;
            Ok(())
        })();
        self.finish(result)
    }
    pub fn launch_installer(&mut self) -> Result<(), UpdateError> {
        let authorized = std::mem::replace(&mut self.prepared, false);
        let result = (|| {
            if !authorized || self.state.phase != UpdatePhase::PreparingInstall {
                return Err(UpdateError::State("installation was not authorized"));
            }
            let checked = self.validate_candidate()?;
            self.backend.launch(&checked)
        })();
        self.finish(result)
    }
    pub fn abort_install(&mut self, reason: String) {
        self.prepared = false;
        if self.state.phase == UpdatePhase::PreparingInstall {
            self.state.phase = UpdatePhase::ReadyToInstall;
            self.state.error = reason;
        }
    }
    fn authenticate(&self, candidate: &VerifiedManifest) -> Result<VerifiedManifest, UpdateError> {
        VerifiedManifest::verify(
            candidate.raw_bytes(),
            candidate.signature(),
            &self.config.public_key,
            self.config.platform,
        )
    }
    fn validate_candidate(&self) -> Result<VerifiedManifest, UpdateError> {
        let candidate =
            self.authenticate(self.candidate.as_ref().ok_or(UpdateError::State("no candidate"))?)?;
        if !candidate.is_newer_than(self.backend.current_version()) {
            return Err(UpdateError::State("update is not newer"));
        }
        candidate.verify_package(&self.backend.packages_dir().join(&candidate.asset().FileName))?;
        Ok(candidate)
    }
    fn set_candidate(&mut self, candidate: VerifiedManifest, phase: UpdatePhase) {
        self.state = UpdateSnapshot {
            phase,
            version: candidate.manifest().version.clone(),
            notes: candidate.asset().NotesMarkdown.clone(),
            progress: if phase == UpdatePhase::Available { 0 } else { 100 },
            error: String::new(),
        };
        self.candidate = Some(candidate);
    }
    fn set_error(&mut self, error: &UpdateError) {
        self.prepared = false;
        self.state.phase = UpdatePhase::Error;
        self.state.error = error.to_string();
    }
    fn finish<T>(&mut self, result: Result<T, UpdateError>) -> Result<T, UpdateError> {
        if let Err(error) = &result {
            self.set_error(error);
        }
        result
    }
}
#[cfg(test)]
mod tests;
