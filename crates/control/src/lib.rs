mod client;
mod paths;
mod protocol;

pub const PROTOCOL_VERSION: u64 = 1;

pub use client::{
    emit_error, emit_job_event, ensure_daemon, run_daemon_foreground, send_request, wait_for_job,
    DaemonClient,
};
pub use paths::{daemon_log_path, now_ms, socket_path, wrec_home};
pub use protocol::{
    generic_daemon_error, response_error, AgentError, AgentWarning, EventLevel, IpcRequest,
    IpcResponse, JobEvent, JobSnapshot, JobStatus, RecordingOptions, StartRecordingParams,
    TargetSelector,
};
