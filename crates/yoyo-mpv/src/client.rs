use yoyo_core::{BackendCommand, BackendEvent, MediaLocator, PlayerBackend};

use crate::{
    MpvAction, MpvError, MpvEvent, MpvRenderBridge, map_event, translate_command, translate_open,
};

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

pub struct MpvBackend {
    client: MpvClient,
    pending_events: Vec<BackendEvent>,
    #[allow(dead_code)]
    render_bridge: MpvRenderBridge,
}

impl MpvBackend {
    pub fn new_runtime() -> Result<Self, MpvError> {
        let mut client = MpvClient::new()?;
        client.observe_default_properties()?;
        Ok(Self { client, pending_events: Vec::new(), render_bridge: MpvRenderBridge::default() })
    }

    pub fn render_bridge(&mut self) -> &mut MpvRenderBridge {
        &mut self.render_bridge
    }
}

#[cfg(not(feature = "mpv-runtime"))]
impl Default for MpvBackend {
    fn default() -> Self {
        Self {
            client: MpvClient,
            pending_events: Vec::new(),
            render_bridge: MpvRenderBridge::default(),
        }
    }
}

impl PlayerBackend for MpvBackend {
    fn open(&mut self, locator: &MediaLocator) -> Result<(), String> {
        execute_actions(&mut self.client, &translate_open(locator))
            .map_err(|error| error.to_string())
    }

    fn send(&mut self, command: BackendCommand) -> Result<(), String> {
        execute_actions(&mut self.client, &translate_command(&command))
            .map_err(|error| error.to_string())
    }

    fn drain_events(&mut self) -> Vec<BackendEvent> {
        self.pending_events.clear();
        for event in self.client.drain_typed_events() {
            match event {
                Ok(event) => {
                    if let Some(mapped) = map_event(event) {
                        self.pending_events.push(mapped);
                    }
                }
                Err(error) => self.pending_events.push(BackendEvent::Error(error.to_string())),
            }
        }
        std::mem::take(&mut self.pending_events)
    }
}

#[cfg(feature = "mpv-runtime")]
pub struct MpvClient {
    handle: *mut libmpv_sys::mpv_handle,
}

#[cfg(feature = "mpv-runtime")]
impl MpvClient {
    pub fn new() -> Result<Self, MpvError> {
        let handle = unsafe { libmpv_sys::mpv_create() };
        if handle.is_null() {
            return Err(MpvError::CreateHandle);
        }

        let init_result = unsafe { libmpv_sys::mpv_initialize(handle) };
        if init_result < 0 {
            unsafe { libmpv_sys::mpv_terminate_destroy(handle) };
            return Err(MpvError::Initialize(mpv_error_message(init_result)));
        }

        Ok(Self { handle })
    }

    pub fn observe_default_properties(&mut self) -> Result<(), MpvError> {
        self.observe_property(1, "pause", libmpv_sys::mpv_format_MPV_FORMAT_FLAG)?;
        self.observe_property(2, "time-pos", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE)?;
        self.observe_property(3, "duration", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE)?;
        self.observe_property(4, "speed", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE)?;
        self.observe_property(5, "volume", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE)?;
        self.observe_property(6, "video-rotate", libmpv_sys::mpv_format_MPV_FORMAT_INT64)?;
        Ok(())
    }

    fn observe_property(
        &mut self,
        reply_user_data: u64,
        name: &str,
        format: libmpv_sys::mpv_format,
    ) -> Result<(), MpvError> {
        let property_name = cstring(name)?;
        let result = unsafe {
            libmpv_sys::mpv_observe_property(
                self.handle,
                reply_user_data,
                property_name.as_ptr(),
                format,
            )
        };
        if result < 0 {
            return Err(MpvError::Property(format!(
                "observe {name}: {}",
                mpv_error_message(result)
            )));
        }
        Ok(())
    }

