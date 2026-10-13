# upskill — Specification

> Upskill your coding agents.

## 1. Overview

`upskill` is a single-binary CLI that lets authors **create, manage, and
prolong** AI-assistance content — rules, skills, agents — across multiple AI
coding clients (Claude Code, GitHub Copilot, opencode) from a single source of
truth.

The central abstraction is **generation**: the SSOT (Single Source of Truth)
on disk is portable; per-client output is produced on demand by a generation
pipeline.

### 1.1 Three item kinds

| Kind  | Purpose                             | Entrypoint file |
| ----- | ----------------------------------- | --------------- |
| Rule  | Always-on behavioural guidance.     | `RULE.md`       |
| Skill | On-demand procedural content.       | `SKILL.md`      |
| Agent | Persona definition with tool scope. | `AGENT.md`      |

The on-disk contract is defined by the [portable format spec](./format-spec.md).
This document describes upskill's behaviour against that contract.

### 1.2 Two scopes

| Scope   | Lives at         | Use                                            |
| ------- | ---------------- | ---------------------------------------------- |
| Project | `<cwd>/.agents/` | Content shared across the team via git.        |
| Global  | `$HOME/.agents/` | Content available to a single user, all repos. |

### 1.3 Two roles

- **Source registry** — repository where SSOT items are authored. Holds the
  canonical `RULE.md` / `SKILL.md` / `AGENT.md` files and `*.bundle.yaml`
  manifests. Author commands (`new`, `lint`, `fmt`) operate here.
- **Consumer project** — repository where generated outputs are installed
  for use by AI clients. Holds only generated per-client files. Consumer
  commands (`add`, `remove`, `update`, `list`, `doctor`, `search`) operate
  here.

The same repository can be both, but SSOT and generated outputs do not mix
in the same tree.

## 2. CLI surface

No overlap between verbs.

| Command  | Role     | Purpose                                             |
| -------- | -------- | --------------------------------------------------- |
| `add`    | Consumer | Install items or bundles from any source.           |
| `remove` | Consumer | Remove installed content.                           |
| `update` | Consumer | Pull latest, regenerate changed items.              |
| `list`   | Consumer | Show installed content from the lock file.          |
| `doctor` | Consumer | Verify installation consistency.                    |
| `search` | Consumer | Look up skills via the public registry.             |
| `index`  | Consumer | Build or manage the local registry index cache.     |
| `new`    | Author   | Scaffold a new rule, skill, or agent.               |
| `lint`   | Author   | Validate SSOT files against the format spec.        |
| `fmt`    | Author   | Canonicalise YAML frontmatter (key order, quoting). |

### 2.1 `add`

```text
upskill add <source> [items...] [--global|--project]
```

`<source>` accepts (parity with `npx skills add`):

- `owner/repo` — GitHub shorthand
- `owner/repo@ref` — pinned ref (branch, tag, or commit SHA)
- `owner/repo:path/to/item` — subfolder
- `owner/repo:path/to/name.bundle.yaml` — bundle file (resolves transitively, see §2.6)
- `owner/repo@ref:path` — combined
- `https://github.com/owner/repo[...]` — full HTTPS URL
- `https://<host>/<path>[...]` — any other https git host: GitLab
  (including self-hosted instances and subgroups at any nesting depth),
  Bitbucket, Gitea, or a plain git server
- `./path`, `../path`, `/abs/path`, `~/path` — local paths

`upskill add <source>` installs everything the source contains. Optional
`items...` after the source filter to a subset. There is no interactive
picker; install-all-by-default fits bundle curation.

