use std::{
    ops::{Deref, DerefMut},
    os::fd::{AsRawFd, FromRawFd, IntoRawFd},
    str::FromStr,
    sync::Arc,
};

use genvm_common::*;
use tokio::io::AsyncBufReadExt;

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Copy)]
pub struct GenVMId(pub u64);

impl std::fmt::Display for GenVMId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub struct SingleGenVMContextDone {
    pub finished_at: chrono::DateTime<chrono::Utc>,
    pub stdout: String,
    pub stderr: String,
    pub genvm_log: Option<Vec<serde_json::Map<String, serde_json::Value>>>,
}

impl serde::Serialize for SingleGenVMContextDone {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("SingleGenVMContextDone", 3)?;
        state.serialize_field("finished_at", &self.finished_at.timestamp_millis())?;
        state.serialize_field("stdout", &self.stdout)?;
        state.serialize_field("stderr", &self.stderr)?;
        state.serialize_field("genvm_log", &self.genvm_log)?;
        state.end()
    }
}

struct SingleGenVMContext {
    id: GenVMId,
    version: String,
    result: tokio::sync::OnceCell<SingleGenVMContextDone>,
    started_at: chrono::DateTime<chrono::Utc>,
    strict_deadline: chrono::DateTime<chrono::Utc>,

    stdout_stderr_sem: Arc<tokio::sync::Semaphore>,
    stdout: tokio::sync::OnceCell<String>,
    stderr: tokio::sync::OnceCell<String>,
    log_capturer: Option<Arc<tokio::sync::Mutex<LogAppenderToValue>>>,

    process_handle: tokio::sync::Mutex<tokio::process::Child>,
    all_permits: crossbeam::atomic::AtomicCell<Option<Box<dyn std::any::Any + Send + Sync>>>,
}

struct PermitsData {
    max: usize,
    num_throttled: usize,
    throttled: Option<tokio::sync::OwnedSemaphorePermit>,
}

pub struct Ctx {
    known_executions: dashmap::DashMap<GenVMId, sync::DArc<SingleGenVMContext>>,
    next_genvm_id: std::sync::atomic::AtomicU64,
    pub permits: Arc<tokio::sync::Semaphore>,
    max_permits: tokio::sync::Mutex<PermitsData>,

    executors_path: std::path::PathBuf,
}

impl Ctx {
    pub fn executors_path(&self) -> &std::path::Path {
        &self.executors_path
    }

    pub async fn get_max_permits(&self) -> usize {
        let max_permits = self.max_permits.lock().await;
        max_permits.max
    }

    pub fn status_executions(&self) -> serde_json::Value {
        let mut ret = serde_json::Map::new();

        for kv in self.known_executions.iter() {
            let genvm_id = kv.key();
            let exec_ctx = kv.value();

            ret.insert(
                genvm_id.to_string(),
                serde_json::json!({
                    "has_result": exec_ctx.result.initialized(),
                    "version": exec_ctx.version,
                    "started_at": exec_ctx.started_at.to_rfc3339(),
                    "strict_deadline": exec_ctx.strict_deadline.to_rfc3339()
                }),
            );
        }

        ret.into()
    }

    pub fn get_current_permits(&self) -> usize {
        self.permits.available_permits()
    }

    pub async fn set_permits(&self, permits: usize) -> usize {
        let mut permits_lock = self.max_permits.lock().await;

        permits_lock.max += permits_lock.num_throttled;
        permits_lock.num_throttled = 0;
        permits_lock.throttled = None;
        // actually this causes drop of previous one, so we can enter more genvms than we have permits, but it's ok for now
        // especially since this method is expected to be called before starting any genvms at all

        if permits < 2 {
            log_warn!(permits = permits; "cannot set permits below 2");
            return permits_lock.max;
        }

        if permits_lock.max > permits {
            let delta = permits_lock.max - permits;
            let p = self
                .permits
                .clone()
                .acquire_many_owned(delta as u32)
                .await
                .unwrap();
            permits_lock.max = permits;
            permits_lock.num_throttled = delta;
            permits_lock.throttled = Some(p);
        } else {
            let delta = permits - permits_lock.max;
            self.permits.add_permits(delta);
            permits_lock.max = permits;
        }

        permits_lock.max
    }

