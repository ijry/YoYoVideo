use super::{PinCredential, PrivacyDocument, PrivacyError, PrivacyStore};
use chrono::{DateTime, Duration, Local, Utc};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicU64, Ordering},
    },
};
use yoyo_core::{
    MediaLocator, PlaybackAccess,
    privacy::{ManualPrivacy, MediaKey, PrivacyDecision, PrivacySchedule},
};

pub(crate) trait PrivacyClock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}
struct SystemPrivacyClock;
impl PrivacyClock for SystemPrivacyClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthPurpose {
    Setup,
    Unlock,
    Settings,
    ChangePin,
}
/// Opaque and single-use. A caller cannot manufacture a verified PIN result.
pub struct AuthTicket {
    instance: u64,
    request: u64,
    epoch: u64,
    revision: u64,
    next_cycle: Option<DateTime<Utc>>,
    purpose: AuthPurpose,
}
#[derive(Clone, Debug)]
pub struct AuthGrant {
    instance: u64,
    epoch: u64,
    change_pin: bool,
}
pub struct PinVerification {
    ticket: AuthTicket,
    matched: Result<bool, PrivacyError>,
}
pub struct PreparedPinSetup {
    ticket: AuthTicket,
    credential: PinCredential,
}
pub struct PreparedPinChange {
    grant: AuthGrant,
    revision: u64,
    credential: PinCredential,
}
#[derive(Clone, Debug)]
pub struct PrivacySettings {
    pub schedule: PrivacySchedule,
    pub protected: BTreeMap<MediaKey, MediaLocator>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivacySnapshot {
    pub configured: bool,
    pub enabled: bool,
    pub manual: bool,
    pub next_start: Option<DateTime<Utc>>,
    pub blocked_until: Option<DateTime<Utc>>,
    pub cooldown_seconds: u32,
    pub fail_closed: bool,
    pub dirty: bool,
    pub epoch: u64,
}

struct State {
    document: PrivacyDocument,
    fail_closed: bool,
    dirty: bool,
    epoch: u64,
    request: u64,
    in_flight: Option<u64>,
    authorized: Option<u64>,
    next_cycle: Option<DateTime<Utc>>,
    decision: PrivacyDecision,
    cached_second: Option<i64>,
    cached_revision: u64,
}
#[derive(Clone)]
pub struct PrivacyService {
    instance: u64,
    state: Arc<Mutex<State>>,
    store: PrivacyStore,
    clock: Arc<dyn PrivacyClock>,
}

impl PrivacyService {
    pub fn load(store: PrivacyStore) -> Self {
        Self::from_store(store, Arc::new(SystemPrivacyClock))
    }

    #[cfg(any(test, feature = "privacy-qa"))]
    pub(crate) fn with_clock(store: PrivacyStore, clock: Arc<dyn PrivacyClock>) -> Self {
        Self::from_store(store, clock)
    }

