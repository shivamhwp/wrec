use crate::protocol::JobEvent;
use serde_json::json;
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const SOCKET_NAME: &str = "wrec.sock";
const DAEMON_LOG_NAME: &str = "daemon.log";
const JOB_EVENTS_NAME: &str = "job-events.jsonl";

pub fn wrec_home() -> PathBuf {
    std::env::var_os("WREC_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".wrec")))
        .unwrap_or_else(|| PathBuf::from(".wrec"))
}

pub fn socket_path() -> PathBuf {
    wrec_home().join(SOCKET_NAME)
}

pub fn daemon_log_path() -> PathBuf {
    wrec_home().join(DAEMON_LOG_NAME)
}

pub fn job_events_path() -> PathBuf {
    wrec_home().join(JOB_EVENTS_NAME)
}

pub(crate) fn append_daemon_log(message: impl AsRef<str>) {
    append_line(
        &daemon_log_path(),
        &format!("{} {}", now_ms(), message.as_ref()),
    );
}

pub(crate) fn append_job_event(job_id: u64, event: &JobEvent) {
    if let Ok(value) = serde_json::to_string(&json!({
        "job_id": job_id,
        "event": event,
    })) {
        append_line(&job_events_path(), &value);
    }
}

pub(crate) fn append_line(path: &Path, line: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