    pub async fn graceful_shutdown(
        &self,
        genvm_id: GenVMId,
        wait_timeout_ms: u64,
    ) -> anyhow::Result<()> {
        let Some(exec_ctx) = self.known_executions.get(&genvm_id) else {
            anyhow::bail!("GenVM with id {} not found", genvm_id);
        };
        let exec_ctx = exec_ctx.value().clone();

        if exec_ctx.result.get().is_some() {
            return Ok(());
        }

        let mut child = exec_ctx.process_handle.lock().await;

        if let Ok(Some(_)) = child.try_wait() {
            log_trace!(genvm_id = genvm_id; "GenVM process already exited");
            return Ok(());
        }

        log_debug!(genvm_id = genvm_id; "sending SIGTERM to GenVM process");

        if let Some(pid) = child.id() {
            unsafe {
                libc::kill(pid as libc::c_int, libc::SIGTERM);
            }
        }

        let wait_duration = std::time::Duration::from_millis(wait_timeout_ms);
        let wait_start = std::time::Instant::now();

        loop {
            match child.try_wait() {
                Ok(Some(_)) => {
                    log_info!(genvm_id = genvm_id; "GenVM process terminated gracefully");
                    return Ok(());
                }
                Ok(None) => {
                    if wait_start.elapsed() >= wait_duration {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
                Err(e) => {
                    log_error!(genvm_id = genvm_id, error:err = e; "error checking process status");
                    anyhow::bail!("failed to check process status: {}", e);
                }
            }
        }

        log_warn!(genvm_id = genvm_id; "GenVM process did not terminate gracefully, sending SIGKILL");

        let _ = child.start_kill();
        child.wait().await?;

        Ok(())
    }

    pub async fn get_genvm_status(
        &self,
        genvm_id: GenVMId,
    ) -> Option<sync::DArc<SingleGenVMContextDone>> {
        let Some(exec_ctx) = self.known_executions.get(&genvm_id) else {
            log_trace!(genvm_id = genvm_id; "genvm status requested for unknown id");
            return None;
        };
        let exec_ctx = exec_ctx.value().clone();

        let proc_check = self.check_proc(&exec_ctx).await;
        log_debug!(genvm_id = genvm_id, proc_exited = proc_check; "genvm status checked");

        exec_ctx
            .clone()
            .into_get_sub(|data| data.result.get())
            .lift_option()
            .map(sync::DArcStruct::into_arc)
    }

    pub async fn fetch_genvm_status(
        &self,
        genvm_id: GenVMId,
    ) -> Option<sync::DArc<SingleGenVMContextDone>> {
        let res = self.get_genvm_status(genvm_id).await;
        if res.is_some() {
            self.known_executions.remove(&genvm_id);
        }

        res
    }

    #[cfg(target_os = "macos")]
    fn get_default_permits() -> usize {
        log_warn!("automatic permits detection is not supported on macOS, using default value");

        8
    }

    #[cfg(not(target_os = "macos"))]
    fn get_default_permits() -> usize {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_memory();
        sys.refresh_all();

        let free_memory = sys.free_memory();
        let free_swap = sys.free_swap();

        log_info!(
            free_memory = free_memory,
            free_swap = free_swap,
            free_memory_gb = free_memory as f64 / (1024.0 * 1024.0 * 1024.0),
            free_swap_gb = free_swap as f64 / (1024.0 * 1024.0 * 1024.0);
            "memory status"
        );

        let total_mem_kb = free_memory / 1024 + free_swap / 1024;
        let total_mem_gb = total_mem_kb / 1024 / 1024;

        (total_mem_gb / 4).max(2) as usize
    }

    pub fn new(config: &crate::manager::Config) -> anyhow::Result<Self> {
        let permits = if let Some(p) = config.permits {
            p
        } else {
            Self::get_default_permits()
        };
        log_info!(permits = permits; "estimated concurrent GenVM permits");

        let mut exe_path = std::env::current_exe()?;
        exe_path.pop();
        exe_path.pop();
        exe_path.push("executor");

        Ok(Self {
            known_executions: Default::default(),
            next_genvm_id: std::sync::atomic::AtomicU64::new(1),
            permits: Arc::new(tokio::sync::Semaphore::new(permits)),
            max_permits: tokio::sync::Mutex::new(PermitsData {
                max: permits,
                num_throttled: 0,
                throttled: None,
            }),

            executors_path: exe_path,
        })
    }
}

async fn gc_step(ctx: &sync::DArc<Ctx>) {
    let now = chrono::Utc::now();

    // copy-out so that we don't hold the lock while doing async operations
    let keys = ctx
        .known_executions
        .iter()
        .map(|kv| *kv.key())
        .collect::<Vec<_>>();

    for key in keys {
        let Some(val) = ctx.known_executions.get(&key) else {
            continue;
        };
        let val = val.clone();

        if val.strict_deadline < now {
            log_warn!(genvm_id = key; "genvm execution exceeded strict deadline, terminating");
            let _ = ctx.graceful_shutdown(key, 0).await;
        }
        let _ = ctx.check_proc(&val).await;
    }

    // Remove old finished executions
    ctx.known_executions.retain(|k, v| {
        let Some(result) = v.result.get() else {
            return true;
        };
        let passed = now.signed_duration_since(result.finished_at);
        if passed > chrono::Duration::seconds(60) || result.finished_at < now {
            log_warn!(genvm_id = *k; "removing zombie genvm execution context");
            return false;
        }
        true
    });
}

pub async fn start_service(
    ctx: sync::DArc<Ctx>,
    cancel: Arc<cancellation::Token>,
) -> anyhow::Result<()> {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel.chan.closed() => {
                    log_info!("shutting down run service waiter");
                    break;
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                    gc_step(&ctx).await;
                }
            }
        }
    });

    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Request {
    pub major: u32,
    pub message: serde_json::Value,
    pub is_sync: bool,
    pub capture_output: bool,
    #[serde(default = "default_max_execution_minutes")]
    pub max_execution_minutes: u64,
    pub host_data: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub host: String,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

fn default_max_execution_minutes() -> u64 {
    20
}

trait LogAppender {
    async fn append_structured(
        &mut self,
        level: logger::Level,
        data: serde_json::Map<String, serde_json::Value>,
    );
    async fn append_text(&mut self, serde_err: serde_json::Error, text: &str);
}

struct LogAppenderToLog(String);

impl Deref for LogAppenderToLog {
    type Target = Self;

    fn deref(&self) -> &Self::Target {
        self
    }
}

impl DerefMut for LogAppenderToLog {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self
    }
}

