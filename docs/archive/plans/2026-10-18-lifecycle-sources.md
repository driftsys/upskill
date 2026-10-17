# Gate 5 selected rule sources

Scope: implement the accepted Gate 5 applying source prerequisite. Production
commands and schema 1 stay unchanged; Codex remains gated on Gate 6.

- [x] Write source capture acceptance tests and observe RED.
- [x] Resolve one explicit local or Git item root, retaining qualified scope and
      temporary checkout ownership.
- [x] Bind private strict capture to source metadata; compile complete desired
      artifacts and matching owner contracts through the existing compiler.
- [x] Test aliases, malformed and missing sources, final symlinks, source edits,
      branches, tags, full commit SHA, containment, and checkout lifetime.
- [x] Run formatting, full verification, and explicit MSRV checks.
- [ ] Root review and delivery.

Initial RED: `cargo test --test lifecycle_sources` failed with E0432 because
`lifecycle::sources` did not exist. The focused suite then passed. A second RED
showed repository-root capture included 26 files rather than one because `.git`
metadata lived inside the selected tree. Git metadata now lives beside the work
tree in the private temporary directory; the root-capture test passed afterward.

Decisions: explicit selected roots only; no registry discovery or default
selection. Local paths resolve their ancestors while retaining final symlink
rejection. Git uses synchronous init/fetch/checkout with separate metadata,
structured arguments, and git-native credentials. Branches, tags and full SHAs
record `HEAD^{commit}`. Capture privately borrows its resolver and exposes a
borrowed source snapshot for executor revalidation. Revalidation samples strict
source evidence and checkout revision; it does not freeze external writers. Git
tests use controlled local repositories and child processes with private HOME
and Git configuration rather than process-wide environment mutation.

Bootstrap initially failed resolving api.github.com in the sandbox; required
setup succeeded with escalated permissions. No commit or delivery action belongs
to this worker.

Validation: eight source integration acceptance tests and two private source
unit tests passed. `just fmt`, escalated `just verify`, and
`RUSTC_WRAPPER= just msrv-check` passed using the worktree-local `target/`. The
initial sandboxed full verify had seven existing FIFO/process tests rejected
with EPERM; all passed with required filesystem permissions. Standalone rendered
documentation checks passed after keeping the unpublished validation note linked
through its GitHub source URL. The revision-drift unit test was added during the
full verify run and then explicitly executed green; production source stayed
unchanged during that run. Logs: `/private/tmp/lifecycle-sources-focused.log`,
`/private/tmp/lifecycle-sources-unit.log`,
`/private/tmp/lifecycle-sources-verify-escalated.log`, and
`/private/tmp/lifecycle-sources-msrv.log`.

Review correction: inherited `GIT_INDEX_FILE` could redirect checkout index
writes outside the private temporary directory. A process-isolated acceptance
reproduced this with an absent external index becoming a Git index (RED). Source
subprocesses now clear repository-local overrides listed by
`git rev-parse --local-env-vars`, including index/object/common-directory and
repository-scoped config variables. A unit test checks coverage against
installed Git. HOME, global/system config, credential helpers, SSH and askpass
remain native. The acceptance checks absent and existing external indexes,
immutable commit resolution, and retained global `insteadOf` configuration
without allowing any network protocol. Nine source integration tests, three
private unit tests, and all-target Clippy passed after the correction. The final
configuration-preservation variant also passed.