Default scope is `--project` (write into the current repo's `.agents/...`),
falling back to `--global` (`$HOME/.agents/...`) when the current directory
is not inside a git repo. Either flag can be passed explicitly.

### 2.2 `update`

```text
upskill update [name...] [--global|--project] [--dry-run]
```

Always fetches the latest source before regenerating. Absorbs the role of a
separate `sync` command — there is no `--offline` flag in v1; offline use is
covered by passing local paths to `add` / `update`.

`update` covers two distinct behaviours:

- **Absorb client drift.** When Copilot or Claude Code change a path or
  frontmatter field, `update` regenerates outputs from the same SSOT.
- **Track upstream evolution.** `update` git-pulls source repos and
  regenerates items whose source hash changed.

Persisted schema-1 `gitlab:` and `gitlab+<host>:` source labels remain
readable for updates. Unambiguous historical labels decode to generic HTTPS
sources, preserving namespace, ref and subfolder. Applying `update` canonicalizes
the source labels of reinstalled items and bundles, even when their content is
unchanged. `update --dry-run`, `list` and `doctor` do not rewrite lockfiles or
generated output. Aliases, recorded selection, bundle membership and unrelated
item/plugin/MCP records remain intact. This does not introduce schema 2 or infer
artifact ownership.

The supported historical authority grammar and reconciliation limits are defined
in the [ADR-0011 amendment](https://github.com/driftsys/upskill/blob/main/docs/adr/0011-generic-git-url-source.md#amendment-persisted-gitlab-source-labels-2026-10-12).
Unsupported or ambiguous labels fail before fetch with the recorded label and
guidance to reconcile a verified HTTPS URL, ref and subfolder. Modern `add` still
rejects the removed GitLab shorthands with exit 2; use a full HTTPS URL.

### 2.3 `remove`

```text
upskill remove [name...] [--source <label>] [--global]
```

Removes installed items from the matching scope. Either name one or more
items, or pass `--source <label>` to remove every item that came from a
single source. Bare `upskill remove` is rejected — be explicit. Ancillary
files (`CLAUDE.md`, `opencode.json`, `.vscode/settings.json`) are not
touched.

### 2.4 `list` / `doctor`

- `list` — show installed content from `.upskill-lock.json`, grouped by
  kind. Bundles are surfaced in a separate section. The `--available`
  view (items discoverable from configured sources) is deferred for a
  future release.
- `doctor` — verify on-disk state matches the lock file. Five
  independent buckets: missing per-client outputs (reinstall fixes),
  SSOT hash drift on `local:` sources (`update` fixes), lockfile
  entries with no recoverable source (`remove` fixes), installed
  plugins missing from the client (`update` reinstalls), and skipped
  plugins (informational — CLI was absent at install time). Exit 0
  when clean, 1 when any drift bucket is non-empty. Skipped plugins
  are warnings and do not affect the exit code.

### 2.5 `new` / `lint` / `fmt`

Author commands. Run inside a source-registry working tree.

- `new <kind> <name>` — scaffold a new SSOT item directory (`<kind>` is one
  of `rule`, `skill`, `agent`).
- `lint [paths...] [--strict]` — validate SSOT files. `--strict` is the CI
  mode (warnings become errors).
- `fmt [paths...]` — canonicalise YAML frontmatter only. Markdown body
  formatting is dprint's responsibility.

### 2.6 Bundles

A **bundle** is a `*.bundle.yaml` manifest that names a curated set of items
and (optionally) other bundles to install together. Bundles contain no
content of their own; they reference items by `name`. Frontmatter shape is
defined in [format-spec §3.7](./format-spec.md#37-bundle-schema).

`upskill add <source>:path/to/foo.bundle.yaml` installs a bundle.
Implementation behaviour:

- **Items.** Every entry in `items.rules`, `items.skills`, `items.agents` is
  resolved against the same source repository and installed. Unresolved
  names error out before any write.
- **Transitive `requires:`.** When the bundle declares `requires:`, every
  bundle in the transitive closure is installed. A bare `{ name }` entry is
  resolved against the same source; a `{ name, source }` entry is resolved
  cross-source (`source` reuses the `upskill add` DSL), and the closure MAY
  span sources. Each bundle and item is recorded in the lockfile under its own
  source. Circular `requires:` is rejected; the same bundle name resolving to
  two different sources within one closure is an error.
- **Lockfile.** Each installed bundle is recorded as a top-level `bundles[]`
  entry in `.upskill-lock.json` alongside its resolved items. The same item
  may appear in multiple bundle `items[]` lists; it is installed once.
- **Removal.** `upskill remove --source <label>` removes every item that
  came from that source, including those introduced via bundles. A
  bundle-name-targeted removal (`remove <bundle-name>` removing only items
  exclusive to that bundle) is not yet implemented.

### 2.7 Aliases

`add` does **not** alias to `install`. `remove` does **not** alias to
`uninstall`. The unified verb names are canonical.

## 3. Generation pipeline

SSOT → per-client output runs on the developer's machine, on every install
or update. No CI generation step. No committed `dist/` artifacts.

### 3.1 Per-client output paths

| Item kind | Claude Code                             | GitHub Copilot                                                | opencode                                           |
| --------- | --------------------------------------- | ------------------------------------------------------------- | -------------------------------------------------- |
| Rule      | `.claude/rules/<name>.md` with `paths:` | `.github/instructions/<name>.instructions.md` with `applyTo:` | `.agents/rules/<name>/RULE.md` (canonical-store)   |
| Skill     | `.claude/skills/<name>/SKILL.md`        | `.github/skills/<name>/SKILL.md`                              | `.agents/skills/<name>/SKILL.md` (canonical-store) |
| Agent     | `.claude/agents/<name>.md`              | `.github/agents/<name>.agent.md`                              | `.opencode/agents/<name>.md`                       |

Per-client frontmatter mapping is defined in [format-spec §7](./format-spec.md#7-generation-client-specific-output)
and Appendix B.

### 3.2 Copy, not symlink

All installation is file copy or generated output. No symlinks anywhere. One
code path; Windows portability without Developer Mode or
`core.symlinks=true`.

### 3.3 Markdown formatting

Generated output passes through embedded `dprint-plugin-markdown`
(exact-pinned at `=0.21.1`) before writing. This guarantees idempotence under
the same formatter in `dprint.json`. Authors who run `dprint` locally see no
diffs on generated files.

### 3.4 Ancillary file handling

Three files outside the per-item path tree are managed idempotently:

- **`CLAUDE.md`** at repo root — created once with `@AGENTS.md` content if
  absent. Never overwritten.
- **`.vscode/settings.json`** — `chat.instructionsFilesLocations` is set;
  other keys preserved.
- **`opencode.json`** — managed only on **first** opencode-rule install.
  The entry `".agents/rules/**/RULE.md"` is added to `instructions[]` if not
  already present, then the file is never mutated again. opencode's glob
  picks up subsequent rule additions and removals automatically.

## 4. State files

`.upskill-lock.json` records what was installed, from where, at what
resolved git ref, with what content hashes. Same schema (`schema: 1`),
two possible locations:

| Scope   | Location                   | Committed? |
| ------- | -------------------------- | ---------- |
| Project | `<cwd>/.upskill-lock.json` | Yes        |
| Global  | `$HOME/.upskill-lock.json` | No         |

The lockfile plays the same role as `package-lock.json`: deterministic
regeneration on another developer's machine and in CI for project scope,
and across-repo continuity for global scope.

```json
{
  "schema": 1,
  "items": [
    {
      "kind": "skill",
      "name": "code-review",
      "source": "github:driftsys/skills",
      "ref": "v1.2.0",
      "hash": "sha256:..."
    }
  ],
  "bundles": [
    {
      "name": "platform-defaults",
      "source": "github:driftsys/bundles",
      "ref": "v0.4.0",
      "items": ["code-review", "secret-scanner"]
    }
  ],
  "plugins": []
}
```

### 4.1 Pure legacy-state projection

The internal library boundary `legacy::project_legacy(&Lockfile)` describes typed
schema-1 state without I/O or mutation. It retains an owned snapshot and indexed
item and bundle rows, including order, duplicates, installed aliases, source names,
recorded refs and SSOT hashes, dependency/group links, and raw client strings.
Explicitly tagged plugin and MCP rows preserve their original targets, scopes,
statuses and bundle references. Typed preservation does not recover unknown JSON
fields or the original serialization bytes.

Persisted source labels are decoded with the existing compatibility parser. A
parsed source or recorded ref/hash does not prove an immutable source revision or
ownership of generated bytes. Requested selection remains explicitly unresolved
for every item, including singleton effective client lists. Empty client lists
expand to the fixed historical Claude, Copilot and OpenCode clients; unknown
spellings remain in the snapshot and produce structured diagnostics.

Candidates contain only a historical client, a relative entrypoint and a relative
resource search base. The frozen nine kind/client layouts are shared with the
current pipeline and use the installed alias. They do not inspect roots, files or
resources, establish ownership, or authorize adoption or deletion. Bundle names
and ancillary files do not produce candidates. Invalid installed names suppress
candidates: empty names, dot, dot-dot, slash, backslash, colon and control
characters are invalid; spaces and Unicode remain representable. Invalid sources
retain otherwise known layout facts. Duplicate
identities and identical entrypoints identify all affected item indices without
selecting a winner. Diagnostics retain malformed rows for inspection. Unsupported
schema values retain all typed records but produce no interpreted sources, clients
or destinations.

This prerequisite leaves commands and schema-1 persistence unchanged. Verified
sources, reproducible historical rendering, resource enumeration, adoption,
requested-selection reconciliation, schema-2 lifecycle preservation, exact backup,
containment, preflight, writer locking and recovery remain later work.

See the [projection design](https://github.com/driftsys/upskill/blob/main/docs/design/generation-pipeline.md#pure-legacy-state-projection)
and [AD-0015](https://github.com/driftsys/upskill/blob/main/docs/adr/0015-pure-legacy-state-projection.md) for implementation
boundaries and rationale.

### 4.2 Pure owned instruction planning

The library function `instructions::plan::plan_instructions` joins validated
owned-artifact records with the Markdown aggregate editor. Callers supply complete
installation contribution replacements or removals and exact observations for
every affected file. The result contains the complete resulting artifact vector
and file plans with exact optional before bytes and after bytes, including content
no-ops. Absent files and existing empty files have distinct preconditions. Removing
the last region retains manual bytes and keeps the aggregate file, which is empty
only when no content remains. The plan has no deletion action.

Set replaces one installation's complete contribution set. Existing installation
IDs retain their source, item kind, source name and installed alias; recorded
revision may change. Untouched selectors retain their bodies, owners and consumers
and cannot be taken over, even with identical content. Affected regions require
one owner; affected whole files or shared regions conflict. Unaffected records,
including whole files and shared regions, survive exactly. At affected locations,
the editor rebuilds separator metadata for retained regions after other removals.

`instructions::aggregate::inspect_owned` reads exact verified Markdown bodies in
document order through the same private marker/span validator as reconciliation.
Edited, missing, duplicate, overlapping, unknown or malformed regions conflict.
Bytes outside managed regions remain unchanged. Locations use scope, raw lexical
root spelling and portable relative path; file plans sort by that key. Surviving
artifact rows retain prior order and new selectors append in location/ID order.

Planning performs no I/O and validates prior and final ownership plus all desired
data. Observations must match affected locations exactly; missing, duplicate or
extraneous observations conflict. Supplied bytes do not establish source or
filesystem evidence. Callers must resolve direct/bundle references and native
compatibility first. The separate pure whole-file planner is implemented in
[§4.4](#44-owned-whole-file-operation-plans); captured compilation in
[§4.5](#45-captured-item-compilation) derives complete per-client entrypoint and
resource outputs. The complete mixed coordinator is implemented in
[§4.6](#46-mixed-owned-artifact-operation-plans). Complete reference-graph authority,
per-owner consumer association, connecting compiled outputs to complete lifecycle
desired sets, state/lifecycle integration and transactional execution remain pending. This prerequisite changes no CLI behavior,
source format or lockfile format and enables no native client support.

See the [planner design](https://github.com/driftsys/upskill/blob/main/docs/design/owned-instruction-planning.md)
and its decision records for implementation boundaries and rationale.

### 4.3 Strict captured source manifests

The separate library module `source_manifest` provides
`capture_source(&Path) -> anyhow::Result<SourceManifest>` and pure
`validate_manifest(&SourceManifest) -> anyhow::Result<()>`. It does not change
commands, legacy source hashing/discovery, source formats or lockfile writes.
This supplies the captured values consumed by the implemented
[item compiler](#45-captured-item-compilation) to derive complete per-client
entrypoint and resource outputs. The pure planner for explicitly supplied
whole-file sets is implemented in [§4.4](#44-owned-whole-file-operation-plans).
Connecting compiled outputs to complete lifecycle desired sets remains pending.

`SourceManifest` contains sorted `CapturedSourceFile` records and a lowercase
`content_sha256`. Each file contains a UTF-8 relative `path` joined with `/`,
owned `bytes`, their lowercase `content_sha256`, and optional `mode`. Capture
includes every observed regular file without ignore rules or entrypoint
exclusions, including nested entrypoints, overrides, empty and binary files.
Empty directories have no record. Each file is read once; on Unix its observed
mode is `PermissionsExt::mode() & 0o7777`, and on other hosts mode is absent.
Mode evidence does not authorize applying permissions or executing content.

Capture reconstructs the root from `Path::components()` before metadata lookup,
so trailing separators and dot suffixes do not hide an observed root symlink.
It rejects observed root symlinks and non-directory roots, every observed entry
symlink and special entry, and enumeration, metadata or file-read errors. Errors
return no partial manifest. Relative paths reject empty, dot and parent components,
backslashes, colons, control characters and non-UTF-8 names. Hard-linked regular
files are represented by each path without a topology claim.

The manifest hash is SHA-256 over these exact bytes, in order:

1. ASCII `upskill:source-manifest:v1` followed by one NUL byte.
2. File count as an unsigned 64-bit big-endian integer.
3. For each file in Rust string path order: path byte length as unsigned 64-bit
   big-endian, UTF-8 path bytes, content byte length in the same encoding, exact
   content bytes, then mode-presence byte (`0` absent or `1` present). A present
   mode follows as an unsigned 32-bit big-endian integer.

Length conversion is checked. Per-file hash fields are verified but are not
additional digest inputs. Root spelling, timestamps and enumeration order are
excluded. Pure validation borrows the public values without mutation or sorting
and checks lexical paths, strict ascending uniqueness, optional mode range
`0..=0o7777`, exact file hashes and the aggregate hash. Public records and valid
hashes do not establish trusted provenance or source authenticity.

Standard synchronous filesystem observation is not an atomic snapshot,
descriptor-relative containment or protection against concurrent replacement.
Root ancestors may be symlinks. Lexical validation does not qualify destination
case collisions, reserved names, aliases or filesystem limits. Immutable Git
evidence, source stabilization, qualified filesystem observation, destination
qualification, compiler-to-lifecycle desired-set integration, lifecycle state and
execution, and Codex activation remain pending.

See the [source manifest design](https://github.com/driftsys/upskill/blob/main/docs/design/strict-source-manifests.md)
and its decision records for implementation boundaries and rationale.

### 4.4 Owned whole-file operation plans

The pure library module `files::plan` exports these public values, all deriving
`Debug`, `Clone`, `PartialEq` and `Eq` without serialization:

```rust
pub struct FileLocation {
    pub root: InstallationRoot,
    pub path: String,
}
pub struct FileValue {
    pub bytes: Vec<u8>,
    pub mode: Option<u32>,
}
pub struct ObservedFile {
    pub location: FileLocation,
    pub value: Option<FileValue>,
}
pub struct DesiredFile {
    pub location: FileLocation,
    pub value: FileValue,
    pub consumers: Vec<TargetSurface>,
}
pub enum FileInstallationChange {
    Set { owner: ArtifactOwner, files: Vec<DesiredFile> },
    Remove { installation_id: String },
}
pub struct PlannedFile {
    pub location: FileLocation,
    pub before: Option<FileValue>,
    pub after: Option<FileValue>,
}
pub struct FileOperationPlan {
    pub artifacts: Vec<OwnedArtifact>,
    pub files: Vec<PlannedFile>,
}
pub fn plan_files(
    prior: &[OwnedArtifact],
    changes: &[FileInstallationChange],
    observations: &[ObservedFile],
) -> anyhow::Result<FileOperationPlan>;
```

`FileLocation` uses the ownership model's `InstallationRoot`; desired files use
explicit `TargetSurface` consumers and changes use an explicit `ArtifactOwner`.
Inputs are borrowed and unchanged. Outputs own their bytes and ownership records.
The planner performs no I/O. An observed `None` means an absent directory entry,
not an unreadable file or directory; supplied observations are unqualified
before-state evidence and do not establish provenance.

Prior and desired records pass `ownership::validate_artifacts`. Observed and
desired locations require host-absolute nonempty roots and portable relative paths;
optional modes must be in `0..=0o7777`. Invalid locations and modes are rejected
before observation membership diagnostics. Location identity and sorting compare
scope, the exact root `OsStr` spelling and portable path. There is no component
normalization, canonicalization, case folding, alias resolution or containment
claim.

A `Set` supplies the complete nonempty whole-file set for its installation;
`Remove` removes its last files. Duplicate changes, unknown removals and duplicate
desired locations conflict. Reusing an installation ID cannot change source,
kind, source name or installed alias. Its revision may change consistently across
all replacement records. Equal content establishes neither compatibility nor
shared ownership.

Affected locations include prior records owned by selected installations and
all desired locations. Each requires exactly one observation, including
revision-only changes and byte-identical no-ops. Duplicate, missing and extraneous
observations conflict. Affected Markdown contributions and multi-owner whole files
conflict. Unrelated records, including shared files and contributions, survive
exactly. A desired file cannot take over an untouched installation; explicit
removal of the old owner plus selection of the new owner permits handoff only
when the recorded before-state is verified.

Owned locations require present observed bytes matching the recorded SHA-256 and
exact equality of the optional recorded and observed modes. `None` differs from
`Some(0)`. Edited or missing files conflict on update, removal and no-op. Unowned
locations require absence even when present bytes and modes equal desired values.
The planner does not reinterpret platform mode evidence or authorize permission
application.

An affected location cannot be a strict slash-component ancestor or descendant of
any prior or desired artifact within the same exact root and scope. Contributions
count as files here. `a` and `ab` are distinct siblings; removing `a` while creating
`a/b` still conflicts. Disjoint untouched topology pairs do not block other work.

The result contains one `PlannedFile` per affected location in lexical order,
with exact observed `before` and desired `after` values, including no-ops. Creation
has absent `before`; deletion has absent `after` and identifies only an individual
recorded file, granting no directory ownership or recursive removal authority.
Desired ownership records contain SHA-256 of exact desired bytes, optional mode,
explicit owner and explicit consumers. Untouched rows and same-location
replacements retain prior row order; removed rows disappear and new rows append
in lexical order. The final record set is validated before return. Any failure
returns no successful partial plan.

This prerequisite leaves existing commands, CLI flags, lockfile persistence and
native-client behavior unchanged. Connecting compiled captured item outputs to
complete lifecycle desired sets, qualified observation, destination
equivalence/containment, writer locking, permission policy, durable state,
apply/recovery and Codex activation remain pending. The mixed coordinator in
[§4.6](#46-mixed-owned-artifact-operation-plans) composes complete sets across both
forms; the individual planners do not authorize partial application of mixed
installations. This API does not complete M1-A2, M1-A3 or M1.

See the [whole-file planning design](https://github.com/driftsys/upskill/blob/main/docs/design/owned-file-planning.md)
and the active [operational lifecycle design](https://github.com/driftsys/upskill/blob/main/docs/wip/specs/2026-10-12-m1-owned-lifecycle-design.md).

### 4.5 Captured item compilation

The opt-in `compile::item` library API compiles one captured item into complete
per-client file sets without rereading source paths, writing destinations or
mutating inputs:

`compile_item` borrows a `SourceManifest` and `ItemCompileRequest` and returns
`anyhow::Result<CompiledItem>`. The exact public declarations are in
[`compile::item`](https://github.com/driftsys/upskill/blob/main/src/compile/item.rs).
`ItemCompileRequest` contains `kind: ItemKind`, `folder_name: String`,
`alias: Option<String>` and `clients: Vec<Client>`. `CompiledItemFile` contains
`path: String`, `bytes: Vec<u8>` and `mode: Option<u32>`. `CompiledClientItem`
contains `client: Client` and `files: Vec<CompiledItemFile>`. `CompiledItem`
contains `kind: ItemKind`, `source_name: String`, `installed_name: String`,
`source_manifest_sha256: String`, `outputs: Vec<CompiledClientItem>` and
`audience_excluded: Vec<Client>`.

All four structures derive `Debug`, `Clone`, `PartialEq` and `Eq`, without
serialization. `ItemKind` is the existing pipeline kind and `Client` is the
current Claude, Copilot and OpenCode generation vocabulary. The result owns its
values and records the exact validated supplied manifest digest as source identity;
that digest does not establish provenance or identify generated content.

Compilation first validates every manifest path, ordering, digest and mode,
including files later excluded or ignored. File/descendant pairs such as `a` and
`a/b` fail even when ignored. Requested clients must be explicit, nonempty and
unique. The canonical root entrypoint for the requested kind must exist, contain
UTF-8 and parse as its existing typed frontmatter with a supported schema.
The folder, resolved source name and optional alias must each be one portable
relative component. Skills use the folder name; rules and agents use their
optional declared name, otherwise the folder. An invalid declared rule or agent
name fails. An alias controls installed paths and resource namespaces while
rendered frontmatter retains the source identity under the existing renderers.

Outputs and `audience_excluded` follow `Client::ALL` order regardless of request
order. Each requested client omitted by canonical `audience` is explicitly
excluded; a wholly excluded selection succeeds with empty outputs. This does not
imply removal of existing files.

Resource selection reuses the existing root-only canonical entrypoint and known
`<KIND>.<client>.md` override exclusions and the requested kind's subtractive
`ignore` semantics. Nested equivalents, unrelated client-suffixed filenames,
unknown-client suffixes, hidden files, empty files and binary files remain resources
unless ignored. A root override for the requested kind and an emitted client
fails with an unsupported-override error even if ignored. Overrides for other
kinds or non-emitted clients remain excluded source inputs. Body override
precedence is deferred.

Each included output contains exactly one rendered entrypoint and every retained
resource, sorted by portable UTF-8 relative path. Compilation reuses existing
renderers, directives, passthrough, formatting and frozen historical layout.
For flat outputs it rewrites copied-resource links using the source name, then
re-prefixes source namespaces to the alias when different, including previously
namespaced links. Directory-backed outputs retain relative links. Duplicate or
file/descendant output paths fail; errors return no successful partial result.

Resource bytes and optional permission modes equal captured evidence exactly.
Generated entrypoints carry `mode: None`; this does not authorize permission
changes. A qualified writer must establish destination permission policy and
persist observed mode evidence separately.

Current commands, installer behavior and lockfile persistence remain unchanged.
Returning an OpenCode rule file does not install its instruction registration.
The API supplies no installation roots, ownership, destination observations,
reference claims or application instructions and makes no native support claim.
Connecting these outputs to lifecycle planning, registry/bundle resolution,
requested-selection reconciliation, qualified source/filesystem evidence, writer
locking, durable state, execution/recovery and Codex activation remain pending.
The separate mixed coordinator in [§4.6](#46-mixed-owned-artifact-operation-plans)
accepts explicit desired ownership and observations; compilation does not produce
those lifecycle inputs. This prerequisite completes neither M1-A2 nor
M1-A3.

See the [compiler design](https://github.com/driftsys/upskill/blob/main/docs/design/captured-item-compilation.md),
[AD-0024](https://github.com/driftsys/upskill/blob/main/docs/adr/0024-pure-captured-item-compilation.md),
[AD-0025](https://github.com/driftsys/upskill/blob/main/docs/adr/0025-reject-active-captured-overrides.md)
and [AD-0026](https://github.com/driftsys/upskill/blob/main/docs/adr/0026-separate-generated-permission-policy.md).

### 4.6 Mixed owned-artifact operation plans

The pure library module `operations::plan` composes the existing dedicated-file
and instruction planners into one complete installation operation. Its two new
value types derive `Debug`, `Clone`, `PartialEq` and `Eq`, without serialization:

```rust
pub enum DesiredArtifact {
    File(DesiredFile),
    Instruction(DesiredInstructionContribution),
}

pub enum InstallationChange {
    Set {
        owner: ArtifactOwner,
        artifacts: Vec<DesiredArtifact>,
    },
    Remove {
        installation_id: String,
    },
}

pub fn plan_owned_artifacts(
    prior: &[OwnedArtifact],
    changes: &[InstallationChange],
    observations: &[ObservedFile],
) -> anyhow::Result<FileOperationPlan>;
```

The existing `files::plan` and `instructions::plan` modules supply the desired
values. `ownership` supplies records and owners. The result reuses `FileOperationPlan`,
`PlannedFile`, `FileLocation` and `FileValue` from `files::plan`; both physical
forms therefore carry exact optional before and after values.

A Set replaces the installation's complete desired set across both forms.
Omitted old files and contributions are removed through their respective
planners; a global Remove reaches every old form. Empty Sets require explicit
Remove. Duplicate changes, unknown removals, duplicate desired file locations or
contribution selectors, invalid owners and invalid consumers conflict. An existing
installation retains its source, item kind, source name and installed alias even
when the old and desired forms are disjoint. Its revision may change coherently
across all desired records. Any multi-owner record at an affected location
conflicts; untouched shared rows survive exactly.

Locations compare `(scope, raw root OsStr, portable relative path)`. Raw dot
components and project/user scopes remain distinct. Roots must be nonempty and
absolute; paths must use portable relative components. Supplied optional modes
must be within `0..=0o7777`. Every affected physical location requires exactly one
observation, including no-ops: the affected union includes every old row belonging
to a changed installation and every desired location. Missing, duplicate or extra
observations conflict, and all observations are validated before membership checks.

A location cannot participate in both forms anywhere in the prior/desired union.
Even explicit old-owner removal and new-owner replacement cannot convert a
contribution into a whole file or the reverse at that location: contribution
ownership does not establish authority over manual aggregate bytes. Same-form
handoffs retain the underlying planners' rules. An ancestor/descendant pair in
the union conflicts when either location is affected, within the same exact root
and scope. This includes cross-form, removal-only and old/new transitions.
Slash-component boundaries distinguish `a/b` from `ab`; unrelated historical
pairs survive without a filesystem safety claim.

Dedicated actions retain exact optional before/after bytes and modes. Owned
bytes and optional modes must still match prior evidence on update, removal and
no-op; an unowned destination must be absent. Aggregate actions use the exact
observed before value and the editor's after bytes, preserving the observed
optional mode. An absent aggregate receives after mode `None`. Invalid UTF-8,
edited or unrecorded markers and missing owned aggregates conflict. Manual bytes
survive; removing the last contribution retains the aggregate, even when empty.
No permission authority follows from the supplied mode values.

Every affected action remains in the result, including byte-identical no-ops.
Actions sort by exact location and duplicate physical actions conflict. Retained
artifact identities replace rows at their original positions across the original
interleaved forms and installations. Removed identities disappear; new rows
append in exact location/selector order, with whole files distinguished from
contribution selectors. Final combined ownership is validated, including revision
coherence. Reordering changes, desired entries or observations does not change
the result. The API performs no I/O or input mutation; any failure returns no
successful partial plan. It may return the first contextual conflict rather than
all diagnostics.

This library prerequisite activates no CLI selector, native adapter, source fetch,
filesystem observation, lock, journal, schema-2 persistence or execution. Connecting
the captured compiler to complete lifecycle desired sets remains pending, along
with complete reference authority, per-owner consumer associations, immutable
source evidence, target qualification, filesystem safety, permission policy,
writer locking, durable state and apply/recovery. It does not complete M1-A2,
M1-A3 or M1. The public declarations are in
[`operations::plan`](https://github.com/driftsys/upskill/blob/main/src/operations/plan.rs).

### 4.7 Qualified scope roots and advisory writer locking

`filesystem::QualifiedScope::open(&InstallationRoot)` pins an existing absolute
scope directory on Linux and macOS. Parent components, missing or non-directory
roots and final symlinks are rejected. Dot components and trailing separators
are normalized before final symlink checks. Ancestor aliases are permitted during
explicit selection; native non-UTF-8 paths are preserved where the underlying
filesystem accepts them. Filesystem refusals remain contextual errors.

`verify_binding()` revalidates the normalized selected spelling, canonical path
and directory device/inode identity. A renamed, replaced or symlink-substituted
root, or retargeted ancestor alias, invalidates the old binding. The held handle
does not freeze the path namespace or establish descendant containment.

`try_lock_writer()` uses a fresh close-on-exec directory description and a
nonblocking exclusive advisory lock. It revalidates binding before and after
acquisition. Same-process and cross-process writers to one physical directory
contend, including aliases and different Project/User labels; distinct directories
remain independent. Contention returns a busy error. Other filesystem failures
remain errors. An opaque borrowing `ScopeWriterGuard` retains exclusion until
drop or holder termination; ordinary exec does not retain it. Temporary fork
inheritance is possible. No fork-safe lease, fairness, network-filesystem guarantee
or exclusion against external editors is promised.

These operations create no files, directories or persistent state and preserve
existing bytes and permissions. The API exposes no raw descriptor, cloning or
write operation. Other operating systems explicitly report unsupported behavior.
This library prerequisite activates no CLI command, schema change, lifecycle
execution, descendant containment, observation or recovery. It does not complete
the M1 filesystem stage or Windows qualification.

See [AD-0028](../adr/0028-lock-qualified-scope-directories.md) and the
[qualified scope-lock design](../design/qualified-scope-locking.md) for rationale,
acceptance examples and platform evidence.

### 4.8 Qualified destination observations

`QualifiedScope::observe_destinations(&[FileLocation])` captures exact optional
`FileValue` values beneath the borrowed physical scope. Regular files return
binary bytes and Unix permission bits (`Some(mode & 0o7777)`); absent destinations
return `None`. Observations preserve the supplied native roots and portable
paths, sorted by scope, raw root spelling and relative path. Every supplied root
must independently qualify and bind to the same physical directory, including
ancestor-alias spellings and different Project/User labels. Empty input still
validates the scope and filesystem profile.

Capture validates portable paths and rejects every potentially overlapping pair
before descendant reads. Components may match when either contains non-ASCII
characters or both compare equal ignoring ASCII case. If every component pair
through the shorter path may match, the batch is rejected, including duplicates,
prefix pairs and aliases across root spellings or labels. This conservative rule
can reject distinct files on case-sensitive filesystems.

The inactive API qualifies APFS on macOS and ext-family/tmpfs filesystems on
Linux. Linux requires a returned statx mount ID; macOS retains the fstatfs
filesystem ID and mount bindings. Unknown profiles, missing mount evidence,
cross-mount traversal and other operating systems return explicit errors.
Descriptor-relative enumeration requires exact stored component spellings.
No-follow traversal rejects symlinks, non-directory ancestors, directory leaves,
files with multiple links and special files. Only ENOENT establishes absence;
recoverable filesystem, permission, I/O, descriptor-exhaustion and unsupported
failures remain contextual errors, including allocation errors reported by a
fallible read. Recoverable memory exhaustion is not guaranteed: ordinary Rust
infallible allocation failures may abort, and the operating system may terminate
the process instead of returning `Err`.

An opaque, non-Clone `DestinationSnapshot` retains close-on-exec root, directory
and file handles, selected root bindings, mount evidence and either exact file
stamps or the first missing component with its unresolved suffix. Capture checks
identity, type, links, size, mode, modification/change times and byte count around
each read. Before returning, and again from `verify_unchanged()`, it checks fresh
bindings and directory identities, exact stored spellings, mounts, exact bytes,
file stamps and absence evidence. These checks never refresh stale expectations.
Timestamp-only changes, edits with bytes and mtime restored but changed ctime,
same-size edits with restored mtime, chmod, identical-byte inode replacement,
ancestor replacement, missing-to-present transitions and root-alias retargeting
invalidate evidence. Access times and unrelated directory timestamp changes do
not invalidate it. Retained handles close on drop and returned-error paths;
process abort or termination does not guarantee Rust destructors run.

Observation creates no files, directories, locks or persistent state and acquires
no writer guard. Plain observations convey no execution or permission authority.
These sampled checks do not exclude external writers or hostile ancestor
relocation. A future executor must retain its writer guard and independently
establish complete lifecycle inputs, authority, permission policy, immediate
pre-effect checks and recovery. Existing commands and schema 1 remain unchanged;
Windows qualification and lifecycle execution remain pending. Hosted Linux
runtime qualification and Windows unsupported-boundary checks passed on
2026-10-13 for PR head `765e70b9b7ce5386530ffea1fe9814b228724428` (merge checkout
`242930ef9107aff6eff432484f474e47157983f2`) in
[Actions run 38330080515](https://github.com/driftsys/upskill/actions/runs/38330080515).
That evidence does not establish a qualified Windows backend or cover later
local timestamp and special-bit tests. See
[AD-0029](../adr/0029-capture-qualified-destination-evidence.md) and
the [destination observation design](../design/qualified-destination-observation.md).

## 5. Source format and authentication

Source format: see [§2.1](#21-add).

upskill never injects credentials into clone URLs. Clones use the bare
`https://<host>/...` URL and rely entirely on git's own configuration —
credential helpers, `url.<base>.insteadOf` rewrites, and SSH. A private
repository works whenever a manual `git clone <url>` would; configure git
the same way. upskill resolves no tokens and reads no token env vars.

Any https host is treated as an opaque git remote
(`https://git.mycompany.com/team/repo`), including projects
nested under groups to any depth
(`https://git.mycompany.com/group/subgroup/team/repo`).

## 6. CLI compliance

### 6.1 Exit codes

| Code  | Meaning                              |
| ----- | ------------------------------------ |
| `0`   | Success.                             |
| `1`   | General error (network, parse, I/O). |
| `2`   | Usage error (invalid args or flags). |
| `130` | Interrupted (Ctrl+C / SIGINT).       |

### 6.2 Conventions

- POSIX/GNU long flags (`--flag-name`) and short flags (`-f`).
- Stdout for data, stderr for progress and errors.
- Honors `NO_COLOR` and TTY detection (no color in pipes/CI).
- Global `-q` / `--quiet` suppresses informational stdout. Errors on
  stderr and exit codes are unaffected.
- Single static binary, no runtime dependencies.

### 6.3 Environment variables

| Variable         | Purpose                                        |
| ---------------- | ---------------------------------------------- |
| `NO_COLOR`       | Disable colored output.                        |
| `HTTPS_PROXY`    | HTTP proxy for network requests.               |
| `XDG_CACHE_HOME` | Override cache directory for registry indexes. |

## 7. Out of scope

- Runtime invocation of installed content (each AI client's responsibility).
- Hosting a registry server or central marketplace.
- Content quality review (delegated to AI workflows outside upskill).
- Telemetry, analytics, or usage tracking.
- Publishing to non-Git destinations (npm, crates.io, PyPI).

## 8. References

- Format spec: [`docs/specification/format-spec.md`](./format-spec.md)
- User guide: [Getting started](https://driftsys.github.io/upskill/getting-started.html), [Commands](https://driftsys.github.io/upskill/commands.html), [Recipes](https://driftsys.github.io/upskill/recipes.html)
- Agent Skills open standard: <https://agentskills.io>
- skills.sh ecosystem: <https://skills.sh>
