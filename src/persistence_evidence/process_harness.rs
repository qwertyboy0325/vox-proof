//! Child-process harness for real process interruption evidence.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

pub const READY_LINE: &str = "VOXPROOF_READY";
pub const RESULT_PREFIX: &str = "VOXPROOF_RESULT:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessExitClassification {
    Success,
    AbnormalTermination,
    Signaled,
    TimedOut,
    SpawnFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessEventRecord {
    pub event_id: String,
    pub role: String,
    pub pid: Option<u32>,
    pub command: Vec<String>,
    pub exit_classification: ProcessExitClassification,
    pub exit_code: Option<i32>,
    pub signal: Option<String>,
    pub stdout_excerpt: String,
    pub stderr_excerpt: String,
    pub started_at_ms: u128,
    pub ended_at_ms: u128,
}

#[derive(Debug, Clone)]
pub struct ProcessRunOutcome {
    pub classification: ProcessExitClassification,
    pub exit_status: Option<ExitStatus>,
    pub stdout: String,
    pub stderr: String,
    pub pid: u32,
}

pub struct ProcessHarness {
    worker_bin: PathBuf,
}

pub struct HeldWorker {
    child: Child,
    pub pid: u32,
    pub ready_stdout: String,
}

impl ProcessHarness {
    pub fn discover_worker_bin() -> PathBuf {
        if let Ok(path) = std::env::var("VOXPROOF_EVIDENCE_WORKER_BIN") {
            return PathBuf::from(path);
        }
        let current = std::env::current_exe().expect("current exe");
        let dir = current.parent().expect("parent dir");
        for candidate in [
            dir.join("persistence_evidence_worker"),
            dir.join("../persistence_evidence_worker"),
        ] {
            if candidate.exists() {
                return candidate;
            }
        }
        dir.join("persistence_evidence_worker")
    }

    pub fn new(worker_bin: PathBuf) -> Self {
        Self { worker_bin }
    }

    pub fn spawn_waiting_ready(
        &self,
        command: &str,
        env: &[(&str, &str)],
        timeout: Duration,
    ) -> Result<HeldWorker, String> {
        let mut cmd = Command::new(&self.worker_bin);
        cmd.arg(command)
            .envs(env.iter().copied())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let started = Instant::now();
        let mut child = cmd
            .spawn()
            .map_err(|error| format!("spawn failed: {error}"))?;
        let pid = child.id();
        let stdout = child.stdout.take().expect("stdout");
        let stderr = child.stderr.take();
        let mut stdout_reader = BufReader::new(stdout);
        let mut ready_line = String::new();
        while started.elapsed() < timeout {
            ready_line.clear();
            if stdout_reader.read_line(&mut ready_line).is_err() {
                break;
            }
            if ready_line.trim() == READY_LINE {
                return Ok(HeldWorker {
                    child,
                    pid,
                    ready_stdout: ready_line,
                });
            }
        }
        let _ = child.kill();
        let _ = child.wait();
        let _ = stderr;
        Err("worker ready timeout".to_string())
    }

    pub fn kill_held_worker(&self, mut held: HeldWorker, _started: Instant) -> ProcessRunOutcome {
        let _ = held.child.kill();
        let status = held.child.wait().ok();
        ProcessRunOutcome {
            classification: ProcessExitClassification::Signaled,
            exit_status: status,
            stdout: held.ready_stdout,
            stderr: String::new(),
            pid: held.pid,
        }
    }

    pub fn from_current_exe() -> Self {
        Self::new(Self::discover_worker_bin())
    }

    pub fn worker_path(&self) -> &Path {
        &self.worker_bin
    }

