//! Inactive synchronous native subprocess support, separate from effect receipts.
//!
//! No production profile or CLI route uses this module. Callers must publish the
//! operation's durable Pending checkpoint before invoking an admitted adapter.
//! Group emptiness does not prove absence of escaped or previously orphaned work.
use crate::filesystem::{QualifiedScope, ScopeWriterGuard, native_effects};
use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};
use rustix::process::{
    Pid, Signal, WaitId, WaitIdOptions, kill_process_group, test_kill_process_group, waitid,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt;
use std::fs::{File, Metadata};
use std::io::{ErrorKind, Read};
use std::os::fd::AsFd;
use std::os::unix::{ffi::OsStrExt, fs::MetadataExt, process::CommandExt};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const MAX_OUTPUT: usize = 1024 * 1024;
const MAX_EXECUTABLE: u64 = 1024 * 1024 * 1024;
const POLL: Duration = Duration::from_millis(5);
const DRAIN: Duration = Duration::from_millis(150);

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NativeProcessError {
    #[error("native executable is unavailable or is not an ordinary executable file")]
    Unavailable,
    #[error("native executable identity changed; recapture the exact profile before retrying")]
    ExecutableChanged,
    #[error(
        "native invocation requires an absolute launcher, valid bounded limits and explicit valid arguments/environment"
    )]
    InvalidInvocation,
    #[error(
        "native invocation requires an unchanged qualified scope and its matching writer guard"
    )]
    UnsafeScope,
    #[error("native invocation was cancelled before launch")]
    CancelledBeforeLaunch,
    #[error("native subprocess could not be started; no successful installation is established")]
    SpawnFailed,
    #[error("native process {0} is unresolved (OS code {1:?}); recover without blind replay")]
    ProcessControl(&'static str, Option<i32>),
}

