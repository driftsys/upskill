//! Opt-in execution requests; command activation remains separate.
use crate::files::plan::PlannedFile;
mod request;
impl ExecutionRequest {
    /// Validate supplied paths, topology and permission values without filesystem effects.
    /// This does not qualify observations or establish ownership.
    pub fn validate(&self) -> anyhow::Result<()> {
        request::validate(self)
    }
}
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod backend;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod journal;
#[cfg(target_os = "macos")]
mod macos_acl;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod run;

#[derive(Debug, Clone)]
pub struct ExecutionRequest {
    pub files: Vec<PlannedFile>,
    pub state: PlannedFile,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionReport {
    pub recorded_prefix: usize,
    pub verified_post_prefix: usize,
    pub outcome: ExecutionOutcome,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionOutcome {
    Complete,
    Reconcile { index: usize },
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
type CheckpointHook = Box<dyn FnMut(&'static str, usize) -> anyhow::Result<()>>;
#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
thread_local! {
    static TEST_HOOK: std::cell::RefCell<Option<CheckpointHook>> = std::cell::RefCell::new(None);
}
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn checkpoint(label: &'static str, index: usize) -> anyhow::Result<()> {
    #[cfg(test)]
    {
        TEST_HOOK.with(|hook| match hook.borrow_mut().as_mut() {
            Some(f) => f(label, index),
            None => Ok(()),
        })
    }
    #[cfg(not(test))]
    {
        let _ = (label, index);
        Ok(())
    }
}
#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod process_tests;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
impl crate::filesystem::QualifiedScope {
    /// File execution is explicitly unsupported on this operating system.
    pub fn apply_operation(
        &self,
        guard: &crate::filesystem::ScopeWriterGuard<'_>,
        sources: &[&crate::filesystem::SourceSnapshot<'_>],
        request: ExecutionRequest,
    ) -> anyhow::Result<ExecutionReport> {
        let _ = (guard, sources, request);
        anyhow::bail!("qualified file execution is unsupported on this operating system")
    }
    /// File recovery is explicitly unsupported on this operating system.
    pub fn recover_operation(
        &self,
        guard: &crate::filesystem::ScopeWriterGuard<'_>,
    ) -> anyhow::Result<Option<ExecutionReport>> {
        let _ = guard;
        anyhow::bail!("qualified file recovery is unsupported on this operating system")
    }
}
