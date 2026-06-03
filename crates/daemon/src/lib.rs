mod client;
mod coordinator;
mod jobs;
mod paths;
mod protocol;
mod runtime;
mod server;
mod target_resolution;

pub use client::{
    emit_error, emit_job_event, ensure_daemon, send_request, wait_for_job, DaemonClient,
};
pub use paths::{daemon_log_path, job_events_path, socket_path, wrec_home};
pub use protocol::{
    AgentError, AgentWarning, EventLevel, IpcRequest, IpcResponse, JobEvent, JobSnapshot,
    JobStatus, RecordingOptions, StartRecordingParams, TargetSelector,
};
pub use server::serve_forever;