#[derive(Clone, PartialEq, Eq)]
struct Identity {
    device: u64,
    inode: u64,
    length: u64,
    mode: u32,
    mtime: (i64, i64),
    ctime: (i64, i64),
}
impl From<&Metadata> for Identity {
    fn from(value: &Metadata) -> Self {
        Self {
            device: value.dev(),
            inode: value.ino(),
            length: value.len(),
            mode: value.mode(),
            mtime: (value.mtime(), value.mtime_nsec()),
            ctime: (value.ctime(), value.ctime_nsec()),
        }
    }
}
/// A read-only launcher capture, not an executable/environment qualification.
/// Dependency/interpreter files and the final validation-to-exec race remain
/// outside this capture. No PATH lookup is performed.
#[derive(Clone)]
pub struct NativeExecutable {
    selected: PathBuf,
    resolved: PathBuf,
    identity: Identity,
    digest: [u8; 32],
}
impl NativeExecutable {
    pub fn capture(path: &Path) -> Result<Self, NativeProcessError> {
        if !path.is_absolute() || path.components().any(|p| p == Component::ParentDir) {
            return Err(NativeProcessError::InvalidInvocation);
        }
        let resolved = path
            .canonicalize()
            .map_err(|_| NativeProcessError::Unavailable)?;
        // NONBLOCK also prevents a raced FIFO/device replacement from hanging
        // before its descriptor type has been checked. NOFOLLOW rejects a new
        // final symlink after canonical selection.
        let mut file = File::from(
            rustix::fs::open(
                &resolved,
                OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NONBLOCK | OFlags::NOFOLLOW,
                rustix::fs::Mode::empty(),
            )
            .map_err(|_| NativeProcessError::Unavailable)?,
        );
        let meta = file
            .metadata()
            .map_err(|_| NativeProcessError::Unavailable)?;
        if !meta.is_file()
            || meta.mode() & 0o111 == 0
            || meta.mode() & 0o6000 != 0
            || meta.len() > MAX_EXECUTABLE
        {
            return Err(NativeProcessError::Unavailable);
        }
        let identity = Identity::from(&meta);
        let mut digest = Sha256::new();
        let mut buffer = [0u8; 32768];
        let mut total = 0u64;
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|_| NativeProcessError::Unavailable)?;
            if read == 0 {
                break;
            }
            total += read as u64;
            if total > MAX_EXECUTABLE {
                return Err(NativeProcessError::ExecutableChanged);
            }
            digest.update(&buffer[..read]);
        }
        if path.canonicalize().ok().as_ref() != Some(&resolved)
            || file.metadata().ok().as_ref().map(Identity::from) != Some(identity.clone())
            || std::fs::metadata(&resolved)
                .ok()
                .as_ref()
                .map(Identity::from)
                != Some(identity.clone())
        {
            return Err(NativeProcessError::ExecutableChanged);
        }
        Ok(Self {
            selected: path.into(),
            resolved,
            identity,
            digest: digest.finalize().into(),
        })
    }
    /// Non-secret code identity for private diagnostic provenance only.
    /// This is not an executable, dependency or environment qualification.
    pub(crate) fn code_identity(&self) -> serde_json::Value {
        let sha256: String = self
            .digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        serde_json::json!({"selected":self.selected,"resolved":self.resolved,
            "sha256":sha256,"bytes":self.identity.length,"mode":self.identity.mode & 0o7777})
    }
    fn unchanged(&self) -> bool {
        Self::capture(&self.selected).is_ok_and(|now| {
            now.resolved == self.resolved
                && now.identity == self.identity
                && now.digest == self.digest
        })
    }
}
/// Raw arguments/environment are deliberately excluded from Debug and serde.
/// The caller owns the complete environment contract; no parent values inherit.
pub struct NativeCommandSpec {
    pub executable: NativeExecutable,
    pub arguments: Vec<OsString>,
    pub environment: BTreeMap<OsString, OsString>,
    pub timeout: Duration,
    /// Maximum combined stdout and stderr retained in memory (1..=1 MiB).
    pub output_limit: usize,
}
impl fmt::Debug for NativeCommandSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeCommandSpec")
            .field("argument_count", &self.arguments.len())
            .field("environment_count", &self.environment.len())
            .field("timeout", &self.timeout)
            .field("output_limit", &self.output_limit)
            .finish_non_exhaustive()
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeStopReason {
    Exited,
    TimedOut,
    Cancelled,
    OutputLimit,
    OutputReadFailed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeGroupState {
    Empty,
    Unresolved,
}
/// Raw output may contain credentials. It must be interpreted in memory by the
/// separately qualified adapter and must never be copied into durable evidence.
/// Exit code/group state alone do not establish a native binding or retention.
pub struct NativeProcessOutput {
    pub reason: NativeStopReason,
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub group: NativeGroupState,
    pub direct_child_reaped: bool,
    pub cleanup_signal_sent: bool,
    pub output_complete: bool,
    pub scope_unchanged: bool,
    pub executable_unchanged: bool,
}
impl fmt::Debug for NativeProcessOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeProcessOutput")
            .field("reason", &self.reason)
            .field("exit_code", &self.exit_code)
            .field("stdout_bytes", &self.stdout.len())
            .field("stderr_bytes", &self.stderr.len())
            .field("group", &self.group)
            .field("output_complete", &self.output_complete)
            .field("scope_unchanged", &self.scope_unchanged)
            .field("executable_unchanged", &self.executable_unchanged)
            .finish_non_exhaustive()
    }
}