    pub fn drain_typed_events(&mut self) -> Vec<Result<MpvEvent, MpvError>> {
        let mut events = Vec::new();
        loop {
            let raw_event = unsafe { libmpv_sys::mpv_wait_event(self.handle, 0.0) };
            if raw_event.is_null() {
                break;
            }

            let raw_event = unsafe { &*raw_event };
            if raw_event.event_id == libmpv_sys::mpv_event_id_MPV_EVENT_NONE {
                break;
            }

            if raw_event.error < 0 {
                events.push(Err(MpvError::Api(mpv_error_message(raw_event.error))));
                continue;
            }

            match raw_event.event_id {
                libmpv_sys::mpv_event_id_MPV_EVENT_END_FILE => {
                    events.push(Ok(MpvEvent::EndFile));
                }
                libmpv_sys::mpv_event_id_MPV_EVENT_PROPERTY_CHANGE => {
                    if !raw_event.data.is_null() {
                        let property =
                            unsafe { &*(raw_event.data as *const libmpv_sys::mpv_event_property) };
                        if let Some(event) = decode_property_event(property) {
                            events.push(Ok(event));
                        }
                    }
                }
                libmpv_sys::mpv_event_id_MPV_EVENT_LOG_MESSAGE => {
                    if !raw_event.data.is_null() {
                        let message = unsafe {
                            &*(raw_event.data as *const libmpv_sys::mpv_event_log_message)
                        };
                        events.push(Ok(MpvEvent::Warning(log_message_text(message))));
                    }
                }
                other => {
                    events.push(Ok(MpvEvent::Warning(format!(
                        "ignored mpv event: {}",
                        event_name(other)
                    ))));
                }
            }
        }
        events
    }
}

#[cfg(feature = "mpv-runtime")]
impl MpvActionSink for MpvClient {
    fn command(&mut self, args: &[String]) -> Result<(), MpvError> {
        let cstrings: Result<Vec<_>, _> = args.iter().map(|arg| cstring(arg)).collect();
        let cstrings = cstrings?;
        let mut ptrs: Vec<*const std::os::raw::c_char> =
            cstrings.iter().map(|arg| arg.as_ptr()).collect();
        ptrs.push(std::ptr::null());

        let result = unsafe { libmpv_sys::mpv_command(self.handle, ptrs.as_mut_ptr()) };
        if result < 0 {
            return Err(MpvError::Command(format!(
                "{}: {}",
                args.join(" "),
                mpv_error_message(result)
            )));
        }
        Ok(())
    }

    fn set_flag(&mut self, name: &str, value: bool) -> Result<(), MpvError> {
        let value = if value { 1 } else { 0 };
        self.set_property_i32(name, value)
    }

    fn set_string(&mut self, name: &str, value: &str) -> Result<(), MpvError> {
        let name = cstring(name)?;
        let value = cstring(value)?;
        let result = unsafe {
            libmpv_sys::mpv_set_property_string(self.handle, name.as_ptr(), value.as_ptr())
        };
        if result < 0 {
            return Err(MpvError::Property(format!(
                "set string {}: {}",
                name.to_string_lossy(),
                mpv_error_message(result)
            )));
        }
        Ok(())
    }

    fn set_i64(&mut self, name: &str, value: i64) -> Result<(), MpvError> {
        self.set_property_i64(name, value)
    }

    fn set_f64(&mut self, name: &str, value: f64) -> Result<(), MpvError> {
        let property_name = cstring(name)?;
        let mut value = value;
        let result = unsafe {
            libmpv_sys::mpv_set_property(
                self.handle,
                property_name.as_ptr(),
                libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE,
                (&mut value as *mut f64).cast(),
            )
        };
        if result < 0 {
            return Err(MpvError::Property(format!(
                "set double {name}: {}",
                mpv_error_message(result)
            )));
        }
        Ok(())
    }
}

#[cfg(feature = "mpv-runtime")]
impl MpvClient {
    fn set_property_i32(&mut self, name: &str, value: i32) -> Result<(), MpvError> {
        let property_name = cstring(name)?;
        let mut value = value;
        let result = unsafe {
            libmpv_sys::mpv_set_property(
                self.handle,
                property_name.as_ptr(),
                libmpv_sys::mpv_format_MPV_FORMAT_FLAG,
                (&mut value as *mut i32).cast(),
            )
        };
        if result < 0 {
            return Err(MpvError::Property(format!(
                "set flag {name}: {}",
                mpv_error_message(result)
            )));
        }
        Ok(())
    }

    fn set_property_i64(&mut self, name: &str, value: i64) -> Result<(), MpvError> {
        let property_name = cstring(name)?;
        let mut value = value;
        let result = unsafe {
            libmpv_sys::mpv_set_property(
                self.handle,
                property_name.as_ptr(),
                libmpv_sys::mpv_format_MPV_FORMAT_INT64,
                (&mut value as *mut i64).cast(),
            )
        };
        if result < 0 {
            return Err(MpvError::Property(format!(
                "set int64 {name}: {}",
                mpv_error_message(result)
            )));
        }
        Ok(())
    }
}

