use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, VecDeque},
    fs::OpenOptions,
    io::{BufRead, BufReader, ErrorKind, Write},
    os::unix::net::{UnixListener, UnixStream},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use wrec_backend::{
    build_settings_report, capture_kind_arg, load_config, resolve_target, selected_target_id,
    BackendEvent, RecordingOverrides, WrecBackend,
};
use wrec_core::{
    CaptureSourceKind, CaptureTarget, Codec, FrameRate, Quality, RecorderEngine, RecorderEvent,
    RecorderMetrics, RecorderSettings, Resolution,
};
use wrec_macos::MacosRecorder;

const SOCKET_NAME: &str = "wrec.sock";
const DAEMON_LOG_NAME: &str = "daemon.log";
const JOB_EVENTS_NAME: &str = "job-events.jsonl";
const STARTUP_TIMEOUT: Duration = Duration::from_secs(3);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const WAIT_POLL_INTERVAL: Duration = Duration::from_millis(500);

static TARGET_LIST_LOCK: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcRequest {
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    pub id: u64,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AgentError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
    pub next: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentWarning {
    pub code: String,
    pub message: String,
    pub next: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Starting,
    Recording,
    Paused,
    Finishing,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum TargetSelector {
    Id {
        kind: CaptureSourceKind,
        id: u64,
    },
    Name {
        kind: Option<CaptureSourceKind>,
        query: String,
    },
    App {
        query: String,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecordingOptions {
    pub source_kind: Option<CaptureSourceKind>,
    pub fps: Option<FrameRate>,
    pub codec: Option<Codec>,
    pub quality: Option<Quality>,
    pub resolution: Option<Resolution>,
    pub output_dir: Option<PathBuf>,
    pub include_cursor: Option<bool>,
    pub include_system_audio: Option<bool>,
    pub hide_wrec: Option<bool>,
}

impl From<&RecordingOptions> for RecordingOverrides {
    fn from(options: &RecordingOptions) -> Self {
        Self {
            source_kind: options.source_kind,
            target_id: None,
            fps: options.fps,
            codec: options.codec,
            quality: options.quality,
            resolution: options.resolution,
            output_dir: options.output_dir.clone(),
            include_cursor: options.include_cursor,
            include_system_audio: options.include_system_audio,
            hide_wrec: options.hide_wrec,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartRecordingParams {
    pub selector: Option<TargetSelector>,
    #[serde(default)]
    pub options: RecordingOptions,
    pub duration_ms: Option<u64>,
    #[serde(default = "default_queue")]
    pub queue: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSnapshot {
    pub id: u64,
    pub status: JobStatus,
    pub selector: Option<TargetSelector>,
    pub target: Option<CaptureTarget>,
    pub settings: Option<RecorderSettings>,
    pub output_path: Option<PathBuf>,
    pub queued_position: Option<usize>,
    pub warnings: Vec<AgentWarning>,
    pub events: Vec<JobEvent>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub started_at_ms: Option<u64>,
    pub finished_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEvent {
    pub timestamp_ms: u64,
    pub level: EventLevel,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<RecorderMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventLevel {
    Info,
    Warning,
    Error,
}

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

fn default_queue() -> bool {
    true
}

pub fn serve_forever() -> Result<(), String> {
    let home = wrec_home();
    std::fs::create_dir_all(&home)
        .map_err(|err| format!("failed to create {}: {err}", home.display()))?;
    let socket = socket_path();
    if socket.exists() {
        if UnixStream::connect(&socket).is_ok() {
            return Err(format!(
                "wrec daemon is already running at {}",
                socket.display()
            ));
        }
        std::fs::remove_file(&socket)
            .map_err(|err| format!("failed to remove stale socket {}: {err}", socket.display()))?;
    }

    append_daemon_log("daemon starting");
    let listener = UnixListener::bind(&socket)
        .map_err(|err| format!("failed to bind {}: {err}", socket.display()))?;
    listener
        .set_nonblocking(true)
        .map_err(|err| format!("failed to configure {}: {err}", socket.display()))?;
    let state = Arc::new(Mutex::new(Coordinator::new()));

    while !state.lock().unwrap().shutdown_requested {
        match listener.accept() {
            Ok((stream, _addr)) => {
                let state = state.clone();
                thread::spawn(move || handle_client(stream, state));
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                thread::sleep(POLL_INTERVAL);
            }
            Err(err) => append_daemon_log(format!("client accept failed: {err}")),
        }
    }

    append_daemon_log("daemon stopped");
    let _ = std::fs::remove_file(&socket);
    Ok(())
}

pub fn ensure_daemon() -> Result<(), AgentError> {
    if send_request("daemon.status", json!({})).is_ok() {
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
    let exe = std::env::current_exe().map_err(|err| AgentError {
        code: "daemon_start_failed".into(),
        message: format!("Could not locate current wrec executable: {err}"),
        recoverable: false,
        next: "Run `wrec daemon serve` manually from a known executable.".into(),
    })?;

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
        if send_request("daemon.status", json!({})).is_ok() {
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
    let mut stream = UnixStream::connect(socket_path()).map_err(|err| AgentError {
        code: "daemon_unreachable".into(),
        message: format!("Could not connect to {}: {err}", socket_path().display()),
        recoverable: true,
        next: "Run `wrec daemon start` or retry a command that auto-starts the daemon.".into(),
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
    writeln!(stream, "{line}").map_err(|err| AgentError {
        code: "request_write_failed".into(),
        message: format!("Could not write IPC request: {err}"),
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
    serde_json::from_str(&response).map_err(|err| AgentError {
        code: "response_decode_failed".into(),
        message: format!("Could not decode IPC response: {err}"),
        recoverable: false,
        next: "Inspect ~/.wrec/daemon.log and report this as a wrec IPC protocol bug.".into(),
    })
}

pub fn wait_for_job(job_id: u64, json_output: bool) -> Result<JobSnapshot, AgentError> {
    let mut seen_events = 0;
    loop {
        let response = send_request("job.show", json!({ "job_id": job_id }))?;
        if !response.ok {
            return Err(response.error.unwrap_or_else(generic_daemon_error));
        }
        let job: JobSnapshot = serde_json::from_value(
            response
                .result
                .unwrap_or_else(|| json!({ "job": null }))
                .get("job")
                .cloned()
                .unwrap_or(Value::Null),
        )
        .map_err(|err| AgentError {
            code: "job_decode_failed".into(),
            message: format!("Could not decode job {job_id}: {err}"),
            recoverable: false,
            next: "Inspect `wrec job show {job_id} --json` and report the protocol mismatch."
                .into(),
        })?;

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

fn handle_client(stream: UnixStream, state: Arc<Mutex<Coordinator>>) {
    let mut line = String::new();
    let mut reader = BufReader::new(&stream);
    let response = match reader.read_line(&mut line) {
        Ok(0) => response_error(
            0,
            AgentError {
                code: "empty_request".into(),
                message: "IPC request was empty".into(),
                recoverable: true,
                next: "Retry the command.".into(),
            },
        ),
        Ok(_) => match serde_json::from_str::<IpcRequest>(&line) {
            Ok(request) => handle_request(request, state),
            Err(err) => response_error(
                0,
                AgentError {
                    code: "request_decode_failed".into(),
                    message: format!("Could not decode IPC request: {err}"),
                    recoverable: false,
                    next: "Report this as a wrec IPC protocol bug.".into(),
                },
            ),
        },
        Err(err) => response_error(
            0,
            AgentError {
                code: "request_read_failed".into(),
                message: format!("Could not read IPC request: {err}"),
                recoverable: true,
                next: "Retry the command.".into(),
            },
        ),
    };

    if let Ok(line) = serde_json::to_string(&response) {
        let mut stream = stream;
        let _ = writeln!(stream, "{line}");
    }
}

fn handle_request(request: IpcRequest, state: Arc<Mutex<Coordinator>>) -> IpcResponse {
    let result = match request.method.as_str() {
        "daemon.status" => Ok(state.lock().unwrap().status()),
        "daemon.stop" => Coordinator::daemon_stop(state),
        "targets.list" => Coordinator::targets_list(state),
        "record.start" => serde_json::from_value::<StartRecordingParams>(request.params)
            .map_err(|err| AgentError {
                code: "invalid_record_request".into(),
                message: format!("Could not parse record.start params: {err}"),
                recoverable: false,
                next: "Check the IPC request shape or use `wrec record start --help`.".into(),
            })
            .and_then(|params| Coordinator::record_start(state, params)),
        "jobs.list" => Ok(state.lock().unwrap().jobs_list()),
        "job.show" => job_id_param(&request.params, "job.show")
            .and_then(|job_id| state.lock().unwrap().job_show(job_id)),
        "job.logs" => job_id_param(&request.params, "job.logs")
            .and_then(|job_id| state.lock().unwrap().job_logs(job_id)),
        "job.cancel" => job_id_param(&request.params, "job.cancel")
            .and_then(|job_id| Coordinator::job_cancel(state, job_id)),
        "job.pause" => job_id_param(&request.params, "job.pause")
            .and_then(|job_id| Coordinator::job_pause(state, job_id)),
        "job.resume" => job_id_param(&request.params, "job.resume")
            .and_then(|job_id| Coordinator::job_resume(state, job_id)),
        "job.stop" => job_id_param(&request.params, "job.stop")
            .and_then(|job_id| Coordinator::job_stop(state, job_id)),
        other => Err(AgentError {
            code: "unknown_method".into(),
            message: format!("Unknown IPC method `{other}`"),
            recoverable: false,
            next: "Use a supported wrec CLI command instead of calling this method directly."
                .into(),
        }),
    };

    match result {
        Ok(value) => IpcResponse {
            id: request.id,
            ok: true,
            result: Some(value),
            error: None,
        },
        Err(error) => response_error(request.id, error),
    }
}

fn response_error(id: u64, error: AgentError) -> IpcResponse {
    IpcResponse {
        id,
        ok: false,
        result: None,
        error: Some(error),
    }
}

fn job_id_param(params: &Value, method: &str) -> Result<u64, AgentError> {
    params
        .get("job_id")
        .and_then(Value::as_u64)
        .ok_or_else(|| AgentError {
            code: "missing_job_id".into(),
            message: format!("{method} requires job_id"),
            recoverable: false,
            next: "Pass a numeric job id, for example `wrec job show 42`.".into(),
        })
}

struct Coordinator {
    backend: WrecBackend,
    jobs: BTreeMap<u64, JobRecord>,
    queue: VecDeque<u64>,
    target_cache: Vec<CaptureTarget>,
    active_job_id: Option<u64>,
    next_job_id: u64,
    shutdown_requested: bool,
}

impl Coordinator {
    fn new() -> Self {
        Self {
            backend: WrecBackend::open(),
            jobs: BTreeMap::new(),
            queue: VecDeque::new(),
            target_cache: Vec::new(),
            active_job_id: None,
            next_job_id: now_ms(),
            shutdown_requested: false,
        }
    }

    fn status(&self) -> Value {
        json!({
            "pid": std::process::id(),
            "home": wrec_home(),
            "socket": socket_path(),
            "daemon_log": daemon_log_path(),
            "job_events": job_events_path(),
            "active_job_id": self.active_job_id,
            "queued_jobs": self.queue.iter().copied().collect::<Vec<_>>(),
            "stopping": self.shutdown_requested,
        })
    }

    fn daemon_stop(state: Arc<Mutex<Self>>) -> Result<Value, AgentError> {
        let mut state = state.lock().unwrap();
        if let Some(active) = state.active_job_id {
            return Err(AgentError {
                code: "daemon_busy".into(),
                message: format!("Daemon cannot stop gracefully while job {active} is active."),
                recoverable: true,
                next: format!(
                    "Use `wrec job stop {active}`, wait for it to finish, then retry `wrec daemon stop`."
                ),
            });
        }
        if !state.queue.is_empty() {
            return Err(AgentError {
                code: "daemon_busy".into(),
                message: format!(
                    "Daemon cannot stop gracefully with {} queued job(s).",
                    state.queue.len()
                ),
                recoverable: true,
                next:
                    "Run `wrec jobs --json`, cancel queued jobs with `wrec job cancel <id>`, then retry `wrec daemon stop`."
                        .into(),
            });
        }

        state.shutdown_requested = true;
        append_daemon_log("shutdown requested");
        Ok(json!({
            "stopping": true,
            "home": wrec_home(),
            "socket": socket_path(),
            "daemon_log": daemon_log_path(),
        }))
    }

    fn targets_list(state: Arc<Mutex<Self>>) -> Result<Value, AgentError> {
        let _guard = target_list_lock().lock().unwrap();
        let targets = Self::list_targets_direct()?;
        state.lock().unwrap().target_cache = targets.clone();
        Ok(json!({ "targets": targets }))
    }

    fn list_targets_direct() -> Result<Vec<CaptureTarget>, AgentError> {
        let (tx, _rx) = mpsc::channel();
        let engine = MacosRecorder::new(tx);
        engine.list_targets().map_err(|err| AgentError {
            code: "target_listing_failed".into(),
            message: err.to_string(),
            recoverable: true,
            next: "Run `wrec targets --json` again; if this repeats, check Screen Recording permission and ~/.wrec/daemon.log.".into(),
        })
    }

    fn record_start(
        state: Arc<Mutex<Self>>,
        params: StartRecordingParams,
    ) -> Result<Value, AgentError> {
        if state.lock().unwrap().shutdown_requested {
            return Err(AgentError {
                code: "daemon_stopping".into(),
                message: "Daemon is stopping and is not accepting new recording jobs.".into(),
                recoverable: true,
                next: "Wait a moment, then run `wrec daemon start` and retry the recording.".into(),
            });
        }

        let (job, should_launch) = {
            let config = load_config();
            let overrides = RecordingOverrides::from(&params.options);
            let (settings, warning) = build_settings_report(&config.settings, &overrides);
            let warnings = warning
                .map(|message| AgentWarning {
                    code: "preset_limited".into(),
                    message,
                    next: "Use --quality high to allow native/60 FPS, or accept the effective capped settings.".into(),
                })
                .into_iter()
                .collect::<Vec<_>>();
            let targets = Self::targets_for_submission(state.clone())?;
            let target = resolve_record_target(
                &targets,
                settings.source,
                params.selector.as_ref(),
                selected_target_id(&config, settings.source),
            )?;
            let settings = settings_for_target(settings, &target);

            let mut state = state.lock().unwrap();
            let id = state.allocate_job_id();
            let mut job = JobRecord::new(
                id,
                params.selector,
                target,
                settings,
                params.duration_ms,
                warnings,
            );

            let should_launch = state.active_job_id.is_none();
            if should_launch {
                job.status = JobStatus::Starting;
                job.started_at_ms = Some(now_ms());
                job.push_event(EventLevel::Info, "job starting");
                state.active_job_id = Some(id);
            } else if params.queue {
                state.queue.push_back(id);
                job.push_event(
                    EventLevel::Info,
                    format!(
                        "job queued behind active job {}",
                        state.active_job_id.unwrap_or_default()
                    ),
                );
            } else {
                return Err(AgentError {
                    code: "recording_active".into(),
                    message: format!(
                        "Job {} is already active; this request was not queued.",
                        state.active_job_id.unwrap_or_default()
                    ),
                    recoverable: true,
                    next: "Retry with `--queue`, wait for the active job, or stop it with `wrec job stop <id>`.".into(),
                });
            }

            let snapshot = job.snapshot(state.queued_position(id));
            state.jobs.insert(id, job);
            append_daemon_log(format!("accepted job {id}"));
            (snapshot, should_launch)
        };

        if should_launch {
            launch_job(state.clone(), job.id);
        }

        Ok(json!({
            "job": job,
            "next": if should_launch {
                "Job is starting. Use `wrec job show <id> --json` to inspect it."
            } else {
                "Job is queued. Use `wrec jobs --json` to watch queue position."
            }
        }))
    }

    fn targets_for_submission(state: Arc<Mutex<Self>>) -> Result<Vec<CaptureTarget>, AgentError> {
        if let Some(targets) = {
            let state = state.lock().unwrap();
            (state.active_job_id.is_some() && !state.target_cache.is_empty())
                .then(|| state.target_cache.clone())
        } {
            return Ok(targets);
        }

        let _guard = target_list_lock().lock().unwrap();
        if let Some(targets) = {
            let state = state.lock().unwrap();
            (state.active_job_id.is_some() && !state.target_cache.is_empty())
                .then(|| state.target_cache.clone())
        } {
            return Ok(targets);
        }

        let targets = Self::list_targets_direct()?;
        state.lock().unwrap().target_cache = targets.clone();
        Ok(targets)
    }

    fn jobs_list(&self) -> Value {
        let jobs = self
            .jobs
            .values()
            .map(|job| job.snapshot(self.queued_position(job.id)))
            .collect::<Vec<_>>();
        json!({ "jobs": jobs, "active_job_id": self.active_job_id })
    }

    fn job_show(&self, id: u64) -> Result<Value, AgentError> {
        let job = self.jobs.get(&id).ok_or_else(|| missing_job_error(id))?;
        Ok(json!({ "job": job.snapshot(self.queued_position(id)) }))
    }

    fn job_logs(&self, id: u64) -> Result<Value, AgentError> {
        let job = self.jobs.get(&id).ok_or_else(|| missing_job_error(id))?;
        Ok(json!({ "job_id": id, "events": job.events }))
    }

    fn job_cancel(state: Arc<Mutex<Self>>, id: u64) -> Result<Value, AgentError> {
        let mut state = state.lock().unwrap();
        if state.active_job_id == Some(id) {
            return Err(AgentError {
                code: "job_active".into(),
                message: format!("Job {id} is active and cannot be cancelled as a queued job."),
                recoverable: true,
                next: format!("Use `wrec job stop {id}` to stop the active recording."),
            });
        }
        let Some(position) = state.queue.iter().position(|job_id| *job_id == id) else {
            return Err(missing_job_error(id));
        };
        state.queue.remove(position);
        let queued_position = state.queued_position(id);
        let job = state
            .jobs
            .get_mut(&id)
            .ok_or_else(|| missing_job_error(id))?;
        job.status = JobStatus::Cancelled;
        job.finished_at_ms = Some(now_ms());
        job.push_event(EventLevel::Warning, "queued job cancelled");
        Ok(json!({ "job": job.snapshot(queued_position) }))
    }

    fn job_pause(state: Arc<Mutex<Self>>, id: u64) -> Result<Value, AgentError> {
        let control = {
            let mut state = state.lock().unwrap();
            let job = active_job_mut(&mut state, id)?;
            if job.status != JobStatus::Recording {
                return Err(job_state_error(
                    id,
                    "job_not_recording",
                    format!("Job {id} is {} and cannot be paused.", status_name(&job.status)),
                    "Wait until the job status is recording, or inspect it with `wrec job show <id> --json`.",
                ));
            }
            job.control.clone()
        };

        let Some(control) = control else {
            return Err(missing_job_control_error(id));
        };
        if let Err(err) = control.lock().unwrap().pause() {
            return Err(record_control_error(
                "job_pause_failed",
                id,
                err.to_string(),
            ));
        }

        let mut state = state.lock().unwrap();
        let job = active_job_mut(&mut state, id)?;
        job.status = JobStatus::Paused;
        job.push_event(EventLevel::Info, "pause requested");
        Ok(json!({ "job": job.snapshot(None) }))
    }

    fn job_resume(state: Arc<Mutex<Self>>, id: u64) -> Result<Value, AgentError> {
        let control =
            {
                let mut state = state.lock().unwrap();
                let job = active_job_mut(&mut state, id)?;
                if job.status != JobStatus::Paused {
                    return Err(job_state_error(
                    id,
                    "job_not_paused",
                    format!("Job {id} is {} and cannot be resumed.", status_name(&job.status)),
                    "Pause the active job first, or inspect it with `wrec job show <id> --json`.",
                ));
                }
                job.control.clone()
            };

        let Some(control) = control else {
            return Err(missing_job_control_error(id));
        };
        if let Err(err) = control.lock().unwrap().resume() {
            return Err(record_control_error(
                "job_resume_failed",
                id,
                err.to_string(),
            ));
        }

        let mut state = state.lock().unwrap();
        let job = active_job_mut(&mut state, id)?;
        job.status = JobStatus::Recording;
        job.push_event(EventLevel::Info, "resume requested");
        Ok(json!({ "job": job.snapshot(None) }))
    }

    fn job_stop(state: Arc<Mutex<Self>>, id: u64) -> Result<Value, AgentError> {
        let control = {
            let mut state = state.lock().unwrap();
            let job = active_job_mut(&mut state, id)?;
            let control = job
                .control
                .clone()
                .ok_or_else(|| missing_job_control_error(id))?;
            job.status = JobStatus::Finishing;
            job.push_event(EventLevel::Info, "stop requested");
            control
        };

        if let Err(err) = control.lock().unwrap().stop() {
            let mut state = state.lock().unwrap();
            if let Ok(job) = active_job_mut(&mut state, id) {
                job.status = JobStatus::Recording;
                job.push_event(EventLevel::Error, format!("stop failed: {err}"));
            }
            return Err(record_control_error("job_stop_failed", id, err.to_string()));
        }

        let state = state.lock().unwrap();
        let job = state.jobs.get(&id).ok_or_else(|| missing_job_error(id))?;
        Ok(json!({ "job": job.snapshot(state.queued_position(id)) }))
    }

    fn allocate_job_id(&mut self) -> u64 {
        self.next_job_id = self.next_job_id.saturating_add(1);
        self.next_job_id
    }

    fn queued_position(&self, id: u64) -> Option<usize> {
        self.queue
            .iter()
            .position(|job_id| *job_id == id)
            .map(|index| index + 1)
    }
}

struct JobRecord {
    id: u64,
    status: JobStatus,
    selector: Option<TargetSelector>,
    target: CaptureTarget,
    settings: RecorderSettings,
    output_path: Option<PathBuf>,
    duration_ms: Option<u64>,
    warnings: Vec<AgentWarning>,
    events: Vec<JobEvent>,
    control: Option<Arc<Mutex<MacosRecorder>>>,
    created_at_ms: u64,
    updated_at_ms: u64,
    started_at_ms: Option<u64>,
    finished_at_ms: Option<u64>,
}

impl JobRecord {
    fn new(
        id: u64,
        selector: Option<TargetSelector>,
        target: CaptureTarget,
        settings: RecorderSettings,
        duration_ms: Option<u64>,
        warnings: Vec<AgentWarning>,
    ) -> Self {
        let now = now_ms();
        Self {
            id,
            status: JobStatus::Queued,
            selector,
            target,
            settings,
            output_path: None,
            duration_ms,
            warnings,
            events: Vec::new(),
            control: None,
            created_at_ms: now,
            updated_at_ms: now,
            started_at_ms: None,
            finished_at_ms: None,
        }
    }

    fn snapshot(&self, queued_position: Option<usize>) -> JobSnapshot {
        JobSnapshot {
            id: self.id,
            status: self.status.clone(),
            selector: self.selector.clone(),
            target: Some(self.target.clone()),
            settings: Some(self.settings.clone()),
            output_path: self.output_path.clone(),
            queued_position,
            warnings: self.warnings.clone(),
            events: self.events.clone(),
            created_at_ms: self.created_at_ms,
            updated_at_ms: self.updated_at_ms,
            started_at_ms: self.started_at_ms,
            finished_at_ms: self.finished_at_ms,
        }
    }

    fn push_event(&mut self, level: EventLevel, message: impl Into<String>) {
        let event = JobEvent {
            timestamp_ms: now_ms(),
            level,
            message: message.into(),
            metrics: None,
        };
        append_job_event(self.id, &event);
        self.events.push(event);
        self.updated_at_ms = now_ms();
    }

    fn push_metrics(&mut self, metrics: RecorderMetrics) {
        let event = JobEvent {
            timestamp_ms: now_ms(),
            level: EventLevel::Info,
            message: format!(
                "{}s  {} bytes  {:.2} Mbps",
                metrics.elapsed_secs, metrics.output_bytes, metrics.estimated_bitrate_mbps
            ),
            metrics: Some(metrics),
        };
        append_job_event(self.id, &event);
        self.events.push(event);
        self.updated_at_ms = now_ms();
    }
}

fn launch_job(state: Arc<Mutex<Coordinator>>, job_id: u64) {
    let (target, settings, duration_ms, engine, rx) = {
        let (tx, rx) = mpsc::channel();
        let engine = Arc::new(Mutex::new(MacosRecorder::new(tx)));
        let mut state = state.lock().unwrap();
        let Some(job) = state.jobs.get_mut(&job_id) else {
            return;
        };
        job.status = JobStatus::Starting;
        job.started_at_ms = Some(now_ms());
        job.control = Some(engine.clone());
        (
            job.target.clone(),
            job.settings.clone(),
            job.duration_ms,
            engine,
            rx,
        )
    };

    thread::spawn(move || {
        run_job(
            state.clone(),
            job_id,
            target,
            settings,
            duration_ms,
            engine,
            rx,
        );
        launch_next_queued_job(state);
    });
}

fn run_job(
    state: Arc<Mutex<Coordinator>>,
    job_id: u64,
    target: CaptureTarget,
    settings: RecorderSettings,
    duration_ms: Option<u64>,
    engine: Arc<Mutex<MacosRecorder>>,
    rx: mpsc::Receiver<RecorderEvent>,
) {
    append_daemon_log(format!("job {job_id} starting"));
    let started = Instant::now();
    let start_result = engine.lock().unwrap().start(target, settings);
    if let Err(err) = start_result {
        drain_recorder_events(&state, job_id, &rx);
        finish_job_failed(&state, job_id, format!("recording failed to start: {err}"));
        return;
    }

    {
        let mut state = state.lock().unwrap();
        if let Some(job) = state.jobs.get_mut(&job_id) {
            job.status = JobStatus::Recording;
            job.push_event(EventLevel::Info, "recording active");
        }
    }

    let mut duration_stop_requested = false;
    loop {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(event) => {
                let done = handle_recorder_event(&state, job_id, event);
                if done {
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                finish_job_failed(&state, job_id, "recorder event channel closed");
                break;
            }
        }

        if let Some(duration_ms) = duration_ms {
            if !duration_stop_requested && started.elapsed() >= Duration::from_millis(duration_ms) {
                duration_stop_requested = true;
                append_job_message(
                    &state,
                    job_id,
                    EventLevel::Info,
                    "duration elapsed; stopping",
                );
                let _ = engine.lock().unwrap().stop();
            }
        }
    }
}

fn drain_recorder_events(
    state: &Arc<Mutex<Coordinator>>,
    job_id: u64,
    rx: &mpsc::Receiver<RecorderEvent>,
) {
    while let Ok(event) = rx.try_recv() {
        handle_recorder_event(state, job_id, event);
    }
}

fn handle_recorder_event(
    state: &Arc<Mutex<Coordinator>>,
    job_id: u64,
    event: RecorderEvent,
) -> bool {
    let mut state = state.lock().unwrap();
    let backend_event = state.backend.handle_recorder_event(&event);
    let job = match state.jobs.get_mut(&job_id) {
        Some(job) => job,
        None => return true,
    };

    match backend_event {
        BackendEvent::Starting { output_path, .. } => {
            job.output_path = Some(output_path.clone());
            job.push_event(
                EventLevel::Info,
                format!("starting capture -> {}", output_path.display()),
            );
            false
        }
        BackendEvent::Log { message, .. } => {
            job.push_event(EventLevel::Info, message);
            false
        }
        BackendEvent::Metrics { metrics, .. } => {
            job.push_metrics(metrics);
            false
        }
        BackendEvent::Failed { message, .. } => {
            job.status = JobStatus::Failed;
            job.finished_at_ms = Some(now_ms());
            job.control = None;
            job.push_event(EventLevel::Error, message);
            state.active_job_id = None;
            true
        }
        BackendEvent::Exited {
            success,
            status,
            output_path,
            ..
        } => {
            job.output_path = output_path.or_else(|| job.output_path.clone());
            job.status = if success {
                JobStatus::Completed
            } else {
                JobStatus::Failed
            };
            job.finished_at_ms = Some(now_ms());
            job.control = None;
            job.push_event(
                if success {
                    EventLevel::Info
                } else {
                    EventLevel::Error
                },
                format!("helper exited: {status}"),
            );
            state.active_job_id = None;
            true
        }
    }
}

fn finish_job_failed(state: &Arc<Mutex<Coordinator>>, job_id: u64, message: impl Into<String>) {
    let mut state = state.lock().unwrap();
    if let Some(job) = state.jobs.get_mut(&job_id) {
        job.status = JobStatus::Failed;
        job.finished_at_ms = Some(now_ms());
        job.control = None;
        job.push_event(EventLevel::Error, message);
    }
    state.active_job_id = None;
}

fn append_job_message(
    state: &Arc<Mutex<Coordinator>>,
    job_id: u64,
    level: EventLevel,
    message: impl Into<String>,
) {
    let mut state = state.lock().unwrap();
    if let Some(job) = state.jobs.get_mut(&job_id) {
        job.push_event(level, message);
    }
}

fn launch_next_queued_job(state: Arc<Mutex<Coordinator>>) {
    let next_job = {
        let mut state = state.lock().unwrap();
        if state.active_job_id.is_some() {
            None
        } else {
            let next = state.queue.pop_front();
            if let Some(job_id) = next {
                state.active_job_id = Some(job_id);
            }
            next
        }
    };

    if let Some(job_id) = next_job {
        launch_job(state, job_id);
    }
}

fn resolve_record_target(
    targets: &[CaptureTarget],
    kind: CaptureSourceKind,
    selector: Option<&TargetSelector>,
    saved_id: Option<u64>,
) -> Result<CaptureTarget, AgentError> {
    match selector {
        Some(TargetSelector::Id { kind, id }) => {
            resolve_target(targets, *kind, Some(*id), None).map_err(target_error)
        }
        Some(TargetSelector::Name { kind, query }) => {
            let candidates = targets
                .iter()
                .filter(|target| kind.map_or(true, |kind| target.kind == kind))
                .collect::<Vec<_>>();
            resolve_by_name(candidates, query, "target")
        }
        Some(TargetSelector::App { query }) => {
            let candidates = targets
                .iter()
                .filter(|target| target.kind == CaptureSourceKind::Window)
                .collect::<Vec<_>>();
            resolve_by_app(candidates, query)
        }
        None => resolve_target(targets, kind, None, saved_id).map_err(target_error),
    }
}

fn settings_for_target(mut settings: RecorderSettings, target: &CaptureTarget) -> RecorderSettings {
    settings.source = target.kind;
    settings
}

fn resolve_by_name(
    candidates: Vec<&CaptureTarget>,
    query: &str,
    label: &str,
) -> Result<CaptureTarget, AgentError> {
    let query = normalized(query);
    if query.is_empty() {
        return Err(AgentError {
            code: "empty_target_query".into(),
            message: format!("{label} query cannot be empty"),
            recoverable: true,
            next: "Pass a non-empty target name or use `wrec targets --json` to choose an id."
                .into(),
        });
    }

    for predicate in [MatchKind::Exact, MatchKind::Prefix, MatchKind::Contains] {
        let matches = candidates
            .iter()
            .copied()
            .filter(|target| match predicate {
                MatchKind::Exact => normalized(&target.name) == query,
                MatchKind::Prefix => normalized(&target.name).starts_with(&query),
                MatchKind::Contains => normalized(&target.name).contains(&query),
            })
            .collect::<Vec<_>>();
        if !matches.is_empty() {
            return unique_match(matches, label, &query);
        }
    }

    Err(AgentError {
        code: "target_not_found".into(),
        message: format!("no {label} matches `{query}`"),
        recoverable: true,
        next: "Run `wrec targets --json` and pass `--target kind:id` for an exact target.".into(),
    })
}

fn resolve_by_app(
    candidates: Vec<&CaptureTarget>,
    query: &str,
) -> Result<CaptureTarget, AgentError> {
    let query = normalized(query);
    if query.is_empty() {
        return Err(AgentError {
            code: "empty_app_query".into(),
            message: "app query cannot be empty".into(),
            recoverable: true,
            next: "Pass an app name or use `wrec targets --json` to choose a window id.".into(),
        });
    }

    for predicate in [MatchKind::Exact, MatchKind::Prefix, MatchKind::Contains] {
        let matches = candidates
            .iter()
            .copied()
            .filter(|target| match predicate {
                MatchKind::Exact => normalized(app_name(target)) == query,
                MatchKind::Prefix => normalized(app_name(target)).starts_with(&query),
                MatchKind::Contains => normalized(app_name(target)).contains(&query),
            })
            .collect::<Vec<_>>();
        if !matches.is_empty() {
            return unique_match(matches, "app", &query);
        }
    }

    Err(AgentError {
        code: "app_not_found".into(),
        message: format!("no app matches `{query}`"),
        recoverable: true,
        next: "Run `wrec targets --json` and pass `--target window:id` for an exact window.".into(),
    })
}

enum MatchKind {
    Exact,
    Prefix,
    Contains,
}

fn unique_match(
    matches: Vec<&CaptureTarget>,
    label: &str,
    query: &str,
) -> Result<CaptureTarget, AgentError> {
    match matches.as_slice() {
        [target] => Ok((*target).clone()),
        _ => Err(AgentError {
            code: "ambiguous_target".into(),
            message: format!(
                "multiple {label}s match `{query}`: {}",
                matches
                    .iter()
                    .map(|target| {
                        format!(
                            "{}:{} {}",
                            capture_kind_arg(target.kind),
                            target.id,
                            target.name
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            recoverable: true,
            next: "Pass `--target kind:id` to choose one exact target.".into(),
        }),
    }
}

fn normalized(value: &str) -> String {
    value.trim().to_lowercase()
}

fn app_name(target: &CaptureTarget) -> &str {
    target
        .name
        .split_once(" \u{2014} ")
        .map(|(app, _)| app)
        .unwrap_or(&target.name)
}

fn target_error(message: String) -> AgentError {
    AgentError {
        code: "target_not_found".into(),
        message,
        recoverable: true,
        next: "Run `wrec targets --json` and pass one of the listed target ids.".into(),
    }
}

fn active_job_mut(state: &mut Coordinator, id: u64) -> Result<&mut JobRecord, AgentError> {
    if state.active_job_id != Some(id) {
        return Err(AgentError {
            code: "job_not_active".into(),
            message: format!("Job {id} is not the active recording."),
            recoverable: true,
            next: "Use `wrec jobs --json` to find the active job, or `wrec job cancel <id>` for queued jobs.".into(),
        });
    }
    state.jobs.get_mut(&id).ok_or_else(|| missing_job_error(id))
}

fn job_state_error(
    id: u64,
    code: &str,
    message: impl Into<String>,
    next: impl Into<String>,
) -> AgentError {
    AgentError {
        code: code.into(),
        message: message.into(),
        recoverable: true,
        next: next.into().replace("<id>", &id.to_string()),
    }
}

fn missing_job_control_error(id: u64) -> AgentError {
    AgentError {
        code: "job_control_missing".into(),
        message: format!("Job {id} does not have an active recorder handle."),
        recoverable: true,
        next: "Wait for the job to fail or inspect `wrec job show <id> --json`.".into(),
    }
}

fn record_control_error(code: &str, id: u64, message: String) -> AgentError {
    AgentError {
        code: code.into(),
        message,
        recoverable: true,
        next: format!(
            "Inspect `wrec job show {id} --json`; if the helper is stuck, restart the daemon."
        ),
    }
}

fn status_name(status: &JobStatus) -> &'static str {
    match status {
        JobStatus::Queued => "queued",
        JobStatus::Starting => "starting",
        JobStatus::Recording => "recording",
        JobStatus::Paused => "paused",
        JobStatus::Finishing => "finishing",
        JobStatus::Completed => "completed",
        JobStatus::Failed => "failed",
        JobStatus::Cancelled => "cancelled",
    }
}

fn missing_job_error(id: u64) -> AgentError {
    AgentError {
        code: "job_not_found".into(),
        message: format!("No job with id {id} is known to this daemon."),
        recoverable: true,
        next: "Run `wrec jobs --json` to list jobs known to the current daemon.".into(),
    }
}

fn generic_daemon_error() -> AgentError {
    AgentError {
        code: "daemon_error".into(),
        message: "Daemon returned an error without details.".into(),
        recoverable: true,
        next: "Retry the command or inspect ~/.wrec/daemon.log.".into(),
    }
}

fn append_daemon_log(message: impl AsRef<str>) {
    append_line(
        &daemon_log_path(),
        &format!("{} {}", now_ms(), message.as_ref()),
    );
}

fn target_list_lock() -> &'static Mutex<()> {
    TARGET_LIST_LOCK.get_or_init(|| Mutex::new(()))
}

fn append_job_event(job_id: u64, event: &JobEvent) {
    if let Ok(value) = serde_json::to_string(&json!({
        "job_id": job_id,
        "event": event,
    })) {
        append_line(&job_events_path(), &value);
    }
}

fn append_line(path: &Path, line: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_start_params_queue_by_default() {
        let params: StartRecordingParams = serde_json::from_value(json!({})).unwrap();

        assert!(params.queue);
    }

    #[test]
    fn job_status_serializes_for_agents() {
        assert_eq!(
            serde_json::to_string(&JobStatus::Paused).unwrap(),
            "\"paused\""
        );
    }

    #[test]
    fn settings_source_follows_resolved_target() {
        let target = CaptureTarget {
            id: 42,
            name: "Notes - Draft".into(),
            kind: CaptureSourceKind::Window,
        };
        let settings = settings_for_target(RecorderSettings::default(), &target);

        assert_eq!(settings.source, CaptureSourceKind::Window);
    }
}