struct GroupChild {
    child: Child,
    pid: Pid,
    reaped: bool,
}
impl GroupChild {
    fn exited(&mut self) -> Result<bool, NativeProcessError> {
        match waitid(
            WaitId::Pid(self.pid),
            WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
        ) {
            Ok(state) => Ok(state.is_some()),
            Err(error) => {
                if error == rustix::io::Errno::CHILD {
                    // Another reaper has consumed the leader; never signal a
                    // numeric PID/group whose ownership we can no longer hold.
                    self.reaped = true;
                }
                Err(NativeProcessError::ProcessControl(
                    "observation",
                    Some(error.raw_os_error()),
                ))
            }
        }
    }
    fn finish(&mut self) -> Result<(ExitStatus, NativeGroupState, bool), NativeProcessError> {
        self.finish_with(
            |pid| kill_process_group(pid, Signal::KILL),
            Child::wait,
            test_kill_process_group,
        )
    }
    fn finish_with(
        &mut self,
        mut signal: impl FnMut(Pid) -> rustix::io::Result<()>,
        mut reap: impl FnMut(&mut Child) -> std::io::Result<ExitStatus>,
        mut probe: impl FnMut(Pid) -> rustix::io::Result<()>,
    ) -> Result<(ExitStatus, NativeGroupState, bool), NativeProcessError> {
        // The unreaped leader reserves this PID/group identity during signalling.
        // Never signal this group after reaping: its numeric identity can be reused.
        let signalled = signal(self.pid);
        let status = reap(&mut self.child);
        self.reaped = true;
        let status = status
            .map_err(|error| NativeProcessError::ProcessControl("reaping", error.raw_os_error()))?;
        // macOS may deny signalling a zombie-only group. After reaping, an
        // independent absence probe can still establish this group's emptiness.
        // Retain the signal result separately; never call it native completion.
        let deadline = Instant::now() + DRAIN;
        loop {
            if probe(self.pid) == Err(rustix::io::Errno::SRCH) {
                return Ok((status, NativeGroupState::Empty, signalled.is_ok()));
            }
            if Instant::now() >= deadline {
                return Ok((status, NativeGroupState::Unresolved, signalled.is_ok()));
            }
            std::thread::sleep(POLL);
        }
    }
}
impl Drop for GroupChild {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = kill_process_group(self.pid, Signal::KILL);
            let _ = self.child.wait();
        }
    }
}
fn nonblocking(pipe: &impl AsFd) -> Result<(), NativeProcessError> {
    let flags = fcntl_getfl(pipe).map_err(|error| {
        NativeProcessError::ProcessControl("pipe flags", Some(error.raw_os_error()))
    })?;
    fcntl_setfl(pipe, flags | OFlags::NONBLOCK).map_err(|error| {
        NativeProcessError::ProcessControl("pipe flags", Some(error.raw_os_error()))
    })
}
struct Pipe<R> {
    reader: R,
    bytes: Vec<u8>,
    eof: bool,
}
impl<R: Read> Pipe<R> {
    fn drain(&mut self, remaining: &mut usize) -> Result<bool, NativeProcessError> {
        let mut buffer = [0u8; 4096];
        // A producing process must not prevent the outer deadline/cancel poll.
        for _ in 0..8 {
            match self.reader.read(&mut buffer) {
                Ok(0) => {
                    self.eof = true;
                    return Ok(false);
                }
                Ok(read) => {
                    let keep = read.min(*remaining);
                    self.bytes.extend_from_slice(&buffer[..keep]);
                    *remaining -= keep;
                    if read > keep {
                        return Ok(true);
                    }
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => return Ok(false),
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(error) => {
                    return Err(NativeProcessError::ProcessControl(
                        "output read",
                        error.raw_os_error(),
                    ));
                }
            }
        }
        Ok(false)
    }
}
fn poll_output<R: Read, S: Read>(
    out: &mut Pipe<R>,
    err: &mut Pipe<S>,
    remaining: &mut usize,
) -> Option<NativeStopReason> {
    match (out.drain(remaining), err.drain(remaining)) {
        (Err(_), _) | (_, Err(_)) => Some(NativeStopReason::OutputReadFailed),
        (Ok(true), _) | (_, Ok(true)) => Some(NativeStopReason::OutputLimit),
        _ => None,
    }
}
fn validate(spec: &NativeCommandSpec) -> Result<(), NativeProcessError> {
    if spec.timeout.is_zero()
        || spec.timeout > Duration::from_secs(300)
        || spec.output_limit == 0
        || spec.output_limit > MAX_OUTPUT
        || spec.arguments.iter().any(|v| v.as_bytes().contains(&0))
        || spec.environment.iter().any(|(key, value)| {
            key.is_empty()
                || key.as_bytes().contains(&0)
                || key.as_bytes().contains(&b'=')
                || value.as_bytes().contains(&0)
        })
    {
        return Err(NativeProcessError::InvalidInvocation);
    }
    Ok(())
}
/// Execute one caller-authorized native command. This helper supplies neither
/// profile admission nor durable invocation intent; production routing stays closed.
/// Deadline bounds the polling loop. OS filesystem operations and final reaping
/// are not hard-real-time bounded. Group state covers only this live invocation;
/// parent-death/escaped-process recovery requires a separately qualified witness.
pub fn run_native_process(
    scope: &QualifiedScope,
    guard: &ScopeWriterGuard<'_>,
    spec: &NativeCommandSpec,
    cancellation: &AtomicBool,
) -> Result<NativeProcessOutput, NativeProcessError> {
    run_with_clock(
        scope,
        guard,
        spec,
        cancellation,
        |start| start.elapsed(),
        || Ok(()),
    )
}

fn run_with_clock(
    scope: &QualifiedScope,
    guard: &ScopeWriterGuard<'_>,
    spec: &NativeCommandSpec,
    cancellation: &AtomicBool,
    elapsed: impl Fn(Instant) -> Duration,
    before_launch: impl FnOnce() -> Result<(), NativeProcessError>,
) -> Result<NativeProcessOutput, NativeProcessError> {
    validate(spec)?;
    let directory = native_effects::command_directory(scope, guard)
        .map_err(|_| NativeProcessError::UnsafeScope)?;
    if cancellation.load(Ordering::SeqCst) {
        return Err(NativeProcessError::CancelledBeforeLaunch);
    }
    if !spec.executable.unchanged() {
        return Err(NativeProcessError::ExecutableChanged);
    }
    let mut command = Command::new(&spec.executable.selected);
    command
        .args(&spec.arguments)
        .env_clear()
        .envs(&spec.environment)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    // SAFETY: fchdir is async-signal-safe, the descriptor is already open, and
    // converting its OS errno does not allocate or acquire user-space locks.
    unsafe {
        command.pre_exec(move || rustix::process::fchdir(&directory).map_err(std::io::Error::from));
    }
    guard
        .verify_binding()
        .map_err(|_| NativeProcessError::UnsafeScope)?;
    if cancellation.load(Ordering::SeqCst) {
        return Err(NativeProcessError::CancelledBeforeLaunch);
    }
    before_launch()?;
    let child = command
        .spawn()
        .map_err(|_| NativeProcessError::SpawnFailed)?;
    let pid = Pid::from_child(&child);
    let mut child = GroupChild {
        child,
        pid,
        reaped: false,
    };
    let stdout = child
        .child
        .stdout
        .take()
        .ok_or(NativeProcessError::ProcessControl("pipe acquisition", None))?;
    let stderr = child
        .child
        .stderr
        .take()
        .ok_or(NativeProcessError::ProcessControl("pipe acquisition", None))?;
    nonblocking(&stdout)?;
    nonblocking(&stderr)?;
    let mut out = Pipe {
        reader: stdout,
        bytes: vec![],
        eof: false,
    };
    let mut err = Pipe {
        reader: stderr,
        bytes: vec![],
        eof: false,
    };
    let mut remaining = spec.output_limit;
    let start = Instant::now();
    let mut capture_valid = true;
    let reason = loop {
        if let Some(reason) = poll_output(&mut out, &mut err, &mut remaining) {
            capture_valid = false;
            break reason;
        }
        if cancellation.load(Ordering::SeqCst) {
            break NativeStopReason::Cancelled;
        }
        if child.exited()? {
            break NativeStopReason::Exited;
        }
        if elapsed(start) >= spec.timeout {
            break NativeStopReason::TimedOut;
        }
        std::thread::sleep(POLL);
    };
    let (status, group, cleanup_signal_sent) = child.finish()?;
    let drain_until = Instant::now() + DRAIN;
    let mut reason = reason;
    while !(out.eof && err.eof) && Instant::now() < drain_until {
        if let Some(observed) = poll_output(&mut out, &mut err, &mut remaining) {
            capture_valid = false;
            if reason == NativeStopReason::Exited {
                reason = observed;
            }
            break;
        }
        if !(out.eof && err.eof) {
            std::thread::sleep(POLL);
        }
    }
    Ok(NativeProcessOutput {
        reason,
        exit_code: status.code(),
        stdout: out.bytes,
        stderr: err.bytes,
        group,
        direct_child_reaped: true,
        cleanup_signal_sent,
        output_complete: capture_valid && out.eof && err.eof,
        scope_unchanged: guard.verify_binding().is_ok(),
        executable_unchanged: spec.executable.unchanged(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_delivery::test_support::process_fixture::{
        descendants, stopped_heartbeat, with_scope,
    };
    #[test]
    fn timeout_after_parent_and_descendant_readiness_stops_the_group_and_reaps_its_leader() {
        with_scope(|root, scope, guard| {
            let spec = descendants(root, false);
            let watchdog = Instant::now();
            let output = run_with_clock(
                scope,
                guard,
                &spec,
                &AtomicBool::new(false),
                |_| {
                    assert!(
                        watchdog.elapsed() < Duration::from_secs(5),
                        "fake-native positive did not become ready"
                    );
                    if root.join("parent-ready").exists()
                        && std::fs::metadata(root.join("heartbeat")).is_ok_and(|m| m.len() > 0)
                    {
                        spec.timeout
                    } else {
                        Duration::ZERO
                    }
                },
                || Ok(()),
            )
            .unwrap();
            assert_eq!(output.reason, NativeStopReason::TimedOut);
            assert!(output.stdout.ends_with(b"spawned\n"));
            stopped_heartbeat(root);
            let pid = std::fs::read_to_string(root.join("leader-pid"))
                .unwrap()
                .parse()
                .unwrap();
            let pid = Pid::from_raw(pid).unwrap();
            assert!(matches!(
                rustix::process::waitpid(Some(pid), rustix::process::WaitOptions::NOHANG),
                Err(rustix::io::Errno::CHILD)
            ));
            assert!(output.direct_child_reaped);
        });
    }
    fn leader_reaped(root: &Path) {
        let pid = std::fs::read_to_string(root.join("leader-pid"))
            .unwrap()
            .parse()
            .unwrap();
        assert!(matches!(
            waitid(
                WaitId::Pid(Pid::from_raw(pid).unwrap()),
                WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT
            ),
            Err(rustix::io::Errno::CHILD)
        ));
    }
    #[test]
    fn overflow_after_descendant_readiness_stops_the_group_and_reaps_the_leader() {
        with_scope(|root, scope, guard| {
            let mut spec = descendants(root, false);
            let launcher = root.join("native-fixture");
            let original = std::fs::read_to_string(&launcher).unwrap();
            assert_eq!(original.matches("\nwait\n").count(), 1);
            std::fs::write(
                &launcher,
                original.replace(
                    "\nwait\n",
                    r#"
while [ ! -f "$OVERFLOW_TRIGGER" ]; do /bin/sleep 0.01; done
while :; do printf 'overflow-output'; done
"#,
                ),
            )
            .unwrap();
            spec.executable = NativeExecutable::capture(&launcher).unwrap();
            spec.environment.insert(
                "OVERFLOW_TRIGGER".into(),
                root.join("overflow").into_os_string(),
            );
            spec.output_limit = 128;
            let watchdog = Instant::now();
            let output = run_with_clock(
                scope,
                guard,
                &spec,
                &AtomicBool::new(false),
                |_| {
                    assert!(
                        watchdog.elapsed() < Duration::from_secs(5),
                        "overflow positive control did not complete"
                    );
                    if root.join("parent-ready").exists()
                        && std::fs::metadata(root.join("heartbeat")).is_ok_and(|m| m.len() > 0)
                    {
                        std::fs::write(root.join("overflow"), b"go").unwrap();
                    }
                    Duration::ZERO
                },
                || Ok(()),
            )
            .unwrap();
            assert_eq!(output.reason, NativeStopReason::OutputLimit);
            assert!(output.stdout.len() + output.stderr.len() <= 128);
            assert!(!output.output_complete);
            stopped_heartbeat(root);
            leader_reaped(root);
        });
    }
    #[test]
    fn signal_result_group_observation_and_signal_before_reap_are_independent() {
        use std::cell::RefCell;
        for signal_succeeds in [false, true] {
            for group_absent in [false, true] {
                with_scope(|root, _, _| {
                    let spec = crate::native_delivery::test_support::process_fixture::script(
                        root,
                        if signal_succeeds {
                            "while :; do :; done"
                        } else {
                            "exit 0"
                        },
                    );
                    let child = Command::new(&spec.executable.selected)
                        .env_clear()
                        .envs(&spec.environment)
                        .current_dir(root)
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .process_group(0)
                        .spawn()
                        .unwrap();
                    let mut child = GroupChild {
                        pid: Pid::from_child(&child),
                        child,
                        reaped: false,
                    };
                    if !signal_succeeds {
                        let watchdog = Instant::now();
                        while !child.exited().unwrap() {
                            assert!(watchdog.elapsed() < Duration::from_secs(5));
                            std::thread::sleep(POLL);
                        }
                    }
                    let events = RefCell::new(Vec::new());
                    let (_, group, sent) = child
                        .finish_with(
                            |pid| {
                                assert!(
                                    events.borrow().is_empty(),
                                    "signal occurred after reap/probe"
                                );
                                assert!(
                                    !matches!(
                                        waitid(
                                            WaitId::Pid(pid),
                                            WaitIdOptions::EXITED
                                                | WaitIdOptions::NOHANG
                                                | WaitIdOptions::NOWAIT
                                        ),
                                        Err(rustix::io::Errno::CHILD)
                                    ),
                                    "leader was already reaped before signal"
                                );
                                events.borrow_mut().push("signal");
                                if signal_succeeds {
                                    kill_process_group(pid, Signal::KILL)
                                } else {
                                    Err(rustix::io::Errno::PERM)
                                }
                            },
                            |child| {
                                assert_eq!(*events.borrow(), ["signal"]);
                                let status = child.wait();
                                events.borrow_mut().push("reap");
                                status
                            },
                            |_| {
                                assert!(events.borrow().starts_with(&["signal", "reap"]));
                                events.borrow_mut().push("probe");
                                if group_absent {
                                    Err(rustix::io::Errno::SRCH)
                                } else {
                                    Ok(())
                                }
                            },
                        )
                        .unwrap();
                    assert_eq!(sent, signal_succeeds);
                    assert_eq!(
                        group,
                        if group_absent {
                            NativeGroupState::Empty
                        } else {
                            NativeGroupState::Unresolved
                        }
                    );
                    assert_eq!(&events.borrow()[..3], &["signal", "reap", "probe"]);
                    assert!(events.borrow()[3..].iter().all(|event| *event == "probe"));
                });
            }
        }
    }
    #[test]
    fn output_read_failure_is_explicit_on_either_stream() {
        struct Fault;
        impl Read for Fault {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("private-read-error"))
            }
        }
        let mut fault = Pipe {
            reader: Fault,
            bytes: vec![],
            eof: false,
        };
        let mut empty = Pipe {
            reader: &b""[..],
            bytes: vec![],
            eof: false,
        };
        assert_eq!(
            poll_output(&mut fault, &mut empty, &mut 100),
            Some(NativeStopReason::OutputReadFailed)
        );
        assert_eq!(
            poll_output(&mut empty, &mut fault, &mut 100),
            Some(NativeStopReason::OutputReadFailed)
        );
        let error = fault.drain(&mut 100).unwrap_err();
        assert_eq!(
            error,
            NativeProcessError::ProcessControl("output read", None)
        );
        assert!(!format!("{error:?}").contains("private-read-error"));
    }
    #[test]
    fn launch_namespace_replacement_keeps_effects_on_the_pinned_directory() {
        use crate::native_delivery::test_support::process_fixture::script;
        with_scope(|root, scope, guard| {
            let launcher_root = tempfile::tempdir().unwrap();
            let spec = script(launcher_root.path(), "printf pinned > marker; pwd -P");
            let moved = root.with_file_name("moved");
            let attempt = run_with_clock(
                scope,
                guard,
                &spec,
                &AtomicBool::new(false),
                |start| start.elapsed(),
                || {
                    std::fs::rename(root, &moved).unwrap();
                    std::fs::create_dir(root).unwrap();
                    std::fs::write(root.join("manual"), b"replacement-sentinel").unwrap();
                    Ok(())
                },
            );
            let original_marker = std::fs::read(moved.join("marker"));
            let replacement_marker = root.join("marker").exists();
            let manual = std::fs::read(root.join("manual"));
            std::fs::remove_dir_all(root).unwrap();
            std::fs::rename(&moved, root).unwrap();
            let output = attempt.unwrap();
            assert_eq!(original_marker.unwrap(), b"pinned");
            assert!(!replacement_marker);
            assert_eq!(manual.unwrap(), b"replacement-sentinel");
            assert!(!output.scope_unchanged);
            assert!(output.executable_unchanged);
            assert_eq!(
                output.stdout,
                format!(
                    "{}\n",
                    root.parent()
                        .unwrap()
                        .canonicalize()
                        .unwrap()
                        .join("moved")
                        .display()
                )
                .as_bytes()
            );
        });
    }
}
