use super::service::{PinVerification, PreparedPinChange, PreparedPinSetup};
use super::view::{error_text, parse_hhmm, rule_label, status_text};
use super::{AuthGrant, AuthPurpose, PrivacyError, PrivacyService};
use crate::{PrivacyWindow, UiLanguage};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel, winit_030::WinitWindowAccessor};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::{Arc, mpsc},
    time::Duration,
};
use yoyo_core::{
    MediaLocator,
    privacy::{MediaKey, PrivacySchedule, PrivacyTimeRule},
};
use zeroize::Zeroizing;

pub fn bind_pin_validation(window: &PrivacyWindow) {
    window.on_validate_pin(|pin| super::PinCredential::valid_pin(pin.as_str()));
}

#[derive(Clone)]
pub(crate) enum DialogIntent {
    Enable,
    Unlock,
    Settings,
    Protect(MediaLocator, bool),
    ChangePin,
}
enum Job {
    Verified(PinVerification),
    Setup(PreparedPinSetup),
    Change(PreparedPinChange),
}
struct Pending {
    serial: u64,
    intent: DialogIntent,
    receiver: mpsc::Receiver<Result<Job, PrivacyError>>,
}

pub(crate) struct PrivacyUi {
    window: PrivacyWindow,
    service: Arc<PrivacyService>,
    grant: Option<AuthGrant>,
    language: UiLanguage,
    intent: DialogIntent,
    open: bool,
    serial: u64,
    pending: Option<Pending>,
    draft_rules: Vec<PrivacyTimeRule>,
    protected_keys: Vec<MediaKey>,
    notify: Rc<dyn Fn(Option<PrivacyError>)>,
    self_weak: Weak<RefCell<Self>>,
    timer: slint::Timer,
    flush_sender: mpsc::Sender<Result<(), PrivacyError>>,
    flush_receiver: mpsc::Receiver<Result<(), PrivacyError>>,
}

fn strings(values: impl IntoIterator<Item = String>) -> ModelRc<SharedString> {
    Rc::new(VecModel::from(values.into_iter().map(SharedString::from).collect::<Vec<_>>())).into()
}

