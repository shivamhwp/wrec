use control::AgentError;
use domain::{
    CaptureTarget, RecorderEngine, RecorderError, RecorderEvent, ScreenRecordingPermissionStatus,
};
use macos::MacosRecorder;
use std::sync::mpsc;

pub(crate) trait RecordingRuntime: Clone + Send + Sync + 'static {
    type Engine: RecorderEngine + Send + 'static;

    fn list_targets(&self) -> Result<Vec<CaptureTarget>, AgentError>;
    fn screen_recording_permission_status(
        &self,
    ) -> Result<ScreenRecordingPermissionStatus, AgentError>;
    fn request_screen_recording_permission(
        &self,
    ) -> Result<ScreenRecordingPermissionStatus, AgentError>;
    fn new_engine(&self, events: mpsc::Sender<RecorderEvent>) -> Self::Engine;
}

#[derive(Clone, Default)]
pub(crate) struct MacosRuntime;

impl RecordingRuntime for MacosRuntime {
    type Engine = MacosRecorder;

    fn list_targets(&self) -> Result<Vec<CaptureTarget>, AgentError> {
        let (tx, _rx) = mpsc::channel();
        MacosRecorder::new(tx).list_targets().map_err(|err| AgentError {
            code: "target_listing_failed".into(),
            message: err.to_string(),
            recoverable: true,
            next: "Run `wrec targets --json` again; if this repeats, check Screen Recording permission and ~/.wrec/daemon.log.".into(),
        })
    }

    fn screen_recording_permission_status(
        &self,
    ) -> Result<ScreenRecordingPermissionStatus, AgentError> {
        let (tx, _rx) = mpsc::channel();
        MacosRecorder::new(tx)
            .screen_recording_permission_status()
            .map_err(permission_error)
    }

    fn request_screen_recording_permission(
        &self,
    ) -> Result<ScreenRecordingPermissionStatus, AgentError> {
        let (tx, _rx) = mpsc::channel();
        MacosRecorder::new(tx)
            .request_screen_recording_permission()
            .map_err(permission_error)
    }

    fn new_engine(&self, events: mpsc::Sender<RecorderEvent>) -> Self::Engine {
        MacosRecorder::new(events)
    }
}

fn permission_error(error: RecorderError) -> AgentError {
    match error {
        RecorderError::MissingScreenRecordingPermission => AgentError {
            code: "screen_recording_permission_missing".into(),
            message: "screen recording permission is not granted".into(),
            recoverable: true,
            next: "Grant Screen Recording permission, then retry.".into(),
        },
        RecorderError::Backend(message) if message.contains("capture-engine") => AgentError {
            code: "capture_engine_missing".into(),
            message: format!("backend error: {message}"),
            recoverable: true,
            next: "Build the daemon through Cargo or install the full wrec runtime so daemon and capture-engine are present together.".into(),
        },
        error => AgentError {
            code: "screen_recording_permission_failed".into(),
            message: error.to_string(),
            recoverable: true,
            next: "Fix the backend error above, then retry the permission check.".into(),
        },
    }
}