    fn from_store(store: PrivacyStore, clock: Arc<dyn PrivacyClock>) -> Self {
        static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);
        let (document, fail_closed) = match store.load() {
            Ok(doc) => (doc, false),
            Err(_) => (PrivacyDocument::default(), true),
        };
        let now = clock.now();
        let decision = document.schedule.with_override(document.manual.as_ref(), now, &Local);
        let next_cycle = decision.next_start;
        let service = Self {
            instance: NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed),
            store,
            clock,
            state: Arc::new(Mutex::new(State {
                document,
                fail_closed,
                dirty: false,
                epoch: 0,
                request: 0,
                in_flight: None,
                authorized: None,
                next_cycle,
                decision,
                cached_second: None,
                cached_revision: 0,
            })),
        };
        service.tick();
        service
    }

    // State locks never cover Argon2 work. Disk mutations are serialized with
    // publication so an older unlock write cannot overwrite a newer manual ON.
    // Call the IO/hash methods from the window's background worker, not its UI callback.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poison| {
            let mut state = poison.into_inner();
            state.fail_closed = true;
            state.authorized = None;
            state
        })
    }

    fn invalidate(state: &mut State) {
        state.epoch = state.epoch.wrapping_add(1);
        state.authorized = None;
        // Keep the in-flight slot until its worker returns. Cancel must not allow
        // an unbounded number of parallel expensive PIN guesses.
    }

    fn refresh(state: &mut State, now: DateTime<Utc>) {
        if state.document.blocked_until.is_some_and(|until| now >= until) {
            state.document.blocked_until = None;
            state.document.failed_attempts = 0;
            state.dirty = true;
        }
        if state.cached_second == Some(now.timestamp())
            && state.cached_revision == state.document.revision
        {
            return;
        }
        let cycle_started = state.next_cycle.is_some_and(|start| now >= start);
        let manual_expired =
            state.document.manual.as_ref().is_some_and(|m| m.until.is_some_and(|end| now >= end));
        if cycle_started || manual_expired {
            Self::invalidate(state);
        }
        if manual_expired {
            state.document.manual = None;
            state.document.revision = state.document.revision.saturating_add(1);
            state.dirty = true;
        }
        state.decision =
            state.document.schedule.with_override(state.document.manual.as_ref(), now, &Local);
        state.next_cycle = state.decision.next_start;
        state.cached_revision = state.document.revision;
        state.cached_second = Some(now.timestamp());
    }

    pub fn tick(&self) {
        Self::refresh(&mut self.lock(), self.clock.now());
    }

    pub fn snapshot(&self) -> PrivacySnapshot {
        let now = self.clock.now();
        let mut state = self.lock();
        Self::refresh(&mut state, now);
        PrivacySnapshot {
            configured: state.document.credential.is_some(),
            enabled: state.fail_closed || state.decision.enabled,
            manual: state.decision.manual,
            next_start: state.next_cycle,
            blocked_until: state.document.blocked_until,
            cooldown_seconds: Self::cooldown(&state, now),
            fail_closed: state.fail_closed,
            dirty: state.dirty,
            epoch: state.epoch,
        }
    }

    fn cooldown(state: &State, now: DateTime<Utc>) -> u32 {
        state
            .document
            .blocked_until
            .map(|until| ((until - now).num_milliseconds().max(0) + 999) / 1000)
            .unwrap_or(0)
            .min(u32::MAX as i64) as u32
    }

    fn configured(state: &State) -> Result<(), PrivacyError> {
        if state.fail_closed {
            Err(PrivacyError::CorruptConfig)
        } else if state.document.credential.is_none() {
            Err(PrivacyError::Unconfigured)
        } else {
            Ok(())
        }
    }

    /// Immediate and PIN-free after setup. The caller conceals video immediately,
    /// then flushes in the background. Save failure never undoes this state.
    pub fn enable(&self) -> Result<(), PrivacyError> {
        let now = self.clock.now();
        let mut state = self.lock();
        Self::refresh(&mut state, now);
        Self::configured(&state)?;
        let until = state.document.schedule.evaluate(now, &Local).next_start;
        state.document.manual = Some(ManualPrivacy { enabled: true, until });
        state.document.revision = state.document.revision.saturating_add(1);
        state.dirty = true;
        Self::invalidate(&mut state);
        Self::refresh(&mut state, now);
        Ok(())
    }

    pub fn flush(&self) -> Result<(), PrivacyError> {
        let mut state = self.lock();
        Self::refresh(&mut state, self.clock.now());
        if state.fail_closed {
            return Ok(());
        } // Preserve the corrupt original; never replace it with defaults.
        if state.dirty {
            self.store.save(&state.document)?;
            state.dirty = false;
        }
        Ok(())
    }

    pub fn begin_auth(&self, purpose: AuthPurpose) -> Result<AuthTicket, PrivacyError> {
        let now = self.clock.now();
        let mut state = self.lock();
        Self::refresh(&mut state, now);
        if state.fail_closed {
            return Err(PrivacyError::CorruptConfig);
        }
        if state.in_flight.is_some() {
            return Err(PrivacyError::Busy);
        }
        if purpose == AuthPurpose::Setup {
            if state.document.credential.is_some() {
                return Err(PrivacyError::Locked);
            }
        } else {
            Self::configured(&state)?;
        }
        let cooldown = Self::cooldown(&state, now);
        if cooldown > 0 {
            return Err(PrivacyError::Cooldown(cooldown));
        }
        state.request = state.request.wrapping_add(1);
        state.in_flight = Some(state.request);
        Ok(AuthTicket {
            instance: self.instance,
            request: state.request,
            epoch: state.epoch,
            revision: state.document.revision,
            next_cycle: state.next_cycle,
            purpose,
        })
    }

    fn validate_ticket(
        &self,
        state: &State,
        ticket: &AuthTicket,
        now: DateTime<Utc>,
    ) -> Result<(), PrivacyError> {
        if state.fail_closed {
            return Err(PrivacyError::CorruptConfig);
        }
        if ticket.instance != self.instance
            || state.in_flight != Some(ticket.request)
            || ticket.epoch != state.epoch
            || ticket.revision != state.document.revision
            || ticket.next_cycle.is_some_and(|start| now >= start)
        {
            return Err(PrivacyError::Stale);
        }
        Ok(())
    }

    fn finish_slot(state: &mut State, ticket: &AuthTicket) {
        if state.in_flight == Some(ticket.request) {
            state.in_flight = None;
        }
    }

    fn fail_request(&self, ticket: &AuthTicket, error: PrivacyError) -> PrivacyError {
        let mut state = self.lock();
        if ticket.instance == self.instance {
            Self::finish_slot(&mut state, ticket);
        }
        error
    }

    pub fn prepare_setup(
        &self,
        ticket: AuthTicket,
        pin: &str,
        confirmation: &str,
    ) -> Result<PreparedPinSetup, PrivacyError> {
        if ticket.purpose != AuthPurpose::Setup {
            return Err(self.fail_request(&ticket, PrivacyError::Locked));
        }
        if !PinCredential::valid_pin(pin) || !PinCredential::valid_pin(confirmation) {
            return Err(self.fail_request(&ticket, PrivacyError::InvalidPin));
        }
        if pin != confirmation {
            return Err(self.fail_request(&ticket, PrivacyError::ConfirmationMismatch));
        }
        {
            let mut state = self.lock();
            Self::refresh(&mut state, self.clock.now());
            if let Err(error) = self.validate_ticket(&state, &ticket, self.clock.now()) {
                Self::finish_slot(&mut state, &ticket);
                return Err(error);
            }
        }
        let credential =
            PinCredential::create(pin).map_err(|error| self.fail_request(&ticket, error))?;
        Ok(PreparedPinSetup { ticket, credential })
    }

    pub fn apply_setup(&self, reply: PreparedPinSetup) -> Result<AuthGrant, PrivacyError> {
        let PreparedPinSetup { ticket, credential } = reply;
        let now = self.clock.now();
        let mut state = self.lock();
        Self::refresh(&mut state, now);
        let valid = self.validate_ticket(&state, &ticket, now);
        Self::finish_slot(&mut state, &ticket);
        valid?;
        if state.document.credential.is_some() {
            return Err(PrivacyError::Stale);
        }
        let mut candidate = state.document.clone();
        candidate.credential = Some(credential);
        candidate.revision = candidate.revision.saturating_add(1);
        self.replace_document(&mut state, candidate, false, now)?;
        state.authorized = Some(state.epoch);
        Ok(AuthGrant { instance: self.instance, epoch: state.epoch, change_pin: false })
    }

    pub fn setup(
        &self,
        ticket: AuthTicket,
        pin: &str,
        confirmation: &str,
    ) -> Result<AuthGrant, PrivacyError> {
        self.apply_setup(self.prepare_setup(ticket, pin, confirmation)?)
    }

    fn prepare_auth(&self, ticket: &AuthTicket) -> Result<PinCredential, PrivacyError> {
        let now = self.clock.now();
        let mut state = self.lock();
        Self::refresh(&mut state, now);
        if ticket.purpose == AuthPurpose::Setup {
            Self::finish_slot(&mut state, ticket);
            return Err(PrivacyError::Locked);
        }
        if let Err(error) = self.validate_ticket(&state, ticket, now) {
            Self::finish_slot(&mut state, ticket);
            return Err(error);
        }
        let credential = state.document.credential.clone().ok_or(PrivacyError::Unconfigured)?;
        // Reserve durably before expensive verification. Cancelling or crashing a
        // worker cannot erase a guess. A valid, current reply resets this counter.
        state.document.failed_attempts = state.document.failed_attempts.saturating_add(1).min(5);
        if state.document.failed_attempts == 5 {
            state.document.blocked_until = Some(now + Duration::seconds(30));
        }
        state.dirty = true;
        if let Err(error) = self.store.save(&state.document) {
            Self::finish_slot(&mut state, ticket);
            return Err(error);
        }
        state.dirty = false;
        Ok(credential)
    }

    /// Background phase: the PIN is checked, but no media or settings are released.
    pub fn verify_pin(
        &self,
        ticket: AuthTicket,
        pin: &str,
    ) -> Result<PinVerification, PrivacyError> {
        if !PinCredential::valid_pin(pin) {
            return Err(self.fail_request(&ticket, PrivacyError::InvalidPin));
        }
        let credential = self.prepare_auth(&ticket)?;
        let matched = credential.verify(pin);
        Ok(PinVerification { ticket, matched })
    }

    /// UI phase: only call after checking the dialog is still open and focused.
    /// Epoch/revision/deadline checks also reject cancelled or queued old replies.
    pub fn apply_verification(
        &self,
        reply: PinVerification,
    ) -> Result<Option<AuthGrant>, PrivacyError> {
        self.complete_auth(reply.ticket, reply.matched)
    }

    pub fn authenticate(
        &self,
        ticket: AuthTicket,
        pin: &str,
    ) -> Result<Option<AuthGrant>, PrivacyError> {
        self.apply_verification(self.verify_pin(ticket, pin)?)
    }

    fn complete_auth(
        &self,
        ticket: AuthTicket,
        matched: Result<bool, PrivacyError>,
    ) -> Result<Option<AuthGrant>, PrivacyError> {
        let now = self.clock.now();
        let mut state = self.lock();
        Self::refresh(&mut state, now);
        let valid = self.validate_ticket(&state, &ticket, now);
        Self::finish_slot(&mut state, &ticket);
        if !matched? {
            let cooldown = Self::cooldown(&state, now);
            return Err(if cooldown > 0 {
                PrivacyError::Cooldown(cooldown)
            } else {
                PrivacyError::IncorrectPin
            });
        }
        valid?;
        let mut candidate = state.document.clone();
        candidate.failed_attempts = 0;
        candidate.blocked_until = None;
        if ticket.purpose == AuthPurpose::Unlock {
            candidate.manual = Some(ManualPrivacy {
                enabled: false,
                until: candidate.schedule.evaluate(now, &Local).next_start,
            });
            candidate.revision = candidate.revision.saturating_add(1);
        }
        self.replace_document(&mut state, candidate, false, now)?;
        if ticket.purpose == AuthPurpose::Unlock {
            Self::invalidate(&mut state);
            Ok(None)
        } else {
            state.authorized = Some(state.epoch);
            Ok(Some(AuthGrant {
                instance: self.instance,
                epoch: state.epoch,
                change_pin: ticket.purpose == AuthPurpose::ChangePin,
            }))
        }
    }

    pub fn cancel_authorization(&self) {
        Self::invalidate(&mut self.lock());
    }

    fn validate_grant(&self, state: &State, grant: &AuthGrant) -> Result<(), PrivacyError> {
        Self::configured(state)?;
        if grant.instance != self.instance
            || grant.epoch != state.epoch
            || state.authorized != Some(grant.epoch)
        {
            Err(PrivacyError::Stale)
        } else {
            Ok(())
        }
    }

    pub fn settings(&self, grant: &AuthGrant) -> Result<PrivacySettings, PrivacyError> {
        let mut state = self.lock();
        Self::refresh(&mut state, self.clock.now());
        self.validate_grant(&state, grant)?;
        Ok(PrivacySettings {
            schedule: state.document.schedule.clone(),
            protected: state.document.protected.clone(),
        })
    }

    fn replace_document(
        &self,
        state: &mut State,
        candidate: PrivacyDocument,
        protect_first: bool,
        now: DateTime<Utc>,
    ) -> Result<(), PrivacyError> {
        candidate.validate()?;
        if protect_first {
            state.document = candidate;
            state.dirty = true;
            state.cached_second = None;
            Self::refresh(state, now);
            self.store.save(&state.document)?;
        } else {
            if let Err(error) = self.store.save(&candidate) {
                // Also restore the restrictive in-memory document on the next
                // flush if a filesystem reported an error after atomic replacement.
                state.dirty = true;
                return Err(error);
            }
            state.document = candidate;
            state.cached_second = None;
            Self::refresh(state, now);
        }
        state.dirty = false;
        Ok(())
    }

    pub fn set_schedule(
        &self,
        grant: &AuthGrant,
        schedule: PrivacySchedule,
    ) -> Result<(), PrivacyError> {
        schedule.validate().map_err(|_| PrivacyError::InvalidSchedule)?;
        let now = self.clock.now();
        let mut state = self.lock();
        Self::refresh(&mut state, now);
        self.validate_grant(&state, grant)?;
        let before = state.decision.enabled;
        let mut candidate = state.document.clone();
        candidate.schedule = schedule;
        if let Some(manual) = candidate.manual.as_mut() {
            manual.until = candidate.schedule.evaluate(now, &Local).next_start;
        }
        candidate.revision = candidate.revision.saturating_add(1);
        let after =
            candidate.schedule.with_override(candidate.manual.as_ref(), now, &Local).enabled;
        let result = self.replace_document(&mut state, candidate, after, now);
        if after && !before {
            Self::invalidate(&mut state);
        }
        result
    }

    pub fn set_protection(
        &self,
        grant: &AuthGrant,
        locator: MediaLocator,
        protected: bool,
    ) -> Result<(), PrivacyError> {
        let key = MediaKey::from_locator(&locator).map_err(|_| PrivacyError::InvalidMedia)?;
        if !protected {
            return self.remove_protection(grant, &key);
        }
        let now = self.clock.now();
        let mut state = self.lock();
        Self::refresh(&mut state, now);
        self.validate_grant(&state, grant)?;
        let mut candidate = state.document.clone();
        candidate.protected.insert(key, locator);
        candidate.revision = candidate.revision.saturating_add(1);
        self.replace_document(&mut state, candidate, true, now)
    }

    pub fn remove_protection(&self, grant: &AuthGrant, key: &MediaKey) -> Result<(), PrivacyError> {
        let now = self.clock.now();
        let mut state = self.lock();
        Self::refresh(&mut state, now);
        self.validate_grant(&state, grant)?;
        let mut candidate = state.document.clone();
        candidate.protected.remove(key);
        candidate.revision = candidate.revision.saturating_add(1);
        self.replace_document(&mut state, candidate, false, now)
    }

    pub fn prepare_pin_change(
        &self,
        grant: &AuthGrant,
        pin: &str,
        confirmation: &str,
    ) -> Result<PreparedPinChange, PrivacyError> {
        if !PinCredential::valid_pin(pin) || !PinCredential::valid_pin(confirmation) {
            return Err(PrivacyError::InvalidPin);
        }
        if pin != confirmation {
            return Err(PrivacyError::ConfirmationMismatch);
        }
        let revision = {
            let mut state = self.lock();
            Self::refresh(&mut state, self.clock.now());
            self.validate_grant(&state, grant)?;
            if !grant.change_pin {
                return Err(PrivacyError::Locked);
            }
            state.document.revision
        };
        let credential = PinCredential::create(pin)?;
        Ok(PreparedPinChange { grant: grant.clone(), revision, credential })
    }

    pub fn apply_pin_change(&self, reply: PreparedPinChange) -> Result<(), PrivacyError> {
        let PreparedPinChange { grant, revision, credential } = reply;
        let now = self.clock.now();
        let mut state = self.lock();
        Self::refresh(&mut state, now);
        self.validate_grant(&state, &grant)?;
        if state.document.revision != revision {
            return Err(PrivacyError::Stale);
        }
        let mut candidate = state.document.clone();
        candidate.credential = Some(credential);
        candidate.revision = candidate.revision.saturating_add(1);
        self.replace_document(&mut state, candidate, false, now)?;
        Self::invalidate(&mut state);
        Ok(())
    }

    pub fn change_pin(
        &self,
        grant: &AuthGrant,
        pin: &str,
        confirmation: &str,
    ) -> Result<(), PrivacyError> {
        self.apply_pin_change(self.prepare_pin_change(grant, pin, confirmation)?)
    }
}

impl PlaybackAccess for PrivacyService {
    fn restricted(&self, key: &MediaKey) -> bool {
        let mut state = self.lock();
        if state.fail_closed {
            return true;
        }
        if !state.document.protected.contains_key(key) {
            return false;
        }
        Self::refresh(&mut state, self.clock.now());
        state.decision.enabled
    }
}

#[cfg(test)]
mod tests;