impl LogAppender for LogAppenderToLog {
    #[inline(always)]
    async fn append_structured(
        &mut self,
        level: logger::Level,
        data: serde_json::Map<String, serde_json::Value>,
    ) {
        log_with_level!(level, log:serde = data, genvm_id = self.0; "genvm log");
    }
    #[inline(always)]
    async fn append_text(&mut self, serde_err: serde_json::Error, text: &str) {
        log_info!(error:err = serde_err, genvm_id = self.0, line = text; "genvm log raw");
    }
}

struct LogAppenderToValue(Vec<serde_json::Map<String, serde_json::Value>>, GenVMId);

impl LogAppender for LogAppenderToValue {
    #[inline(always)]
    async fn append_structured(
        &mut self,
        level: logger::Level,
        mut data: serde_json::Map<String, serde_json::Value>,
    ) {
        log_debug!(reported_level = level, log:serde = data, genvm_id = self.1; "genvm log");

        data.insert("level".to_owned(), level.to_string().into());
        self.0.push(data);
    }

    #[inline(always)]
    async fn append_text(&mut self, serde_err: serde_json::Error, text: &str) {
        log_debug!(error:err = serde_err, genvm_id = self.1, line = text; "genvm log raw");

        self.0.push(serde_json::Map::from_iter([
            ("level".into(), serde_json::Value::String("info".into())),
            (
                "message".into(),
                serde_json::Value::String("genvm log".into()),
            ),
            ("line".into(), text.to_owned().into()),
        ]));
    }
}

