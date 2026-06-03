use crate::{
    paths::{daemon_log_path, now_ms, socket_path, wrec_home},
    protocol::{
        generic_daemon_error, AgentError, IpcRequest, IpcResponse, JobEvent, JobSnapshot,
        JobStatus, StartRecordingParams,
    },
};
use serde_json::{json, Value};
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    os::unix::{net::UnixStream, process::CommandExt},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use wrec_core::CaptureTarget;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(3);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const WAIT_POLL_INTERVAL: Duration = Duration::from_millis(500);
const IPC_READ_TIMEOUT: Duration = Duration::from_secs(10);
const IPC_WRITE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, Default)]
pub struct DaemonClient;

impl DaemonClient {
    pub fn new() -> Self {
        Self
    }

    pub fn ensure(&self) -> Result<(), AgentError> {
        ensure_daemon()
    }

    pub fn send_request(&self, method: &str, params: Value) -> Result<IpcResponse, AgentError> {
        let mut stream = UnixStream::connect(socket_path()).map_err(|err| AgentError {
            code: "daemon_unreachable".into(),
            message: format!("Could not connect to {}: {err}", socket_path().display()),
            recoverable: true,
            next: "Run `wrec daemon start` or retry a command that auto-starts the daemon.".into(),
        })?;
        stream
            .set_write_timeout(Some(IPC_WRITE_TIMEOUT))
            .map_err(|err| AgentError {
                code: "request_timeout_config_failed".into(),
                message: format!("Could not configure IPC write timeout: {err}"),
                recoverable: true,
                next: "Retry the command; if it repeats, restart the daemon.".into(),
            })?;
        stream
            .set_read_timeout(Some(IPC_READ_TIMEOUT))
            .map_err(|err| AgentError {
                code: "response_timeout_config_failed".into(),
                message: format!("Could not configure IPC read timeout: {err}"),
                recoverable: true,
                next: "Retry the command; if it repeats, restart the daemon.".into(),
            })?;

        let request = IpcRequest {
            id: now_ms(),
            method: method.to_string(),
            params,
        };
        let line = serde_json::to_string(&request).map_err(|err| AgentError {
            code: "request_encode_failed".into(),
            message: err.to_string(),
            recoverable: false,
            next: "Report this as a wrec IPC serialization bug.".into(),
        })?;
        stream
            .write_all(line.as_bytes())
            .map_err(|err| AgentError {
                code: "request_write_failed".into(),
                message: format!("Could not write IPC request: {err}"),
                recoverable: true,
                next: "Retry the command; if it repeats, run `wrec daemon status`.".into(),
            })?;
        stream.write_all(b"\n").map_err(|err| AgentError {
            code: "request_write_failed".into(),
            message: format!("Could not finish IPC request: {err}"),
            recoverable: true,
            next: "Retry the command; if it repeats, run `wrec daemon status`.".into(),
        })?;
        stream.flush().map_err(|err| AgentError {
            code: "request_write_failed".into(),
            message: format!("Could not flush IPC request: {err}"),
            recoverable: true,
            next: "Retry the command; if it repeats, run `wrec daemon status`.".into(),
        })?;

        let mut response = String::new();
        BufReader::new(stream)
            .read_line(&mut response)
            .map_err(|err| AgentError {
                code: "response_read_failed".into(),
                message: format!("Could not read IPC response: {err}"),
                recoverable: true,
                next: "Retry the command; if it repeats, restart the daemon.".into(),
            })?;
        if response.is_empty() {
            return Err(AgentError {
                code: "response_read_failed".into(),
                message: "Daemon closed the IPC stream without a response.".into(),
                recoverable: true,
                next: "Retry the command; if it repeats, restart the daemon.".into(),
            });
        }
        serde_json::from_str(&response).map_err(|err| AgentError {
            code: "response_decode_failed".into(),
            message: format!("Could not decode IPC response: {err}"),
            recoverable: false,
            next: "Inspect ~/.wrec/daemon.log and report this as a wrec IPC protocol bug.".into(),
        })
    }