    pub fn spawn_worker(
        &self,
        command: &str,
        env: &[(&str, &str)],
        timeout: Duration,
    ) -> Result<ProcessRunOutcome, String> {
        let mut cmd = Command::new(&self.worker_bin);
        cmd.arg(command)
            .envs(env.iter().copied())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let started = Instant::now();
        let mut child = cmd
            .spawn()
            .map_err(|error| format!("spawn failed: {error}"))?;
        let pid = child.id();
        let stdout = child.stdout.take().expect("stdout");
        let stderr = child.stderr.take().expect("stderr");
        let mut stdout_reader = BufReader::new(stdout);
        let mut ready_line = String::new();
        let ready_deadline = started + timeout;
        loop {
            if Instant::now() > ready_deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(ProcessRunOutcome {
                    classification: ProcessExitClassification::TimedOut,
                    exit_status: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    pid,
                });
            }
            ready_line.clear();
            stdout_reader
                .read_line(&mut ready_line)
                .map_err(|error| format!("read ready failed: {error}"))?;
            if ready_line.trim() == READY_LINE {
                break;
            }
            if ready_line.starts_with(RESULT_PREFIX) {
                break;
            }
        }

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(b"proceed\n");
        }

        let wait_deadline = started + timeout;
        loop {
            if let Ok(Some(status)) = child.try_wait() {
                let mut stderr_reader = BufReader::new(stderr);
                let mut stderr_buf = String::new();
                let _ = stderr_reader.read_to_string(&mut stderr_buf);
                let mut stdout_rest = String::new();
                let _ = stdout_reader.read_to_string(&mut stdout_rest);
                let full_stdout = format!("{ready_line}{stdout_rest}");
                return Ok(classify_exit(status, pid, full_stdout, stderr_buf));
            }
            if Instant::now() > wait_deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(ProcessRunOutcome {
                    classification: ProcessExitClassification::TimedOut,
                    exit_status: None,
                    stdout: ready_line,
                    stderr: String::new(),
                    pid,
                });
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn spawn_and_hold_until_killed(
        &self,
        command: &str,
        env: &[(&str, &str)],
        hold_timeout: Duration,
    ) -> Result<ProcessRunOutcome, String> {
        let mut cmd = Command::new(&self.worker_bin);
        cmd.arg(command)
            .envs(env.iter().copied())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let started = Instant::now();
        let mut child = cmd
            .spawn()
            .map_err(|error| format!("spawn failed: {error}"))?;
        let pid = child.id();
        let stdout = child.stdout.take().expect("stdout");
        let stderr = child.stderr.take().expect("stderr");
        let mut stdout_reader = BufReader::new(stdout);
        let mut ready_line = String::new();
        let ready_deadline = started + Duration::from_secs(30);
        loop {
            if Instant::now() > ready_deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(ProcessRunOutcome {
                    classification: ProcessExitClassification::TimedOut,
                    exit_status: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    pid,
                });
            }
            ready_line.clear();
            stdout_reader
                .read_line(&mut ready_line)
                .map_err(|error| format!("read ready failed: {error}"))?;
            if ready_line.trim() == READY_LINE {
                break;
            }
        }

        std::thread::sleep(hold_timeout);
        let _ = child.kill();
        let status = child
            .wait()
            .map_err(|error| format!("wait failed: {error}"))?;
        let mut stderr_reader = BufReader::new(stderr);
        let mut stderr_buf = String::new();
        let _ = stderr_reader.read_to_string(&mut stderr_buf);
        Ok(classify_exit(status, pid, ready_line, stderr_buf))
    }
}

pub fn exit_signal_name(status: Option<&ExitStatus>) -> Option<String> {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        status.and_then(|status| {
            status.signal().map(|signal| match signal {
                6 => "SIGABRT".to_string(),
                9 => "SIGKILL".to_string(),
                other => format!("SIG{other}"),
            })
        })
    }
    #[cfg(not(unix))]
    {
        let _ = status;
        None
    }
}

fn classify_exit(
    status: ExitStatus,
    pid: u32,
    stdout: String,
    stderr: String,
) -> ProcessRunOutcome {
    let classification = if status.success() {
        ProcessExitClassification::Success
    } else if status.code().is_some() {
        ProcessExitClassification::AbnormalTermination
    } else {
        ProcessExitClassification::Signaled
    };
    ProcessRunOutcome {
        classification,
        exit_status: Some(status),
        stdout,
        stderr,
        pid,
    }
}

pub fn excerpt(text: &str, max: usize) -> String {
    if text.len() <= max {
        text.to_string()
    } else {
        format!("{}…", &text[..max])
    }
}
