use control::{daemon_log_path, job_events_path, now_ms, JobEvent};
use serde_json::json;
use std::{fs::OpenOptions, io::Write, path::Path};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{env_lock, isolate_env};
    use control::EventLevel;

    #[test]
    fn append_line_creates_missing_parent_directories() {
        let _guard = env_lock();
        let path = isolate_env().join("nested").join("dirs").join("log.txt");

        append_line(&path, "one");

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "one\n");
    }

    #[test]
    fn append_line_appends_in_order() {
        let _guard = env_lock();
        let path = isolate_env().join("log.txt");

        append_line(&path, "one");
        append_line(&path, "two");

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "one\ntwo\n");
    }

    #[test]
    fn daemon_log_lives_under_wrec_home() {
        let _guard = env_lock();
        let home = isolate_env();

        append_daemon_log("hello daemon");

        let contents = std::fs::read_to_string(home.join("daemon.log")).unwrap();
        let (timestamp, message) = contents.trim_end().split_once(' ').unwrap();
        assert!(timestamp.parse::<u64>().unwrap() > 0);
        assert_eq!(message, "hello daemon");
    }

    #[test]
    fn job_events_are_json_lines_under_wrec_home() {
        let _guard = env_lock();
        let home = isolate_env();
        let event = JobEvent {
            timestamp_ms: now_ms(),
            level: EventLevel::Info,
            message: "recording active".into(),
            metrics: None,
        };

        append_job_event(42, &event);

        let contents = std::fs::read_to_string(home.join("job-events.jsonl")).unwrap();
        let value: serde_json::Value = serde_json::from_str(contents.trim_end()).unwrap();
        assert_eq!(value["job_id"], 42);
        assert_eq!(value["event"]["message"], "recording active");
    }
}