    pub fn status(&self) -> Result<Value, AgentError> {
        self.request_result("daemon.status", json!({}))
    }

    pub fn stop_daemon(&self) -> Result<Value, AgentError> {
        self.request_result("daemon.stop", json!({}))
    }

    pub fn list_targets(&self) -> Result<Vec<CaptureTarget>, AgentError> {
        let result = self.request_result("targets.list", json!({}))?;
        serde_json::from_value(result.get("targets").cloned().unwrap_or_else(|| json!([])))
            .map_err(|err| protocol_mismatch("targets_decode_failed", err))
    }

    pub fn start_recording(&self, params: StartRecordingParams) -> Result<JobSnapshot, AgentError> {
        let value = serde_json::to_value(params).map_err(|err| AgentError {
            code: "record_request_encode_failed".into(),
            message: format!("Could not encode record.start request: {err}"),
            recoverable: false,
            next: "Report this as a wrec IPC serialization bug.".into(),
        })?;
        self.request_job("record.start", value)
    }

    pub fn list_jobs(&self) -> Result<Vec<JobSnapshot>, AgentError> {
        let result = self.request_result("jobs.list", json!({}))?;
        serde_json::from_value(result.get("jobs").cloned().unwrap_or_else(|| json!([])))
            .map_err(|err| protocol_mismatch("jobs_decode_failed", err))
    }

    pub fn show_job(&self, job_id: u64) -> Result<JobSnapshot, AgentError> {
        self.request_job("job.show", json!({ "job_id": job_id }))
    }

    pub fn job_logs(&self, job_id: u64) -> Result<Vec<JobEvent>, AgentError> {
        let result = self.request_result("job.logs", json!({ "job_id": job_id }))?;
        serde_json::from_value(result.get("events").cloned().unwrap_or_else(|| json!([])))
            .map_err(|err| protocol_mismatch("job_logs_decode_failed", err))
    }

    pub fn pause_job(&self, job_id: u64) -> Result<JobSnapshot, AgentError> {
        self.request_job("job.pause", json!({ "job_id": job_id }))
    }

    pub fn resume_job(&self, job_id: u64) -> Result<JobSnapshot, AgentError> {
        self.request_job("job.resume", json!({ "job_id": job_id }))
    }

    pub fn stop_job(&self, job_id: u64) -> Result<JobSnapshot, AgentError> {
        self.request_job("job.stop", json!({ "job_id": job_id }))
    }

    pub fn cancel_job(&self, job_id: u64) -> Result<JobSnapshot, AgentError> {
        self.request_job("job.cancel", json!({ "job_id": job_id }))
    }

    fn request_job(&self, method: &str, params: Value) -> Result<JobSnapshot, AgentError> {
        let result = self.request_result(method, params)?;
        serde_json::from_value(result.get("job").cloned().unwrap_or(Value::Null))
            .map_err(|err| protocol_mismatch("job_decode_failed", err))
    }

    fn request_result(&self, method: &str, params: Value) -> Result<Value, AgentError> {
        let response = self.send_request(method, params)?;
        if response.ok {
            Ok(response.result.unwrap_or_else(|| json!({})))
        } else {
            Err(response.error.unwrap_or_else(generic_daemon_error))
        }
    }
}