impl PrivacyUi {
    pub(crate) fn new(
        service: Arc<PrivacyService>,
        notify: Rc<dyn Fn(Option<PrivacyError>)>,
    ) -> Result<Rc<RefCell<Self>>, slint::PlatformError> {
        let window = PrivacyWindow::new()?;
        bind_pin_validation(&window);
        let (flush_sender, flush_receiver) = mpsc::channel();
        let ui = Rc::new_cyclic(|weak| {
            RefCell::new(Self {
                window,
                service: service.clone(),
                grant: None,
                language: UiLanguage::Chinese,
                intent: DialogIntent::Settings,
                open: false,
                serial: 0,
                pending: None,
                draft_rules: Vec::new(),
                protected_keys: Vec::new(),
                notify,
                self_weak: weak.clone(),
                timer: slint::Timer::default(),
                flush_sender,
                flush_receiver,
            })
        });
        let window = ui.borrow().window.clone_strong();
        window.on_submit_requested({
            let weak = Rc::downgrade(&ui);
            move || {
                if let Some(ui) = weak.upgrade() {
                    ui.borrow_mut().submit();
                }
            }
        });
        window.on_cancel_requested({
            let weak = Rc::downgrade(&ui);
            move || {
                if let Some(ui) = weak.upgrade() {
                    ui.borrow_mut().revoke(true);
                }
            }
        });
        window.on_add_rule_requested({
            let weak = Rc::downgrade(&ui);
            move || {
                if let Some(ui) = weak.upgrade() {
                    ui.borrow_mut().add_rule();
                }
            }
        });
        window.on_remove_rule_requested({
            let weak = Rc::downgrade(&ui);
            move |index| {
                if let Some(ui) = weak.upgrade() {
                    ui.borrow_mut().remove_rule(index);
                }
            }
        });
        window.on_save_schedule_requested({
            let weak = Rc::downgrade(&ui);
            move || {
                if let Some(ui) = weak.upgrade() {
                    ui.borrow_mut().save_schedule();
                }
            }
        });
        window.on_remove_protection_requested({
            let weak = Rc::downgrade(&ui);
            move |index| {
                if let Some(ui) = weak.upgrade() {
                    ui.borrow_mut().remove_protection(index);
                }
            }
        });
        window.on_change_pin_requested({
            let weak = Rc::downgrade(&ui);
            move || {
                if let Some(ui) = weak.upgrade() {
                    let mut ui = ui.borrow_mut();
                    let language = ui.language;
                    if let Err(error) = ui.present(DialogIntent::ChangePin, language) {
                        ui.report_error(error);
                    }
                }
            }
        });
        window.window().on_close_requested({
            let weak = Rc::downgrade(&ui);
            move || {
                if let Some(ui) = weak.upgrade() {
                    ui.borrow_mut().revoke(true);
                }
                slint::CloseRequestResponse::HideWindow
            }
        });
        window.window().on_winit_window_event({
            let weak = Rc::downgrade(&ui);
            let service = service.clone();
            move |_, event| {
                use slint::winit_030::winit::{
                    event::{ElementState, WindowEvent},
                    keyboard::{Key, NamedKey},
                };
                if matches!(event, WindowEvent::Focused(false)) {
                    service.cancel_authorization();
                    if let Some(ui) = weak.upgrade() {
                        if let Ok(mut ui) = ui.try_borrow_mut() {
                            ui.revoke(false);
                        }
                    }
                }
                if let WindowEvent::KeyboardInput { event, .. } = event {
                    if event.state == ElementState::Pressed
                        && event.logical_key == Key::Named(NamedKey::Escape)
                    {
                        if let Some(ui) = weak.upgrade() {
                            if let Ok(mut ui) = ui.try_borrow_mut() {
                                ui.revoke(true);
                            }
                        }
                        return slint::winit_030::EventResult::PreventDefault;
                    }
                }
                // Deliberately no player shortcut routing in this window.
                slint::winit_030::EventResult::Propagate
            }
        });
        ui.borrow().timer.start(slint::TimerMode::Repeated, Duration::from_millis(100), {
            let weak = Rc::downgrade(&ui);
            move || {
                if let Some(ui) = weak.upgrade() {
                    if let Ok(mut ui) = ui.try_borrow_mut() {
                        ui.poll();
                    }
                }
            }
        });
        Ok(ui)
    }

    pub(crate) fn toggle(&mut self, language: UiLanguage) -> Result<(), PrivacyError> {
        self.language = language;
        let snapshot = self.service.snapshot();
        if snapshot.fail_closed {
            return Err(PrivacyError::CorruptConfig);
        }
        if !snapshot.configured {
            return self.present(DialogIntent::Enable, language);
        }
        if snapshot.enabled {
            self.present(DialogIntent::Unlock, language)
        } else {
            self.enable_now()
        }
    }

    fn enable_now(&mut self) -> Result<(), PrivacyError> {
        self.service.enable()?;
        self.revoke(true);
        (self.notify)(None); // Hide/pause on the UI thread before asking the disk to save.
        let service = self.service.clone();
        let sender = self.flush_sender.clone();
        std::thread::spawn(move || {
            let _ = sender.send(service.flush());
        });
        Ok(())
    }

