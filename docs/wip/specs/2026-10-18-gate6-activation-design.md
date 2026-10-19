# Gate 6 qualification and production activation

Status: revised on 2026-10-19 after the user's explicit architecture correction.
Original draft prepared against `eba3dae`; reconciled against `4d26d64` after
complete preflight reporting (#331). Production/native gates remain incomplete.
The [delivery contract](../../topics/0009-customization-delivery.md) and
[AD-0040](../../adr/0040-separate-customization-delivery-from-discovery.md)
supersede production interpretations of experimental managed `AGENTS.md`
contributions. Original draft remains preserved in the prior planning worktree.

## Intent and authority

Complete the remaining M1 rule/instruction gates before enabling ownership-aware
production commands and schema-2 writes. Source-backed library integration is
available as an experiment; independent Codex/OpenCode production rules require
qualified plugin delivery alongside unchanged team `AGENTS.md`. Native loading,
production routing and complete all-kind compatibility remain separate gates.

The [roadmap](../../roadmap.md) controls dependency order: M1 activation, then
owned-skill lifecycle, then automatic maintenance. The existing
[M1 implementation contract](../plans/2026-10-11-instructions-rules-m1.md),
[source contract](../../topics/0003-sources.md),
[lifecycle contract](../../topics/0005-lifecycle-planning.md) and
[qualification evidence](../../technotes/m1-gate6-native-qualification.md)
remain authoritative. This proposal does not supersede their safeguards.

## Settled decisions

The user delegated settlement of the remaining policies. The choices below are
author decisions under that delegation; they are requirements for subsequent
implementation, not evidence of working support.

1. Independent Codex/OpenCode rule delivery must use the qualified plugin route,
   never managed team `AGENTS.md` contributions as an implicit production
   fallback. Apply CD-01 through CD-10 across every customization and reader.
2. Preserve the four-client MVP: Claude Code, Copilot CLI, Codex CLI and
   OpenCode. Record CLI and IDE observations separately. Keep Copilot VS Code
   Local and Codex IDE obligations visible wherever an existing contract
   requires them; CLI evidence cannot satisfy those obligations.
3. Record an explicit version, operating system, scope, configuration and
   content-kind matrix before activation. Native results from macOS do not
   qualify Linux. Unsupported or unavailable cells remain explicit blockers for
   any support claim that includes them.
4. Reject production global Codex writes when an external `CODEX_HOME` cannot be
   bound to the declared qualified root. Supporting that home requires a
   separate recorded root/state authority contract and native evidence. Never
   silently fall back to a different personal instruction destination.
5. Introduce no automatic legacy ownership adoption. Migration requires exact
   original-state backup, complete preflight and the existing guarded recovery
   protocol.
6. Qualify every currently supported kind for compatibility with activation.
   This does not advance skills, agents, MCP or plugin lifecycle into M1. If a
   non-rule operation cannot safely preserve schema-2 state, it blocks broad
   activation until a compatible adapter or an explicitly approved narrower
   activation policy exists.

## Acceptance matrix

Each row needs named executable examples and evidence before it can pass.
Existing tests are candidate reuse points, not new qualification results.

| ID    | Required result                                                                                                                                                                                                             | Evidence and candidate coverage                                                                                                                                               |
| ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G6-01 | Explicit Codex selection works; no-selection defaults and existing client outputs retain their contracts. Unsupported target/scope combinations fail before writes.                                                         | CLI acceptance tests covering add, update, remove, list, doctor and preview; reuse `lifecycle_selection` and `codex_instruction_boundary`.                                    |
| G6-02 | Two independent plugin rules coexist alongside unchanged team `AGENTS.md`; update/remove preserves the other rule and all manual bytes/modes. Recorded removal/recovery needs no source or active adapter.                  | Installed-plugin project/user examples and CLI tests; existing lifecycle tests prove only reusable preservation/recovery foundations.                                         |
| G6-03 | Complete selected-set conflicts are reported deterministically before any journal or publication. Reader reports include excluded content and incidental readers without granting ownership.                                | Multiple independent conflicts, changed and unchanged controls, exact report inventories, and no-effect filesystem/state assertions.                                          |
| G6-04 | Production uses the qualified root, writer lock, exact schema-1 backup and guarded journal/recovery boundaries. Failed or interrupted operations retain recoverable state.                                                  | Process-termination checkpoints and lock contention on each supported executor platform; reuse `lifecycle_application` and `lifecycle_state_origin`.                          |
| G6-05 | Schema-1 migration preserves original bytes; schema-2 reads and all existing commands preserve unrelated kinds and ownership. Unsupported states fail without mutation.                                                     | Mixed rule/skill/agent fixtures, current MCP/plugin command inventory, migration and regression tests. Record each command's supported state transitions explicitly.          |
| G6-06 | Installed Codex/OpenCode plugins deliver independent rules in fresh and resumed sessions beside team guidance. Disabled/removed plugin controls suppress only managed rules; canonical-file probes are background evidence. | CD-02/CD-03 plugin, team-guidance, disabled/removed and unintended-reader controls at exact version/OS/scope/configuration; failed requests never qualify negatives.          |
| G6-07 | All customization payloads have explicit intended/unintended/unresolved readers, including compatibility directories, imports, globs and shared paths; test negative loading, scoped matching and duplicate activation.     | CD-01/CD-03/CD-04 inventories and native matching/nonmatching controls; direct-plus-plugin, aliases, project/user copies and compatibility imports must not activate twice.   |
| G6-08 | User-root and external-home authority is explicit and enforced before source resolution or mutation.                                                                                                                        | Isolated authenticated user-root fixtures and invalid-root/no-effect controls; never modify real personal instructions to obtain evidence.                                    |
| G6-09 | Rules, skills/resources, agents, bundles, plugins and MCP retain command, configuration and native behavior; hooks/tools/permissions remain visible without expanding implementation scope.                                 | Per-client CD matrix with F/R/N/B preservation, configuration/native effects and status; mixed-state CD-05 through CD-08 controls. Missing applicable cells block activation. |
| G6-10 | Published contracts, migration guidance, support matrix and failure/recovery guidance match the production routes.                                                                                                          | Specification, usage, ADR updates, rendered documentation checks and exact-head review/CI evidence.                                                                           |

### Diagnostic boundary

Aggregate independent selected-item resolution, parsing, capability and
destination conflicts when their required observations can be obtained safely. A
failed source must not hide independently observable conflicts in other selected
items. Do not invent downstream diagnostics for an item that failed resolution
or compilation. Root qualification, writer-lock acquisition and invalid
authoritative state remain operation-level failures: stop when safe inspection
cannot proceed. Existing pure planners may retain first-error results; the
orchestration layer owns operation-level collection and stable ordering by
installation identity and diagnostic category.

Add a G6-03 acceptance example containing one invalid source and two valid
sources with independent destination conflicts. Require all three independently
observable failures, no fabricated errors for the invalid source, and no
publication. This diagnostic policy is settled for subsequent implementation.

## Native evidence inventory

Create one row for every required client/version/OS/scope/kind combination.
Record automatic discovery, representative behavior, canonical positive control,
staging and unintended-reader negative controls, duplicate-activation controls,
relevant configuration, sanitized receipts and hashes, and evidence path.
Distinguish pass, fail, blocked and not run.

The existing native note records bounded historical project observations. It
also records that older successful controls predate stronger harness isolation,
all user-root discovery remains unqualified, and OpenCode configured discovery
differs from default discovery. Re-run affected cells with probes for the
corrected installed-plugin/native routes; the existing harness tests canonical
files only and cannot qualify those routes. Do not upgrade historical
observations using offline tests. Linux hosted Rust checks do not supply Linux
native-client evidence.

### Required support matrix

| Native surface  | macOS arm64 project/user                 | Linux x86_64 project/user                | Version policy               |
| --------------- | ---------------------------------------- | ---------------------------------------- | ---------------------------- |
| Claude Code CLI | Required; not yet qualified by this plan | Required; not yet qualified by this plan | One pinned stable release    |
| Copilot CLI     | Required; not yet qualified by this plan | Required; not yet qualified by this plan | One pinned stable release    |
| Codex CLI       | Required; not yet qualified by this plan | Required; not yet qualified by this plan | One pinned stable release    |
| OpenCode CLI    | Required; not yet qualified by this plan | Required; not yet qualified by this plan | One pinned stable v2 release |

These sixteen client/platform/scope cells are the minimum new native
qualification baseline. At the start of qualification, select the latest stable
released version available on both required platforms, verify its upstream
contract, and record the exact version and OS release. Freeze those versions for
the campaign; any version change requires rerunning affected cells. No rolling
"latest" support claim follows. OpenCode v1 receives no new qualification claim.

macOS x86_64 and Linux arm64 remain release binary targets. Executor, migration
and compatibility verification must cover the existing platform contract; the
two-platform native baseline does not remove binary support or qualify native
loading on other architectures. Windows, WSL, IDE, remote and cloud surfaces
receive no new native support claim from this campaign. Existing supported IDE
behavior must still pass regression checks; any M1 requirement that needs
IDE-native proof remains an additional blocker until satisfied.

Apply the kind inventory from G6-09 to every applicable matrix cell. An
unsupported kind/client combination requires a documented capability rejection,
not a fabricated native pass. A missing required cell blocks broad activation;
there is no automatic project-only or single-client fallback release.

### Codex home and state authority

The experimental global contribution destination is
`<qualified user root>/.codex/AGENTS.md`; it is not the production
independent-rule route. The intended plugin adapter must bind its actual
package, configuration, state and rule payload roots before qualification. For
any retained experimental write, keep the existing default-home restriction
below. Accept unset `CODEX_HOME` or an explicit absolute path that resolves to
that same qualified `.codex` directory. Reject relative, unresolved or external
home overrides before source resolution and before mutation, even if the
external path is otherwise accessible. Never ignore an override and write to the
default home. Preserve the existing user scope lockfile authority; this policy
does not relocate it into `CODEX_HOME`.

Project operations bind to the declared project root and project state. An
external Codex home does not authorize global writes during a project operation,
and project-native evidence must record that configuration separately. Read-only
inspection and recorded removal/recovery must retain their existing contracts:
validate recorded roots and ownership rather than retargeting to the current
environment. If safe access cannot be established, fail without writes.

External-home support is a later task requiring an explicit destination/state
mapping, qualified root acquisition, migration/recovery examples and native
user-scope evidence. This is a scope decision, not a native client limitation.

Credentials must remain in normal client-managed authentication and must never
be copied into fixtures or tracked evidence.

## Reconciliation of earlier acceptance assumptions

| Earlier statement                                                            | Corrected interpretation                                                                                                                 |
| ---------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Two managed contributions in team `AGENTS.md` qualify production Codex rules | Experimental aggregate-editor evidence only; production requires two independently managed plugin rules and byte-identical team guidance |
| Canonical instruction positives qualify the next adapter                     | They qualify only the exact canonical-file fixture; installed plugin lifecycle and unintended-reader controls remain separate            |
| Reporting incidental readers plus equal body text makes sharing acceptable   | Reports reveal exposure; explicit intended consumers and native equivalence/negative evidence are required                               |
| OpenCode only needs preservation of old rule behavior                        | OpenCode is an equal initial client; independent plugin rules and all applicable customization cells are required                        |
| Schema-2 availability permits switching production dispatch                  | Every installed-state command, native side effect, configuration owner and corrected G6/CD gate must pass first                          |

## Delivery sequence

1. Reconcile the roadmap, AD-0040, all active M1/Gate 6 plans and CD matrix;
   classify merged experiments and historical observations without promoting
   them. Inventory all current command/kind combinations and four-client
   discovery.
2. Pin released client versions and roots; produce installed-plugin probes for
   independent Codex/OpenCode rule delivery alongside manual team instructions.
   Qualify lifecycle/trust/context entrypoints, scope and precedence before
   choosing package/event implementation. An unavailable route blocks
   activation.
3. Write failing CD-01 through CD-05 acceptance examples for compatibility
   directories, imports/globs, resources, agents, package exports, intended
   sharing and duplicate activation. Extend foundation reader reports and
   preflight without inventing owner/reference authority from exposure.
4. Implement bounded qualified adapters against those examples; reuse captured
   evidence, owner contracts, qualified writer/recovery and complete preflight
   (#331). Keep schema-1 production dispatch. Extend native-effect recovery only
   with explicit observed-state/inverse contracts.
5. Complete CD-06 through CD-09 mixed-kind command/native qualification,
   including all existing platforms/surfaces' regression obligations and the
   required four-client project/user macOS/Linux matrix. Record blocked cells
   without reducing release scope; native credentials stay client-managed.
6. Only after those prerequisites pass, deliver production dispatch and
   schema-1-to-2 migration together. First add CLI examples for G6-01 through
   G6-05/G6-08 plus plugin deactivation, manual configuration, unrelated
   ownership and interrupted/native-effect cases. No alternate writer or
   legacy-only save.
7. Update usage, migration, failure/reload and support contracts; run
   `just fmt`, `just verify`, `just sanity`, `just msrv-check` and native
   probes; retain receipts. Require exact-head independent review and hosted
   platform checks.

The
[corrected implementation plan](../plans/2026-10-19-customization-delivery-reconciliation.md)
provides file-level tasks and named failing examples. Track child issues under
the existing human-owned epic; do not create another epic. Historical completed
preflight work stays completed as a foundation, not as route qualification.

## Activation exit gate

Gate 6 passes only when the required matrix cells and G6-01 through G6-10 and
applicable CD-01 through CD-10 have linked evidence, production schema-2 routing
has passed review and hosted checks, and the supported behavior is documented. A
green library suite or merged source integration alone cannot satisfy this gate.

The builder may choose module boundaries, test decomposition and task sizing
within these contracts. Support scope and external-home authority follow the
settled policies above. Any narrower activation policy requires a new explicit
user decision; missing evidence does not authorize weakening this gate.