async fn read_log_pipe<LA: LogAppender>(
    reader_fd: std::os::unix::io::OwnedFd,
    mut sink: impl DerefMut<Target = LA>,
) -> std::io::Result<()> {
    let file = unsafe { tokio::fs::File::from_raw_fd(reader_fd.into_raw_fd()) };

    let reader = tokio::io::BufReader::new(file);

    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str(line) {
            Ok::<serde_json::Map<String, serde_json::Value>, _>(mut log_record) => {
                let level = log_record
                    .remove("level")
                    .map(|x| {
                        if let serde_json::Value::String(s) = x {
                            Some(s)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(None)
                    .and_then(|x| logger::Level::from_str(&x).ok())
                    .unwrap_or(logger::Level::Info);

                sink.append_structured(level, log_record).await;
            }
            Err(e) => {
                sink.append_text(e, line).await;
            }
        }
    }

    Ok(())
}

async fn pipe_read<P: tokio::io::AsyncReadExt + Unpin>(
    mut reader: P,
    to: sync::DArc<tokio::sync::OnceCell<std::string::String>>,
    permit: tokio::sync::OwnedSemaphorePermit,
) {
    let mut result = String::new();
    let mut buf = vec![0u8; 8192];

    loop {
        let n = reader.read(&mut buf).await;

        log_trace!(n:? = n; "read from genvm stdout/stderr");

        let n = match n {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };

        match std::str::from_utf8(&buf[..n]) {
            Ok(s) => {
                log_trace!(data = s; "read from genvm stdout/stderr");
                result.push_str(s)
            }
            Err(_) => {
                let s = String::from_utf8_lossy(&buf[..n]);
                log_trace!(data = s; "read from genvm stdout/stderr");
                result.push_str(&s);
            }
        }
    }

    if let Err(e) = to.set(result) {
        log_error!(error:err = e; "failed to set stdout/stderr content");
    }

    std::mem::drop(permit);
}

impl Ctx {
    async fn check_proc(&self, exec: &SingleGenVMContext) -> bool {
        if exec.result.initialized() {
            return true;
        }

        let mut proc = exec.process_handle.lock().await;

        match proc.try_wait() {
            Ok(Some(status)) => {
                log_debug!(id = exec.id, status = status; "genvm exited");
                exec.all_permits.store(None); // drop all resources it owns
                let _ = exec.stdout_stderr_sem.acquire_many(2).await; // wait for stderr/stdout to be fully read
                log_debug!(id = exec.id, status = status; "stdout/stderr sem acquired");
                let stdout = exec.stdout.get().map(|x| x.as_str()).unwrap_or("");
                let stderr = exec.stderr.get().map(|x| x.as_str()).unwrap_or("");
                let genvm_log = if let Some(genvm_log) = &exec.log_capturer {
                    let mut data = Vec::new();
                    std::mem::swap(&mut data, &mut genvm_log.lock().await.0);

                    Some(data)
                } else {
                    None
                };

                if let Err(e) = exec.result.set(SingleGenVMContextDone {
                    finished_at: chrono::Utc::now(),
                    stdout: stdout.to_owned(),
                    stderr: stderr.to_owned(),
                    genvm_log,
                }) {
                    log_warn!(error:err = e; "error setting genvm result; it can happen rarely due to concurrency");
                }
                true
            }
            Ok(None) => false,
            Err(e) => {
                log_error!(error:err = e; "error checking process status");
                true
            }
        }
    }
}

pub async fn start_genvm(
    full_ctx: sync::DArc<crate::manager::AppContext>,
    req: Request,
    _modules_lock: Box<dyn std::any::Any + Send + Sync>,
) -> anyhow::Result<(GenVMId, tokio::sync::oneshot::Receiver<()>)> {
    let reroute_to = full_ctx.config.reroute_to.clone();

    let version = full_ctx
        .ver_ctx
        .get_version(req.major, req.timestamp)
        .await
        .ok_or_else(|| anyhow::anyhow!("no compatible version found for major {}", req.major))?;

    let ctx = full_ctx.into_gep(|x| &x.run_ctx);

    let genvm_id = GenVMId(
        ctx.next_genvm_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
    );

    let permits = if req.is_sync {
        ctx.permits.clone().acquire_owned().await?
    } else {
        ctx.permits.clone().acquire_many_owned(2).await?
    };

    let mut command_path = ctx.executors_path.clone();

    let version_str_owned = format!("v{}.{}.{}", version.major, version.minor, version.patch);
    let mut version_str: &str = &version_str_owned;
    if !reroute_to.is_empty() {
        version_str = &*reroute_to;
    }
    command_path.push(version_str);

    command_path.push("bin");
    command_path.push("genvm");

    log_debug!(genvm_id = genvm_id, exe:? = command_path, version:? = version; "genvm path");

    let mut proc = tokio::process::Command::new(command_path);

    let mut read_right_fd = [0; 2];
    let result_code = unsafe { libc::pipe(std::ptr::from_mut(&mut read_right_fd).cast()) };
    if result_code != 0 {
        anyhow::bail!("failed to create pipe for genvm logging: {result_code}");
    }

    let read_fd = read_right_fd[0];
    let write_fd = read_right_fd[1];

    let _ = unsafe { libc::fcntl(read_fd, libc::F_SETFD, libc::O_CLOEXEC) };

    let read_fd = unsafe { std::os::unix::io::OwnedFd::from_raw_fd(read_fd) };
    let write_fd = unsafe { std::os::unix::io::OwnedFd::from_raw_fd(write_fd) };

    proc.stdin(std::process::Stdio::null());
    proc.arg(format!("--log-fd={}", write_fd.as_raw_fd()));

    proc.arg("run");

    proc.args(&req.extra_args);

    if req.is_sync {
        proc.arg("--sync");
    }

    proc.arg("--host-data");
    proc.arg(&req.host_data);

    proc.arg("--host");
    proc.arg(&req.host);

    proc.arg("--message");
    proc.arg(serde_json::to_string(&req.message)?);

    proc.arg(format!("--cookie={}", genvm_id.0));

    let log_capturer = if req.capture_output {
        proc.stdout(std::process::Stdio::piped());
        proc.stderr(std::process::Stdio::piped());

        let logger = Arc::new(tokio::sync::Mutex::new(LogAppenderToValue(
            Vec::new(),
            genvm_id,
        )));
        let l = logger.clone().lock_owned().await;
        tokio::spawn(read_log_pipe(read_fd, l));

        Some(logger)
    } else {
        proc.stdout(std::process::Stdio::null());
        proc.stderr(std::process::Stdio::null());

        tokio::spawn(read_log_pipe(
            read_fd,
            LogAppenderToLog(format!("{}", genvm_id.0)),
        ));
        None
    };

    let mut child = proc.spawn()?;

    log_debug!(genvm_id = genvm_id, pid:? = child.id(); "genvm process started");

    let stdout_stderr_sem = Arc::new(tokio::sync::Semaphore::new(2));

    let stdout_perm = stdout_stderr_sem.clone().acquire_owned().await?;
    let stderr_perm = stdout_stderr_sem.clone().acquire_owned().await?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let all_resources = (permits, tx);

    let exec_ctx = sync::DArc::new(SingleGenVMContext {
        result: tokio::sync::OnceCell::new(),
        version: version_str.to_owned(),
        id: genvm_id,
        process_handle: tokio::sync::Mutex::new(child),
        started_at: chrono::Utc::now(),
        strict_deadline: chrono::Utc::now()
            + chrono::Duration::minutes(req.max_execution_minutes as i64),

        stdout_stderr_sem,
        stdout: tokio::sync::OnceCell::new(),
        stderr: tokio::sync::OnceCell::new(),
        log_capturer,

        all_permits: crossbeam::atomic::AtomicCell::new(Some(Box::new(all_resources))),
    });

    if let Some(stdout) = stdout {
        tokio::spawn(pipe_read(stdout, exec_ctx.gep(|x| &x.stdout), stdout_perm));
    }
    if let Some(stderr) = stderr {
        tokio::spawn(pipe_read(stderr, exec_ctx.gep(|x| &x.stderr), stderr_perm));
    }

    ctx.known_executions.insert(genvm_id, exec_ctx.clone());

    Ok((genvm_id, rx))
}