    pub(crate) fn present(
        &mut self,
        intent: DialogIntent,
        language: UiLanguage,
    ) -> Result<(), PrivacyError> {
        if self.service.snapshot().fail_closed {
            return Err(PrivacyError::CorruptConfig);
        }
        self.revoke(false);
        self.language = language;
        self.intent = intent;
        self.open = true;
        self.window.set_ui_language_code(language.code().into());
        self.window.set_mode(if self.service.snapshot().configured { 1 } else { 0 });
        self.window.set_error_message("".into());
        self.window.set_intent_label(
            match (&self.intent, language) {
                (DialogIntent::Unlock, UiLanguage::Chinese) => {
                    "验证后关闭隐私模式；不会自动恢复播放。"
                }
                (DialogIntent::Unlock, UiLanguage::English) => {
                    "Verify to turn privacy off. Playback will remain paused."
                }
                (DialogIntent::ChangePin, UiLanguage::Chinese) => {
                    "请先验证当前 PIN，再设置新的 PIN。"
                }
                (DialogIntent::ChangePin, UiLanguage::English) => {
                    "Verify your current PIN before choosing a new one."
                }
                (DialogIntent::Protect(..), UiLanguage::Chinese) => {
                    "验证 PIN 后修改所选媒体的保护状态。"
                }
                (DialogIntent::Protect(..), UiLanguage::English) => {
                    "Verify your PIN to change protection for the selected media."
                }
                (_, UiLanguage::Chinese) => "输入 4 位数字（0–9）。首次使用需再次确认。",
                (_, UiLanguage::English) => "Enter 4 digits (0–9). Confirm the PIN on first use.",
            }
            .into(),
        );
        self.window.window().set_size(slint::LogicalSize::new(520.0, 420.0));
        self.window.show().map_err(|_| PrivacyError::WindowUnavailable)?;
        self.window.window().with_winit_window(|window| window.focus_window());
        Ok(())
    }

    /// Clear the actual models as well as their visibility. Stale accessibility
    /// nodes or a later focus/resize must not recover authorized filenames.
    fn revoke(&mut self, hide: bool) {
        self.service.cancel_authorization();
        self.grant = None;
        self.serial = self.serial.wrapping_add(1);
        self.window.set_pin("".into());
        self.window.set_confirmation("".into());
        self.window.set_protected_items(strings(Vec::new()));
        self.window.set_rules(strings(Vec::new()));
        self.protected_keys.clear();
        self.draft_rules.clear();
        self.window.set_mode(if self.service.snapshot().configured { 1 } else { 0 });
        self.window.set_busy(self.pending.is_some());
        if hide {
            self.open = false;
            let weak = self.self_weak.clone();
            let serial = self.serial;
            // Defer native operations out of callbacks/RefCell borrows (AppKit).
            slint::Timer::single_shot(Duration::ZERO, move || {
                let window = weak.upgrade().and_then(|ui| {
                    let ui = ui.try_borrow().ok()?;
                    (ui.serial == serial && !ui.open).then(|| ui.window.clone_strong())
                });
                if let Some(window) = window {
                    let _ = window.hide();
                }
            });
        }
    }

    pub(crate) fn cancel_for_exit(&mut self) {
        self.revoke(true);
    }

    fn report_error(&mut self, error: PrivacyError) {
        if matches!(error, PrivacyError::Stale | PrivacyError::Locked | PrivacyError::CorruptConfig)
        {
            self.revoke(false);
        }
        self.window.set_error_message(error_text(&error, self.language).into());
        (self.notify)(Some(error));
    }

    fn live_grant(&self) -> Result<AuthGrant, PrivacyError> {
        let grant = self.grant.clone().ok_or(PrivacyError::Locked)?;
        self.service.settings(&grant)?;
        Ok(grant)
    }

    fn show_settings(&mut self) -> Result<(), PrivacyError> {
        let grant = self.live_grant()?;
        let settings = self.service.settings(&grant)?;
        self.draft_rules = settings.schedule.rules;
        self.protected_keys = settings.protected.keys().cloned().collect();
        self.window
            .set_protected_items(strings(settings.protected.values().map(MediaLocator::as_label)));
        self.window.set_schedule_enabled(settings.schedule.enabled);
        self.refresh_rule_rows();
        self.window.set_pin("".into());
        self.window.set_confirmation("".into());
        self.window.set_mode(2);
        self.window.set_privacy_status(status_text(&self.service.snapshot(), self.language).into());
        self.window.window().set_size(slint::LogicalSize::new(520.0, 620.0));
        Ok(())
    }

