# Gate 5 selected rule source validation

The applying-only `lifecycle::sources` boundary resolves one explicitly selected
rule item root. It retains a qualified source scope and private Git checkout
until the resolver is dropped. Captured source bytes are privately associated
with their resolved metadata, and compilation produces both complete desired
artifacts and matching owner contracts through the existing compiler.

Acceptance coverage in `tests/lifecycle_sources.rs` includes exact captured
bytes, aliases, executable resource modes, malformed and unavailable sources,
final symlink rejection, post-capture edits, explicit expected-name/profile
mismatches, Codex rejection, temporary checkout retention, and Git
branch/tag/full SHA resolution to the actual commit. Git acceptance tests run in
child processes with private HOME and Git configuration and controlled local
repositories. They perform no network authentication or process-wide environment
mutation.

The initial acceptance test failed because the API was absent. A repository-root
capture test then exposed Git metadata in the captured tree (26 files instead of
one). Moving metadata beside the work tree fixed the test without detaching the
selected source bytes from their qualified scope. A private unit test checks
temporary checkout cleanup on resolver drop.

This evidence establishes sampled source revalidation and captured compilation,
not native target acceptance, destination application or exclusion against
external writers. Registry discovery, command selection and Codex qualification
remain later orchestration. Production commands and schema 1 are unchanged.

Compiler permissions remain evidence rather than application policy: rendered
rule entrypoints and contracts contain `mode: None`; resources and contracts
retain raw captured permission bits. The guarded command adapter must call
`files::permissions::select_file_mode` with qualified destination mode evidence,
`PermissionSource::Generated` or `PermissionSource::Resource`, and an explicit
`ResourceExecutionPolicy`. It must keep desired artifacts and owner contracts
consistent with those selected modes before planning/application. This boundary
does not select executor-private modes.

Validation completed on macOS with the worktree-local build directory:
`just fmt`, the full escalated `just verify`, and explicit Rust 1.85
`just msrv-check` passed. Eight integration examples and both private unit tests
passed. The initial sandboxed full run rejected seven existing FIFO/process
examples with EPERM; the required-permission run passed those examples. The
revision-drift unit example was added during full verification and explicitly
run green afterward. This full verification predates the inherited Git
environment correction below. It is local evidence; hosted final-head CI remains
a delivery check.

Independent review exposed inherited `GIT_INDEX_FILE` redirecting checkout
writes outside the private directory. The new process-isolated regression first
failed when an absent external index was created. Clearing repository-local Git
environment overrides fixed the defect; the regression checks both absent and
existing external indexes and verifies the actual resolved commit. A global Git
`insteadOf` rewrite remains effective, with protocols restricted to local files
for this test. An additional unit test ensures the cleared set covers the
installed Git's `rev-parse --local-env-vars` output. After correction, nine
source integration tests, three private unit tests and all-target Clippy passed.

Explicit Rust 1.85 all-target checking also passed after the correction. The
configuration-preservation variant was then rerun green. Independent scoped
review approved the correction after the inherited-index finding was fixed.
Final `just verify` passed after the correction and gardening, including all
tests, lint, documentation checks and build. Hosted Linux/Windows checks and
native client qualification remain separate delivery evidence.

The bounded implementation plan is preserved in
[the archive](../archive/plans/2026-10-18-lifecycle-sources.md). The canonical
[source contract and design](../topics/0003-sources.md#selected-lifecycle-rule-sources)
describe the implemented boundary. The Gate 5 umbrella remains active because
source-backed command orchestration and complete lifecycle acceptance are still
pending. Captured applying adapters are implemented in the lifecycle command
module.

A bounded coverage correction strengthened the same nine integration examples
and three private unit examples. Git branch, tag and historic full SHA now
select three distinct commits with distinct captured bytes. Lexical containment
examples select the existing private metadata sibling (`../metadata`) and an
existing in-tree path with parent traversal (`item/../item`), and require the
source selection diagnostic. QualifiedScope independently rejects parent
components; the diagnostic assertion pins rejection at the source-selection
boundary rather than relying on that later defense. Local and Git examples
compile the same capture successfully before mutation, then reject compilation
through that same capture after a local edit or equal-byte Git revision change.
Local and repository fallback names, installation IDs and canonical source
labels are also asserted.

Four mutation controls failed as intended: ignoring the requested Git ref;
omitting compile revalidation after a local edit; omitting it after an
equal-byte Git revision change; and removing both source containment guards.
Production code was restored exactly afterward. The corrections change tests and
evidence, with no production behavior change.

The full PR review identified fixture isolation and three regression-protection
gaps. Fixture Git commands and isolated child builders now clear every inherited
`GIT_*` variable before applying explicit fixture configuration. Only controlled
regression children receive a temporary external index override. A process
example proves fixture setup and nested isolated children preserve both an
absent external index and an existing unrelated sentinel. Production command
creation is exposed only through a private helper so a unit test can inspect the
actual `Command::get_envs` removals for every repository-local key, rather than
checking constant membership alone. Native authentication configuration remains
unchanged.

A lexically normal `bridge/objects` selection traverses a committed ancestor
symlink to the private metadata sibling and requires the canonical-containment
error. Resource-bearing compilation now asserts two complete artifacts and two
matching contracts, exact entrypoint/resource keys and resource bytes, required
source-body presence in the generated entrypoint, modes, consumer profiles,
installation identity, captured-manifest digest and contract-digest consistency
with produced bytes. Archive status now identifies the earlier
immutable-Git-evidence deferral as historical and links to the implemented
selected-source contract.

All four associated mutation controls failed at runtime: removing fixture
isolation, clearing only the index override, deleting canonical containment, and
filtering out the generated entrypoint contract when a resource contract exists.
All temporary mutations were restored. Controlled launchers clear ambient Git
variables without logging their values. Final coverage is eleven integration
examples and three private unit examples; native-client and
network-authentication acceptance remain outside this evidence.