pub fn ensure_daemon() -> Result<(), AgentError> {
    let client = DaemonClient::new();
    if client.status().is_ok() {
        return Ok(());
    }

    std::fs::create_dir_all(wrec_home()).map_err(|err| AgentError {
        code: "daemon_home_unavailable".into(),
        message: format!("Could not create {}: {err}", wrec_home().display()),
        recoverable: true,
        next: "Create the directory manually or set WREC_HOME to a writable path.".into(),
    })?;

    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(daemon_log_path())
        .map_err(|err| AgentError {
            code: "daemon_log_unavailable".into(),
            message: format!("Could not open {}: {err}", daemon_log_path().display()),
            recoverable: true,
            next: "Check permissions for ~/.wrec or set WREC_HOME to a writable path.".into(),
        })?;
    let stderr = log.try_clone().map_err(|err| AgentError {
        code: "daemon_log_unavailable".into(),
        message: format!("Could not duplicate daemon log handle: {err}"),
        recoverable: true,
        next: "Check permissions for ~/.wrec and try again.".into(),
    })?;
    let exe = daemon_executable()?;

    Command::new(exe)
        .arg("daemon")
        .arg("serve")
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|err| AgentError {
            code: "daemon_start_failed".into(),
            message: format!("Could not start wrec daemon: {err}"),
            recoverable: true,
            next: "Run `wrec daemon serve` manually and inspect ~/.wrec/daemon.log.".into(),
        })?;

    let started = Instant::now();
    while started.elapsed() < STARTUP_TIMEOUT {
        if client.status().is_ok() {
            return Ok(());
        }
        thread::sleep(POLL_INTERVAL);
    }

    Err(AgentError {
        code: "daemon_unreachable".into(),
        message: format!(
            "wrec daemon did not become reachable at {} within {}s",
            socket_path().display(),
            STARTUP_TIMEOUT.as_secs()
        ),
        recoverable: true,
        next: "Inspect ~/.wrec/daemon.log, then run `wrec daemon serve` manually if needed.".into(),
    })
}

pub fn send_request(method: &str, params: Value) -> Result<IpcResponse, AgentError> {
    DaemonClient::new().send_request(method, params)
}

pub fn wait_for_job(job_id: u64, json_output: bool) -> Result<JobSnapshot, AgentError> {
    let client = DaemonClient::new();
    let mut seen_events = 0;
    loop {
        let job = client.show_job(job_id)?;
        for event in job.events.iter().skip(seen_events) {
            emit_job_event(json_output, job.id, event);
        }
        seen_events = job.events.len();

        if matches!(
            job.status,
            JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
        ) {
            return Ok(job);
        }
        thread::sleep(WAIT_POLL_INTERVAL);
    }
}

pub fn emit_error(error: &AgentError, json_output: bool) {
    if json_output {
        println!(
            "{}",
            json!({
                "event": "error",
                "code": error.code,
                "message": error.message,
                "recoverable": error.recoverable,
                "next": error.next,
            })
        );
    } else {
        eprintln!("error: {}", error.message);
        eprintln!("next: {}", error.next);
    }
}

pub fn emit_job_event(json_output: bool, job_id: u64, event: &JobEvent) {
    if json_output {
        println!(
            "{}",
            json!({
                "event": "job_event",
                "job_id": job_id,
                "level": event.level,
                "message": event.message,
                "metrics": event.metrics,
                "timestamp_ms": event.timestamp_ms,
            })
        );
    } else {
        println!("{}", event.message);
    }
}

fn protocol_mismatch(code: &str, err: serde_json::Error) -> AgentError {
    AgentError {
        code: code.into(),
        message: format!("Could not decode daemon response: {err}"),
        recoverable: false,
        next: "Inspect ~/.wrec/daemon.log and report this as a wrec IPC protocol bug.".into(),
    }
}

fn daemon_executable() -> Result<PathBuf, AgentError> {
    if let Some(path) = std::env::var_os("WREC_DAEMON_BIN").map(PathBuf::from) {
        return Ok(path);
    }

    let current = std::env::current_exe().map_err(|err| AgentError {
        code: "daemon_start_failed".into(),
        message: format!("Could not locate current wrec executable: {err}"),
        recoverable: false,
        next: "Run `wrec daemon serve` manually from a known executable.".into(),
    })?;

    if current.file_name().and_then(|name| name.to_str()) == Some("wrec") {
        return Ok(current);
    }

    let sibling_cli = current
        .parent()
        .map(|dir| dir.join("wrec"))
        .filter(|path| path.is_file());

    Ok(sibling_cli.unwrap_or(current))
}