    fn refresh_rule_rows(&self) {
        self.window.set_rules(strings(
            self.draft_rules.iter().map(|rule| rule_label(rule, self.language)),
        ));
    }

    fn add_rule(&mut self) {
        let result = (|| {
            self.live_grant()?;
            let days = [
                self.window.get_monday(),
                self.window.get_tuesday(),
                self.window.get_wednesday(),
                self.window.get_thursday(),
                self.window.get_friday(),
                self.window.get_saturday(),
                self.window.get_sunday(),
            ];
            let weekdays = days
                .into_iter()
                .enumerate()
                .fold(0u8, |mask, (index, on)| mask | if on { 1 << index } else { 0 });
            let rule = PrivacyTimeRule {
                weekdays,
                start_minute: parse_hhmm(self.window.get_start_time().as_str())?,
                end_minute: parse_hhmm(self.window.get_end_time().as_str())?,
            };
            let mut rules = self.draft_rules.clone();
            rules.push(rule);
            PrivacySchedule { enabled: true, rules: rules.clone() }
                .validate()
                .map_err(|_| PrivacyError::InvalidSchedule)?;
            self.draft_rules = rules;
            self.refresh_rule_rows();
            self.window.set_error_message("".into());
            Ok(())
        })();
        if let Err(error) = result {
            self.report_error(error);
        }
    }

    fn remove_rule(&mut self, index: i32) {
        if let Err(error) = self.live_grant() {
            self.report_error(error);
            return;
        }
        if let Ok(index) = usize::try_from(index) {
            if index < self.draft_rules.len() {
                self.draft_rules.remove(index);
                self.refresh_rule_rows();
            }
        }
    }

    fn save_schedule(&mut self) {
        let result = self.live_grant().and_then(|grant| {
            self.service.set_schedule(
                &grant,
                PrivacySchedule {
                    enabled: self.window.get_schedule_enabled(),
                    rules: self.draft_rules.clone(),
                },
            )
        });
        match result {
            Ok(()) => {
                self.window.set_error_message("".into());
                (self.notify)(None);
            }
            Err(error) => self.report_error(error),
        }
    }

    fn remove_protection(&mut self, index: i32) {
        let Some(key) =
            usize::try_from(index).ok().and_then(|index| self.protected_keys.get(index)).cloned()
        else {
            return;
        };
        let result = self
            .live_grant()
            .and_then(|grant| self.service.remove_protection(&grant, &key))
            .and_then(|()| self.show_settings());
        match result {
            Ok(()) => (self.notify)(None),
            Err(error) => self.report_error(error),
        }
    }

    fn submit(&mut self) {
        if self.pending.is_some() || !self.window.get_submit_enabled() {
            return;
        }
        let pin = Zeroizing::new(self.window.get_pin().to_string());
        let confirmation = Zeroizing::new(self.window.get_confirmation().to_string());
        self.window.set_pin("".into());
        self.window.set_confirmation("".into());
        self.window.set_error_message("".into());
        let service = self.service.clone();
        let mode = self.window.get_mode();
        if mode == 3 {
            let grant = match self.live_grant() {
                Ok(grant) => grant,
                Err(error) => {
                    self.report_error(error);
                    return;
                }
            };
            self.start_job(move || {
                service.prepare_pin_change(&grant, &pin, &confirmation).map(Job::Change)
            });
            return;
        }
        let purpose = if mode == 0 {
            AuthPurpose::Setup
        } else {
            match self.intent {
                DialogIntent::Unlock => AuthPurpose::Unlock,
                DialogIntent::ChangePin => AuthPurpose::ChangePin,
                _ => AuthPurpose::Settings,
            }
        };
        let ticket = match service.begin_auth(purpose) {
            Ok(ticket) => ticket,
            Err(error) => {
                self.report_error(error);
                return;
            }
        };
        if mode == 0 {
            self.start_job(move || {
                service.prepare_setup(ticket, &pin, &confirmation).map(Job::Setup)
            });
        } else {
            self.start_job(move || service.verify_pin(ticket, &pin).map(Job::Verified));
        }
    }

