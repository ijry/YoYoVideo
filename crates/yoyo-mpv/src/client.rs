use yoyo_core::{BackendCommand, BackendEvent, MediaLocator, PlayerBackend};

use crate::{
    MpvAction, MpvClientOptions, MpvError, MpvEvent, MpvRenderBridge, map_event, translate_command,
    translate_open,
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
    #[cfg(feature = "native-qa")]
    pub fn qa_output_flags(&self) -> Result<(bool, bool, bool), MpvError> {
        let flag = |name: &str| -> Result<bool, MpvError> {
            let name = cstring(name)?;
            let mut value = 0_i32;
            // QA-only, fixed flag properties. Never inspect locators or arbitrary strings.
            let result = unsafe {
                libmpv_sys::mpv_get_property(
                    self.client.handle,
                    name.as_ptr(),
                    libmpv_sys::mpv_format_MPV_FORMAT_FLAG,
                    (&mut value as *mut i32).cast(),
                )
            };
            if result < 0 {
                return Err(MpvError::Property("QA output flag unavailable".into()));
            }
            Ok(value != 0)
        };
        Ok((flag("pause")?, flag("mute")?, flag("idle-active")?))
    }

    pub fn new_runtime() -> Result<Self, MpvError> {
        Self::new_runtime_with_options(MpvClientOptions::default())
    }

    pub fn new_runtime_with_options(options: MpvClientOptions) -> Result<Self, MpvError> {
        let mut client = MpvClient::new_with_options(options)?;
        client.observe_default_properties()?;
        Ok(Self { client, pending_events: Vec::new(), render_bridge: MpvRenderBridge::default() })
    }

    pub fn render_bridge(&mut self) -> &mut MpvRenderBridge {
        &mut self.render_bridge
    }

    /// Creates an OpenGL render context bound to this backend's mpv handle.
    ///
    /// The handle stays private: callers get a render context, not the ability to
    /// reach into mpv. Needed on macOS, where mpv has no `--wid` embedding and the
    /// render API is the only way to put video on screen.
    ///
    /// # Safety
    ///
    /// The GL context behind `get_proc` must be current on the thread that later
    /// calls into the returned context, and this backend must outlive it.
    #[cfg(feature = "mpv-runtime")]
    pub unsafe fn create_gl_render_context(
        &self,
        get_proc: crate::GetProcAddress,
    ) -> Result<crate::MpvGlRenderContext, MpvError> {
        // SAFETY: the caller upholds the contract above; `handle` is an initialised
        // mpv handle owned by `self.client`.
        unsafe { crate::MpvGlRenderContext::new(self.client.handle, get_proc) }
    }

    /// Creates a render context using a GL loader borrowed during initialization.
    ///
    /// # Safety
    /// The same GL context must be current for creation, rendering and teardown;
    /// this backend must outlive the returned render context.
    #[cfg(feature = "mpv-runtime")]
    pub unsafe fn create_gl_render_context_with_loader(
        &self,
        get_proc: &dyn Fn(&std::ffi::CStr) -> *const std::ffi::c_void,
    ) -> Result<crate::MpvGlRenderContext, MpvError> {
        // SAFETY: the caller upholds the context and backend lifetime requirements.
        unsafe { crate::MpvGlRenderContext::new_with_loader(self.client.handle, get_proc) }
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
        Self::new_with_options(MpvClientOptions::default())
    }

    pub fn new_with_options(options: MpvClientOptions) -> Result<Self, MpvError> {
        let handle = unsafe { libmpv_sys::mpv_create() };
        if handle.is_null() {
            return Err(MpvError::CreateHandle);
        }

        if let Err(error) = apply_client_options(handle, &options) {
            unsafe { libmpv_sys::mpv_terminate_destroy(handle) };
            return Err(error);
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
        self.observe_property(7, "track-list", libmpv_sys::mpv_format_MPV_FORMAT_NODE)?;
        self.observe_property(8, "sub-visibility", libmpv_sys::mpv_format_MPV_FORMAT_FLAG)?;
        self.observe_property(9, "sub-delay", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE)?;
        self.observe_property(10, "sub-scale", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE)?;
        self.observe_property(11, "sub-pos", libmpv_sys::mpv_format_MPV_FORMAT_INT64)?;
        self.observe_property(12, "mute", libmpv_sys::mpv_format_MPV_FORMAT_FLAG)?;
        self.observe_property(13, "chapter-list", libmpv_sys::mpv_format_MPV_FORMAT_NODE)?;
        // Display size, i.e. after aspect/rotation correction. Grid mode lays tiles out
        // by aspect ratio, so it needs the size the picture is actually presented at.
        self.observe_property(14, "dwidth", libmpv_sys::mpv_format_MPV_FORMAT_INT64)?;
        self.observe_property(15, "dheight", libmpv_sys::mpv_format_MPV_FORMAT_INT64)?;
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

            // The event payload is owned by mpv until the next wait_event call.
            if let Some(event) = unsafe { decode_runtime_event(raw_event) } {
                events.push(event);
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

    pub fn new_with_options(_options: MpvClientOptions) -> Result<Self, MpvError> {
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
fn apply_client_options(
    handle: *mut libmpv_sys::mpv_handle,
    options: &MpvClientOptions,
) -> Result<(), MpvError> {
    for (name, value) in options.mpv_option_pairs() {
        let name = cstring(name)?;
        let value = cstring(&value)?;
        let result =
            unsafe { libmpv_sys::mpv_set_option_string(handle, name.as_ptr(), value.as_ptr()) };
        if result < 0 {
            return Err(MpvError::VideoOutput(format!(
                "set option {}: {}",
                name.to_string_lossy(),
                mpv_error_message(result)
            )));
        }
    }
    Ok(())
}

#[cfg(feature = "mpv-runtime")]
/// Decode one live mpv event. Any payload pointers must remain valid for its event kind.
unsafe fn decode_runtime_event(
    raw_event: &libmpv_sys::mpv_event,
) -> Option<Result<MpvEvent, MpvError>> {
    if raw_event.error < 0 {
        return Some(Err(MpvError::Api(mpv_error_message(raw_event.error))));
    }
    match raw_event.event_id {
        libmpv_sys::mpv_event_id_MPV_EVENT_END_FILE => {
            if raw_event.data.is_null() {
                return None;
            }
            let end = unsafe { &*(raw_event.data as *const libmpv_sys::mpv_event_end_file) };
            match end.reason as u32 {
                libmpv_sys::mpv_end_file_reason_MPV_END_FILE_REASON_EOF => {
                    Some(Ok(MpvEvent::EndFile))
                }
                libmpv_sys::mpv_end_file_reason_MPV_END_FILE_REASON_ERROR => {
                    Some(Err(MpvError::Api(mpv_error_message(end.error))))
                }
                _ => None, // Stop/replace/quit/redirect are not automatic playlist navigation.
            }
        }
        libmpv_sys::mpv_event_id_MPV_EVENT_PROPERTY_CHANGE => {
            if raw_event.data.is_null() {
                return None;
            }
            let property = unsafe { &*(raw_event.data as *const libmpv_sys::mpv_event_property) };
            decode_property_event(property).map(Ok)
        }
        libmpv_sys::mpv_event_id_MPV_EVENT_LOG_MESSAGE => {
            if raw_event.data.is_null() {
                return None;
            }
            let message = unsafe { &*(raw_event.data as *const libmpv_sys::mpv_event_log_message) };
            Some(Ok(MpvEvent::Warning(log_message_text(message))))
        }
        other => {
            // Notifications such as start-file and playback-restart are not playback warnings.
            tracing::debug!(event_id = other, event = %event_name(other), "ignored mpv event");
            None
        }
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
        ("mute", libmpv_sys::mpv_format_MPV_FORMAT_FLAG) => {
            let value = unsafe { *(property.data as *const std::os::raw::c_int) };
            Some(MpvEvent::Muted(value != 0))
        }
        ("video-rotate", libmpv_sys::mpv_format_MPV_FORMAT_INT64) => {
            let value = unsafe { *(property.data as *const i64) };
            Some(MpvEvent::Rotation(value))
        }
        ("dwidth", libmpv_sys::mpv_format_MPV_FORMAT_INT64) => {
            let value = unsafe { *(property.data as *const i64) };
            Some(MpvEvent::VideoWidth(value))
        }
        ("dheight", libmpv_sys::mpv_format_MPV_FORMAT_INT64) => {
            let value = unsafe { *(property.data as *const i64) };
            Some(MpvEvent::VideoHeight(value))
        }
        ("track-list", libmpv_sys::mpv_format_MPV_FORMAT_NODE) => {
            crate::track_list::decode_track_list_property(property)
        }
        ("chapter-list", libmpv_sys::mpv_format_MPV_FORMAT_NODE) => {
            crate::chapter_list::decode_chapter_list_property(property)
        }
        ("sub-visibility", libmpv_sys::mpv_format_MPV_FORMAT_FLAG) => {
            let value = unsafe { *(property.data as *const std::os::raw::c_int) };
            Some(MpvEvent::SubtitleVisible(value != 0))
        }
        ("sub-delay", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE) => {
            let value = unsafe { *(property.data as *const f64) };
            Some(MpvEvent::SubtitleDelay(value))
        }
        ("sub-scale", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE) => {
            let value = unsafe { *(property.data as *const f64) };
            Some(MpvEvent::SubtitleScale(value as f32))
        }
        ("sub-pos", libmpv_sys::mpv_format_MPV_FORMAT_INT64) => {
            let value = unsafe { *(property.data as *const i64) };
            Some(MpvEvent::SubtitlePosition(value.clamp(0, 100) as u8))
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

#[cfg(all(test, feature = "mpv-runtime"))]
mod runtime_event_tests {
    use super::*;

    fn event(id: libmpv_sys::mpv_event_id) -> libmpv_sys::mpv_event {
        libmpv_sys::mpv_event {
            event_id: id,
            error: 0,
            reply_userdata: 0,
            data: std::ptr::null_mut(),
        }
    }

    #[test]
    fn routine_mpv_notifications_do_not_become_user_warnings() {
        for id in [
            libmpv_sys::mpv_event_id_MPV_EVENT_START_FILE,
            libmpv_sys::mpv_event_id_MPV_EVENT_FILE_LOADED,
            libmpv_sys::mpv_event_id_MPV_EVENT_SEEK,
            libmpv_sys::mpv_event_id_MPV_EVENT_PLAYBACK_RESTART,
            libmpv_sys::mpv_event_id_MPV_EVENT_VIDEO_RECONFIG,
            libmpv_sys::mpv_event_id_MPV_EVENT_AUDIO_RECONFIG,
        ] {
            let decoded = unsafe { decode_runtime_event(&event(id)) };
            assert!(decoded.is_none(), "Routine event {id} became a user warning: {decoded:?}");
        }
    }

    #[test]
    fn unknown_event_errors_are_still_reported() {
        let mut raw = event(65535);
        raw.error = -4;
        assert!(matches!(unsafe { decode_runtime_event(&raw) }, Some(Err(MpvError::Api(_)))));
    }

    #[test]
    fn real_mpv_log_warnings_are_preserved() {
        let text = std::ffi::CString::new("audio output unavailable").unwrap();
        let mut message: libmpv_sys::mpv_event_log_message = unsafe { std::mem::zeroed() };
        message.text = text.as_ptr();
        let mut raw = event(libmpv_sys::mpv_event_id_MPV_EVENT_LOG_MESSAGE);
        raw.data = (&mut message as *mut libmpv_sys::mpv_event_log_message).cast();
        match unsafe { decode_runtime_event(&raw) } {
            Some(Ok(MpvEvent::Warning(message))) => assert_eq!(message, "audio output unavailable"),
            other => panic!("Lost a real mpv warning: {other:?}"),
        }
    }

    #[test]
    fn end_of_file_is_preserved() {
        let mut payload: libmpv_sys::mpv_event_end_file = unsafe { std::mem::zeroed() };
        payload.reason = libmpv_sys::mpv_end_file_reason_MPV_END_FILE_REASON_EOF as i32;
        let mut raw = event(libmpv_sys::mpv_event_id_MPV_EVENT_END_FILE);
        raw.data = (&mut payload as *mut libmpv_sys::mpv_event_end_file).cast();
        assert!(matches!(unsafe { decode_runtime_event(&raw) }, Some(Ok(MpvEvent::EndFile))));
    }

    #[test]
    fn replacing_or_stopping_a_file_is_not_automatic_eof_navigation() {
        for reason in [
            libmpv_sys::mpv_end_file_reason_MPV_END_FILE_REASON_STOP,
            libmpv_sys::mpv_end_file_reason_MPV_END_FILE_REASON_QUIT,
            libmpv_sys::mpv_end_file_reason_MPV_END_FILE_REASON_REDIRECT,
        ] {
            let mut payload: libmpv_sys::mpv_event_end_file = unsafe { std::mem::zeroed() };
            payload.reason = reason as i32;
            let mut raw = event(libmpv_sys::mpv_event_id_MPV_EVENT_END_FILE);
            raw.data = (&mut payload as *mut libmpv_sys::mpv_event_end_file).cast();
            assert!(unsafe { decode_runtime_event(&raw) }.is_none());
        }
        assert!(
            unsafe { decode_runtime_event(&event(libmpv_sys::mpv_event_id_MPV_EVENT_END_FILE)) }
                .is_none()
        );
    }

    #[test]
    fn failed_media_reports_an_error_instead_of_advancing_the_playlist() {
        let mut payload: libmpv_sys::mpv_event_end_file = unsafe { std::mem::zeroed() };
        payload.reason = libmpv_sys::mpv_end_file_reason_MPV_END_FILE_REASON_ERROR as i32;
        payload.error = -13;
        let mut raw = event(libmpv_sys::mpv_event_id_MPV_EVENT_END_FILE);
        raw.data = (&mut payload as *mut libmpv_sys::mpv_event_end_file).cast();
        assert!(matches!(unsafe { decode_runtime_event(&raw) }, Some(Err(MpvError::Api(_)))));
    }

    #[test]
    fn observed_pause_properties_are_preserved() {
        let name = std::ffi::CString::new("pause").unwrap();
        let mut paused: std::os::raw::c_int = 1;
        let mut property = libmpv_sys::mpv_event_property {
            name: name.as_ptr(),
            format: libmpv_sys::mpv_format_MPV_FORMAT_FLAG,
            data: (&mut paused as *mut std::os::raw::c_int).cast(),
        };
        let mut raw = event(libmpv_sys::mpv_event_id_MPV_EVENT_PROPERTY_CHANGE);
        raw.data = (&mut property as *mut libmpv_sys::mpv_event_property).cast();
        assert!(matches!(unsafe { decode_runtime_event(&raw) }, Some(Ok(MpvEvent::Pause(true)))));
    }
}

#[cfg(all(test, feature = "native-qa"))]
mod privacy_native_probe_tests {
    use super::*;
    #[test]
    fn qa_probe_reads_actual_backend_pause_and_mute_not_player_view_state() {
        let mut backend = MpvBackend::new_runtime_with_options(MpvClientOptions {
            audio_output: Some("null".into()),
            ..Default::default()
        })
        .unwrap();
        backend.send(BackendCommand::SetPaused(true)).unwrap();
        backend.send(BackendCommand::SetMuted(true)).unwrap();
        let (paused, muted, _) = backend.qa_output_flags().unwrap();
        assert!(paused);
        assert!(muted);
        backend.send(BackendCommand::SetMuted(false)).unwrap();
        assert!(!backend.qa_output_flags().unwrap().1);
    }
}
