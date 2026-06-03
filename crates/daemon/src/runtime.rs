use crate::protocol::AgentError;
use std::sync::mpsc;
use wrec_core::{CaptureTarget, RecorderEngine, RecorderEvent};
use wrec_macos::MacosRecorder;

pub(crate) trait RecordingRuntime: Clone + Send + Sync + 'static {
    type Engine: RecorderEngine + Send + 'static;

    fn list_targets(&self) -> Result<Vec<CaptureTarget>, AgentError>;
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

    fn new_engine(&self, events: mpsc::Sender<RecorderEvent>) -> Self::Engine {
        MacosRecorder::new(events)
    }
}