    fn start_job(&mut self, task: impl FnOnce() -> Result<Job, PrivacyError> + Send + 'static) {
        let (sender, receiver) = mpsc::channel();
        self.pending = Some(Pending { serial: self.serial, intent: self.intent.clone(), receiver });
        self.window.set_busy(true);
        std::thread::spawn(move || {
            let _ = sender.send(task());
        });
    }

    fn is_focused(&self) -> bool {
        let visible = self.open && self.window.window().is_visible();
        if !visible {
            return false;
        }
        #[cfg(feature = "privacy-qa")]
        if std::env::var_os("YOYOVIDEO_PRIVACY_QA_FOCUS").is_some() {
            // Xvfb has no window manager to grant focus. This switch is
            // compiled only into privacy-qa builds and does not bypass PIN
            // verification; it only lets the isolated native driver exercise
            // the same visible-window callback path.
            return true;
        }
        self.window.window().with_winit_window(|window| window.has_focus()).unwrap_or(false)
    }

    fn poll(&mut self) {
        for result in self.flush_receiver.try_iter().collect::<Vec<_>>() {
            if let Err(error) = result {
                (self.notify)(Some(error));
            }
        }
        let snapshot = self.service.snapshot();
        self.window.set_cooldown_seconds(snapshot.cooldown_seconds.min(i32::MAX as u32) as i32);
        self.window.set_privacy_status(status_text(&snapshot, self.language).into());
        if self.grant.is_some()
            && (!self.is_focused()
                || self.grant.as_ref().is_some_and(|grant| self.service.settings(grant).is_err()))
        {
            self.revoke(false);
            self.window.set_error_message(error_text(&PrivacyError::Stale, self.language).into());
        }
        let Some(pending) = self.pending.take() else {
            return;
        };
        let reply = match pending.receiver.try_recv() {
            Ok(reply) => reply,
            Err(mpsc::TryRecvError::Empty) => {
                self.pending = Some(pending);
                return;
            }
            Err(mpsc::TryRecvError::Disconnected) => Err(PrivacyError::Stale),
        };
        self.window.set_busy(false);
        let live = pending.serial == self.serial && self.is_focused();
        if !live {
            self.service.cancel_authorization();
        }
        let mut changed_pin = false;
        let result = reply.and_then(|job| match job {
            Job::Verified(reply) => self.service.apply_verification(reply),
            Job::Setup(reply) => self.service.apply_setup(reply).map(Some),
            Job::Change(reply) => {
                changed_pin = true;
                self.service.apply_pin_change(reply).map(|()| None)
            }
        });
        if !live {
            return;
        }
        match result {
            Err(error) => self.report_error(error),
            Ok(grant) => {
                self.grant = grant;
                if changed_pin {
                    self.revoke(true);
                    (self.notify)(None);
                    return;
                }
                let result = match pending.intent {
                    DialogIntent::Unlock => {
                        self.revoke(true);
                        Ok(())
                    }
                    DialogIntent::Enable => self.enable_now(),
                    DialogIntent::Settings => self.show_settings(),
                    DialogIntent::ChangePin => {
                        self.window.set_mode(3);
                        self.window.set_intent_label(
                            if self.language == UiLanguage::Chinese {
                                "旧 PIN 已验证，请输入并确认新的 4 位 PIN。"
                            } else {
                                "Current PIN verified. Enter and confirm your new 4-digit PIN."
                            }
                            .into(),
                        );
                        Ok(())
                    }
                    DialogIntent::Protect(locator, protected) => self
                        .live_grant()
                        .and_then(|grant| self.service.set_protection(&grant, locator, protected))
                        .map(|()| self.revoke(true)),
                };
                match result {
                    Ok(()) => (self.notify)(None),
                    Err(error) => self.report_error(error),
                }
            }
        }
    }

    #[cfg(feature = "privacy-qa")]
    pub(crate) fn qa_window(&self) -> PrivacyWindow {
        self.window.clone_strong()
    }
}

#[cfg(test)]
mod tests;
