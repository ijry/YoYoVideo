use yoyo_core::{BackendCommand, BackendEvent, MediaLocator, PlayerBackend};

use crate::{MpvAction, MpvError, MpvEvent, map_event, translate_command, translate_open};

pub trait MpvActionSink {
    fn command(&mut self, args: &[String]) -> Result<(), MpvError>;
    fn set_flag(&mut self, name: &str, value: bool) -> Result<(), MpvError>;
    fn set_string(&mut self, name: &str, value: &str) -> Result<(), MpvError>;
    fn set_i64(&mut self, name: &str, value: i64) -> Result<(), MpvError>;
    fn set_f64(&mut self, name: &str, value: f64) -> Result<(), MpvError>;
}

pub fn execute_actions<S: MpvActionSink>(
    sink: &mut S,
    actions: &[MpvAction],
) -> Result<(), MpvError> {
    for action in actions {
        match action {
            MpvAction::Command(args) => sink.command(args)?,
            MpvAction::SetString { name, value } => sink.set_string(name, value)?,
            MpvAction::SetInt { name, value } => sink.set_i64(name, *value)?,
            MpvAction::SetDouble { name, value } => sink.set_f64(name, *value)?,
            MpvAction::SetFlag { name, value } => sink.set_flag(name, *value)?,
        }
    }
    Ok(())
}

#[derive(Default)]
struct RecordingSink {
    actions: Vec<String>,
}

impl MpvActionSink for RecordingSink {
    fn command(&mut self, args: &[String]) -> Result<(), MpvError> {
        self.actions.push(format!("Command({args:?})"));
        Ok(())
    }

    fn set_flag(&mut self, name: &str, value: bool) -> Result<(), MpvError> {
        self.actions.push(format!("SetFlag {{ name: \"{name}\", value: {value} }}"));
        Ok(())
    }

    fn set_string(&mut self, name: &str, value: &str) -> Result<(), MpvError> {
        self.actions.push(format!("SetString {{ name: \"{name}\", value: \"{value}\" }}"));
        Ok(())
    }

    fn set_i64(&mut self, name: &str, value: i64) -> Result<(), MpvError> {
        self.actions.push(format!("SetInt {{ name: \"{name}\", value: {value} }}"));
        Ok(())
    }

    fn set_f64(&mut self, name: &str, value: f64) -> Result<(), MpvError> {
        self.actions.push(format!("SetDouble {{ name: \"{name}\", value: {value} }}"));
        Ok(())
    }
}

#[derive(Default)]
pub struct DryRunMpvBackend {
    pending_events: Vec<BackendEvent>,
    sink: RecordingSink,
}

impl DryRunMpvBackend {
    pub fn recorded_actions(&self) -> &[String] {
        &self.sink.actions
    }

    pub fn push_event(&mut self, event: MpvEvent) {
        if let Some(mapped) = map_event(event) {
            self.pending_events.push(mapped);
        }
    }
}

impl PlayerBackend for DryRunMpvBackend {
    fn open(&mut self, locator: &MediaLocator) -> Result<(), String> {
        execute_actions(&mut self.sink, &translate_open(locator)).map_err(|error| error.to_string())
    }

    fn send(&mut self, command: BackendCommand) -> Result<(), String> {
        execute_actions(&mut self.sink, &translate_command(&command))
            .map_err(|error| error.to_string())
    }

    fn drain_events(&mut self) -> Vec<BackendEvent> {
        std::mem::take(&mut self.pending_events)
    }
}

pub type MpvBackend = DryRunMpvBackend;