#[cfg(feature = "mpv-runtime")]
impl Drop for MpvClient {
    fn drop(&mut self) {
        unsafe { libmpv_sys::mpv_terminate_destroy(self.handle) };
    }
}

#[cfg(not(feature = "mpv-runtime"))]
pub struct MpvClient;

#[cfg(not(feature = "mpv-runtime"))]
impl MpvClient {
    pub fn new() -> Result<Self, MpvError> {
        Err(MpvError::RuntimeDisabled)
    }

    pub fn observe_default_properties(&mut self) -> Result<(), MpvError> {
        Err(MpvError::RuntimeDisabled)
    }

    pub fn drain_typed_events(&mut self) -> Vec<Result<MpvEvent, MpvError>> {
        Vec::new()
    }
}

#[cfg(not(feature = "mpv-runtime"))]
impl MpvActionSink for MpvClient {
    fn command(&mut self, _args: &[String]) -> Result<(), MpvError> {
        Err(MpvError::RuntimeDisabled)
    }

    fn set_flag(&mut self, _name: &str, _value: bool) -> Result<(), MpvError> {
        Err(MpvError::RuntimeDisabled)
    }

    fn set_string(&mut self, _name: &str, _value: &str) -> Result<(), MpvError> {
        Err(MpvError::RuntimeDisabled)
    }

    fn set_i64(&mut self, _name: &str, _value: i64) -> Result<(), MpvError> {
        Err(MpvError::RuntimeDisabled)
    }

    fn set_f64(&mut self, _name: &str, _value: f64) -> Result<(), MpvError> {
        Err(MpvError::RuntimeDisabled)
    }
}

#[cfg(feature = "mpv-runtime")]
fn decode_property_event(property: &libmpv_sys::mpv_event_property) -> Option<MpvEvent> {
    let name = cstr_to_string(property.name)?;
    if property.data.is_null() {
        return None;
    }

    match (name.as_str(), property.format) {
        ("pause", libmpv_sys::mpv_format_MPV_FORMAT_FLAG) => {
            let value = unsafe { *(property.data as *const std::os::raw::c_int) };
            Some(MpvEvent::Pause(value != 0))
        }
        ("time-pos", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE) => {
            let value = unsafe { *(property.data as *const f64) };
            Some(MpvEvent::Position(value))
        }
        ("duration", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE) => {
            let value = unsafe { *(property.data as *const f64) };
            Some(MpvEvent::Duration(Some(value)))
        }
        ("speed", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE) => {
            let value = unsafe { *(property.data as *const f64) };
            Some(MpvEvent::Speed(value as f32))
        }
        ("volume", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE) => {
            let value = unsafe { *(property.data as *const f64) };
            Some(MpvEvent::Volume(value.round().clamp(0.0, 100.0) as u8))
        }
        ("video-rotate", libmpv_sys::mpv_format_MPV_FORMAT_INT64) => {
            let value = unsafe { *(property.data as *const i64) };
            Some(MpvEvent::Rotation(value))
        }
        _ => None,
    }
}

#[cfg(feature = "mpv-runtime")]
fn cstring(value: &str) -> Result<std::ffi::CString, MpvError> {
    std::ffi::CString::new(value).map_err(|_| MpvError::InvalidString(value.into()))
}

#[cfg(feature = "mpv-runtime")]
fn cstr_to_string(value: *const std::os::raw::c_char) -> Option<String> {
    if value.is_null() {
        return None;
    }
    Some(unsafe { std::ffi::CStr::from_ptr(value) }.to_string_lossy().into_owned())
}

#[cfg(feature = "mpv-runtime")]
fn log_message_text(message: &libmpv_sys::mpv_event_log_message) -> String {
    cstr_to_string(message.text).unwrap_or_else(|| "mpv log message".into())
}

#[cfg(feature = "mpv-runtime")]
fn event_name(event_id: libmpv_sys::mpv_event_id) -> String {
    let name = unsafe { libmpv_sys::mpv_event_name(event_id) };
    cstr_to_string(name).unwrap_or_else(|| format!("unknown({event_id})"))
}

#[cfg(feature = "mpv-runtime")]
fn mpv_error_message(error: std::os::raw::c_int) -> String {
    let message = unsafe { libmpv_sys::mpv_error_string(error) };
    cstr_to_string(message).unwrap_or_else(|| format!("error code {error}"))
}
