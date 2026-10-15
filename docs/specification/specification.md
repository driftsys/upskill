# upskill — Specification

> Upskill your coding agents.

## 1. Overview

`upskill` is a single-binary CLI that lets authors **create, manage, and
prolong** AI-assistance content — rules, skills, agents — across multiple AI
coding clients (Claude Code, GitHub Copilot, opencode) from a single source of
truth.

The central abstraction is **generation**: the SSOT (Single Source of Truth) on
disk is portable; per-client output is produced on demand by a generation
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
- **Consumer project** — repository where generated outputs are installed for
  use by AI clients. Holds only generated per-client files. Consumer commands
  (`add`, `remove`, `update`, `list`, `doctor`, `search`) operate here.

The same repository can be both, but SSOT and generated outputs do not mix in
the same tree.

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
- `owner/repo:path/to/name.bundle.yaml` — bundle file (resolves transitively,
  see §2.6)
- `owner/repo@ref:path` — combined
- `https://github.com/owner/repo[...]` — full HTTPS URL
- `https://<host>/<path>[...]` — any other https git host: GitLab (including
  self-hosted instances and subgroups at any nesting depth), Bitbucket, Gitea,
  or a plain git server
- `./path`, `../path`, `/abs/path`, `~/path` — local paths

`upskill add <source>` installs everything the source contains. Optional
`items...` after the source filter to a subset. There is no interactive picker;
install-all-by-default fits bundle curation.

Default scope is `--project` (write into the current repo's `.agents/...`),
falling back to `--global` (`$HOME/.agents/...`) when the current directory is
not inside a git repo. Either flag can be passed explicitly.

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
- **Track upstream evolution.** `update` git-pulls source repos and regenerates
  items whose source hash changed.

Persisted schema-1 `gitlab:` and `gitlab+<host>:` source labels remain readable
for updates. Unambiguous historical labels decode to generic HTTPS sources,
preserving namespace, ref and subfolder. Applying `update` canonicalizes the
source labels of reinstalled items and bundles, even when their content is
unchanged. `update --dry-run`, `list` and `doctor` do not rewrite lockfiles or
generated output. Aliases, recorded selection, bundle membership and unrelated
item/plugin/MCP records remain intact. This does not introduce schema 2 or infer
artifact ownership.

The supported historical authority grammar and reconciliation limits are defined
in the
[ADR-0011 amendment](https://github.com/driftsys/upskill/blob/main/docs/adr/0011-generic-git-url-source.md#amendment-persisted-gitlab-source-labels-2026-10-12).
Unsupported or ambiguous labels fail before fetch with the recorded label and
guidance to reconcile a verified HTTPS URL, ref and subfolder. Modern `add`
still rejects the removed GitLab shorthands with exit 2; use a full HTTPS URL.

### 2.3 `remove`

```text
upskill remove [name...] [--source <label>] [--global]
```

Removes installed items from the matching scope. Either name one or more items,
or pass `--source <label>` to remove every item that came from a single source.
Bare `upskill remove` is rejected — be explicit. Ancillary files (`CLAUDE.md`,
`opencode.json`, `.vscode/settings.json`) are not touched.

### 2.4 `list` / `doctor`

- `list` — show installed content from `.upskill-lock.json`, grouped by kind.
  Bundles are surfaced in a separate section. The `--available` view (items
  discoverable from configured sources) is deferred for a future release.
- `doctor` — verify on-disk state matches the lock file. Five independent
  buckets: missing per-client outputs (reinstall fixes), SSOT hash drift on
  `local:` sources (`update` fixes), lockfile entries with no recoverable source
  (`remove` fixes), installed plugins missing from the client (`update`
  reinstalls), and skipped plugins (informational — CLI was absent at install
  time). Exit 0 when clean, 1 when any drift bucket is non-empty. Skipped
  plugins are warnings and do not affect the exit code.

### 2.5 `new` / `lint` / `fmt`

Author commands. Run inside a source-registry working tree.

- `new <kind> <name>` — scaffold a new SSOT item directory (`<kind>` is one of
  `rule`, `skill`, `agent`).
- `lint [paths...] [--strict]` — validate SSOT files. `--strict` is the CI mode
  (warnings become errors).
- `fmt [paths...]` — canonicalise YAML frontmatter only. Markdown body
  formatting is dprint's responsibility.

### 2.6 Bundles

A **bundle** is a `*.bundle.yaml` manifest that names a curated set of items and
(optionally) other bundles to install together. Bundles contain no content of
their own; they reference items by `name`. Frontmatter shape is defined in
[format-spec §3.7](./format-spec.md#37-bundle-schema).

`upskill add <source>:path/to/foo.bundle.yaml` installs a bundle. Implementation
behaviour:

- **Items.** Every entry in `items.rules`, `items.skills`, `items.agents` is
  resolved against the same source repository and installed. Unresolved names
  error out before any write.
- **Transitive `requires:`.** When the bundle declares `requires:`, every bundle
  in the transitive closure is installed. A bare `{ name }` entry is resolved
  against the same source; a `{ name, source }` entry is resolved cross-source
  (`source` reuses the `upskill add` DSL), and the closure MAY span sources.
  Each bundle and item is recorded in the lockfile under its own source.
  Circular `requires:` is rejected; the same bundle name resolving to two
  different sources within one closure is an error.
- **Lockfile.** Each installed bundle is recorded as a top-level `bundles[]`
  entry in `.upskill-lock.json` alongside its resolved items. The same item may
  appear in multiple bundle `items[]` lists; it is installed once.
- **Removal.** `upskill remove --source <label>` removes every item that came
  from that source, including those introduced via bundles. A
  bundle-name-targeted removal (`remove <bundle-name>` removing only items
  exclusive to that bundle) is not yet implemented.

### 2.7 Aliases

`add` does **not** alias to `install`. `remove` does **not** alias to
`uninstall`. The unified verb names are canonical.

## 3. Generation pipeline

SSOT → per-client output runs on the developer's machine, on every install or
update. No CI generation step. No committed `dist/` artifacts.

### 3.1 Per-client output paths

| Item kind | Claude Code                             | GitHub Copilot                                                | opencode                                           |
| --------- | --------------------------------------- | ------------------------------------------------------------- | -------------------------------------------------- |
| Rule      | `.claude/rules/<name>.md` with `paths:` | `.github/instructions/<name>.instructions.md` with `applyTo:` | `.agents/rules/<name>/RULE.md` (canonical-store)   |
| Skill     | `.claude/skills/<name>/SKILL.md`        | `.github/skills/<name>/SKILL.md`                              | `.agents/skills/<name>/SKILL.md` (canonical-store) |
| Agent     | `.claude/agents/<name>.md`              | `.github/agents/<name>.agent.md`                              | `.opencode/agents/<name>.md`                       |

Per-client frontmatter mapping is defined in
[format-spec §7](./format-spec.md#7-generation-client-specific-output) and
Appendix B.

### 3.2 Copy, not symlink

All installation is file copy or generated output. No symlinks anywhere. One
code path; Windows portability without Developer Mode or `core.symlinks=true`.

### 3.3 Markdown formatting

Generated output passes through embedded `dprint-plugin-markdown` (exact-pinned
at `=0.21.1`) before writing. This guarantees idempotence under the same
formatter in `dprint.json`. Authors who run `dprint` locally see no diffs on
generated files.

### 3.4 Ancillary file handling

Three files outside the per-item path tree are managed idempotently:

- **`CLAUDE.md`** at repo root — created once with `@AGENTS.md` content if
  absent. Never overwritten.
- **`.vscode/settings.json`** — `chat.instructionsFilesLocations` is set; other
  keys preserved.
- **`opencode.json`** — managed only on **first** opencode-rule install. The
  entry `".agents/rules/**/RULE.md"` is added to `instructions[]` if not already
  present, then the file is never mutated again. opencode's glob picks up
  subsequent rule additions and removals automatically.

## 4. State files

`.upskill-lock.json` records what was installed, from where, at what resolved
git ref, with what content hashes. Same schema (`schema: 1`), two possible
locations:

| Scope   | Location                   | Committed? |
| ------- | -------------------------- | ---------- |
| Project | `<cwd>/.upskill-lock.json` | Yes        |
| Global  | `$HOME/.upskill-lock.json` | No         |

The lockfile plays the same role as `package-lock.json`: deterministic
regeneration on another developer's machine and in CI for project scope, and
across-repo continuity for global scope.

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

The internal library boundary `legacy::project_legacy(&Lockfile)` describes
typed schema-1 state without I/O or mutation. It retains an owned snapshot and
indexed item and bundle rows, including order, duplicates, installed aliases,
source names, recorded refs and SSOT hashes, dependency/group links, and raw
client strings. Explicitly tagged plugin and MCP rows preserve their original
targets, scopes, statuses and bundle references. Typed preservation does not
recover unknown JSON fields or the original serialization bytes.

Persisted source labels are decoded with the existing compatibility parser. A
parsed source or recorded ref/hash does not prove an immutable source revision
or ownership of generated bytes. Requested selection remains explicitly
unresolved for every item, including singleton effective client lists. Empty
client lists expand to the fixed historical Claude, Copilot and OpenCode
clients; unknown spellings remain in the snapshot and produce structured
diagnostics.

Candidates contain only a historical client, a relative entrypoint and a
relative resource search base. The frozen nine kind/client layouts are shared
with the current pipeline and use the installed alias. They do not inspect
roots, files or resources, establish ownership, or authorize adoption or
deletion. Bundle names and ancillary files do not produce candidates. Invalid
installed names suppress candidates: empty names, dot, dot-dot, slash,
backslash, colon and control characters are invalid; spaces and Unicode remain
representable. Invalid sources retain otherwise known layout facts. Duplicate
identities and identical entrypoints identify all affected item indices without
selecting a winner. Diagnostics retain malformed rows for inspection.
Unsupported schema values retain all typed records but produce no interpreted
sources, clients or destinations.

This prerequisite leaves commands and schema-1 persistence unchanged. Verified
sources, reproducible historical rendering, resource enumeration, adoption,
requested-selection reconciliation, schema-2 lifecycle preservation, exact
backup, containment, preflight, writer locking and recovery remain later work.

See the
[projection design](https://github.com/driftsys/upskill/blob/main/docs/design/generation-pipeline.md#pure-legacy-state-projection)
and
[AD-0015](https://github.com/driftsys/upskill/blob/main/docs/adr/0015-pure-legacy-state-projection.md)
for implementation boundaries and rationale.

### 4.2 Pure owned instruction planning

The library function `instructions::plan::plan_instructions` joins validated
owned-artifact records with the Markdown aggregate editor. Callers supply
complete installation contribution replacements or removals and exact
observations for every affected file. The result contains the complete resulting
artifact vector and file plans with exact optional before bytes and after bytes,
including content no-ops. Absent files and existing empty files have distinct
preconditions. Removing the last region retains manual bytes and keeps the
aggregate file, which is empty only when no content remains. The plan has no
deletion action.

Set replaces one installation's complete contribution set. Existing installation
IDs retain their source, item kind, source name and installed alias; recorded
revision may change. Untouched selectors retain their bodies, owners and
consumers and cannot be taken over, even with identical content. Affected
regions require one owner; affected whole files or shared regions conflict.
Unaffected records, including whole files and shared regions, survive exactly.
At affected locations, the editor rebuilds separator metadata for retained
regions after other removals.

`instructions::aggregate::inspect_owned` reads exact verified Markdown bodies in
document order through the same private marker/span validator as reconciliation.
Edited, missing, duplicate, overlapping, unknown or malformed regions conflict.
Bytes outside managed regions remain unchanged. Locations use scope, raw lexical
root spelling and portable relative path; file plans sort by that key. Surviving
artifact rows retain prior order and new selectors append in location/ID order.

Planning performs no I/O and validates prior and final ownership plus all
desired data. Observations must match affected locations exactly; missing,
duplicate or extraneous observations conflict. Supplied bytes do not establish
source or filesystem evidence. Callers must resolve direct/bundle references and
native compatibility first. The separate pure whole-file planner is implemented
in [§4.4](#44-owned-whole-file-operation-plans); captured compilation in
[§4.5](#45-captured-item-compilation) derives complete per-client entrypoint and
resource outputs. The complete mixed coordinator is implemented in
[§4.6](#46-mixed-owned-artifact-operation-plans). Planner integration of the
implemented reference-graph authority and per-owner consumer associations,
integrating the captured-rule desired sets (§4.14) with state/lifecycle and
transactional execution remain pending. This prerequisite changes no CLI
behavior, source format or lockfile format and enables no native client support.

See the
[planner design](https://github.com/driftsys/upskill/blob/main/docs/design/owned-instruction-planning.md)
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
Captured-rule desired sets are implemented in §4.14; lifecycle reconciliation
remains pending.

`SourceManifest` contains sorted `CapturedSourceFile` records and a lowercase
`content_sha256`. Each file contains a UTF-8 relative `path` joined with `/`,
owned `bytes`, their lowercase `content_sha256`, and optional `mode`. Capture
includes every observed regular file without ignore rules or entrypoint
exclusions, including nested entrypoints, overrides, empty and binary files.
Empty directories have no record. Each file is read once; on Unix its observed
mode is `PermissionsExt::mode() & 0o7777`, and on other hosts mode is absent.
Mode evidence does not authorize applying permissions or executing content.

Capture reconstructs the root from `Path::components()` before metadata lookup,
so trailing separators and dot suffixes do not hide an observed root symlink. It
rejects observed root symlinks and non-directory roots, every observed entry
symlink and special entry, and enumeration, metadata or file-read errors. Errors
return no partial manifest. Relative paths reject empty, dot and parent
components, backslashes, colons, control characters and non-UTF-8 names.
Hard-linked regular files are represented by each path without a topology claim.

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
evidence, integration of captured-rule desired sets (§4.14) with lifecycle state
and execution, and Codex activation remain pending. Qualified scope roots and
writer locking (§4.7) and the inactive destination observation boundary (§4.8)
are implemented separately. Qualified source snapshots (§4.9) add retained
source evidence without changing this freely mutable value interface.

See the
[source manifest design](https://github.com/driftsys/upskill/blob/main/docs/design/strict-source-manifests.md)
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
Inputs are borrowed and unchanged. Outputs own their bytes and ownership
records. The planner performs no I/O. An observed `None` means an absent
directory entry, not an unreadable file or directory; supplied observations are
unqualified before-state evidence and do not establish provenance.

Prior and desired records pass `ownership::validate_artifacts`. Observed and
desired locations require host-absolute nonempty roots and portable relative
paths; optional modes must be in `0..=0o7777`. Invalid locations and modes are
rejected before observation membership diagnostics. Location identity and
sorting compare scope, the exact root `OsStr` spelling and portable path. There
is no component normalization, canonicalization, case folding, alias resolution
or containment claim.

A `Set` supplies the complete nonempty whole-file set for its installation;
`Remove` removes its last files. Duplicate changes, unknown removals and
duplicate desired locations conflict. Reusing an installation ID cannot change
source, kind, source name or installed alias. Its revision may change
consistently across all replacement records. Equal content establishes neither
compatibility nor shared ownership.

Affected locations include prior records owned by selected installations and all
desired locations. Each requires exactly one observation, including
revision-only changes and byte-identical no-ops. Duplicate, missing and
extraneous observations conflict. Affected Markdown contributions and
multi-owner whole files conflict. Unrelated records, including shared files and
contributions, survive exactly. A desired file cannot take over an untouched
installation; explicit removal of the old owner plus selection of the new owner
permits handoff only when the recorded before-state is verified.

Owned locations require present observed bytes matching the recorded SHA-256 and
exact equality of the optional recorded and observed modes. `None` differs from
`Some(0)`. Edited or missing files conflict on update, removal and no-op.
Unowned locations require absence even when present bytes and modes equal
desired values. The planner does not reinterpret platform mode evidence or
authorize permission application.

An affected location cannot be a strict slash-component ancestor or descendant
of any prior or desired artifact within the same exact root and scope.
Contributions count as files here. `a` and `ab` are distinct siblings; removing
`a` while creating `a/b` still conflicts. Disjoint untouched topology pairs do
not block other work.

The result contains one `PlannedFile` per affected location in lexical order,
with exact observed `before` and desired `after` values, including no-ops.
Creation has absent `before`; deletion has absent `after` and identifies only an
individual recorded file, granting no directory ownership or recursive removal
authority. Desired ownership records contain SHA-256 of exact desired bytes,
optional mode, explicit owner and explicit consumers. Untouched rows and
same-location replacements retain prior row order; removed rows disappear and
new rows append in lexical order. The final record set is validated before
return. Any failure returns no successful partial plan.

This prerequisite leaves existing commands, CLI flags, lockfile persistence and
native-client behavior unchanged. Integrating captured-rule desired sets (§4.14)
with lifecycle planning, qualified observation, destination
equivalence/containment, writer locking, permission policy, durable state,
apply/recovery and Codex activation remain pending. The mixed coordinator in
[§4.6](#46-mixed-owned-artifact-operation-plans) composes complete sets across
both forms; the individual planners do not authorize partial application of
mixed installations. This API does not complete M1-A2, M1-A3 or M1.

See the
[whole-file planning design](https://github.com/driftsys/upskill/blob/main/docs/design/owned-file-planning.md)
and the active
[operational lifecycle design](https://github.com/driftsys/upskill/blob/main/docs/wip/specs/2026-10-12-m1-owned-lifecycle-design.md).

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
values and records the exact validated supplied manifest digest as source
identity; that digest does not establish provenance or identify generated
content.

Compilation first validates every manifest path, ordering, digest and mode,
including files later excluded or ignored. File/descendant pairs such as `a` and
`a/b` fail even when ignored. Requested clients must be explicit, nonempty and
unique. The canonical root entrypoint for the requested kind must exist, contain
UTF-8 and parse as its existing typed frontmatter with a supported schema. The
folder, resolved source name and optional alias must each be one portable
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
unknown-client suffixes, hidden files, empty files and binary files remain
resources unless ignored. A root override for the requested kind and an emitted
client fails with an unsupported-override error even if ignored. Overrides for
other kinds or non-emitted clients remain excluded source inputs. Body override
precedence is deferred.

Each included output contains exactly one rendered entrypoint and every retained
resource, sorted by portable UTF-8 relative path. Compilation reuses existing
renderers, directives, passthrough, formatting and frozen historical layout. For
flat outputs it rewrites copied-resource links using the source name, then
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
The separate mixed coordinator in
[§4.6](#46-mixed-owned-artifact-operation-plans) accepts explicit desired
ownership and observations; compilation does not produce those lifecycle inputs.
This prerequisite completes neither M1-A2 nor M1-A3.

See the
[compiler design](https://github.com/driftsys/upskill/blob/main/docs/design/captured-item-compilation.md),
[AD-0024](https://github.com/driftsys/upskill/blob/main/docs/adr/0024-pure-captured-item-compilation.md),
[AD-0025](https://github.com/driftsys/upskill/blob/main/docs/adr/0025-reject-active-captured-overrides.md)
and
[AD-0026](https://github.com/driftsys/upskill/blob/main/docs/adr/0026-separate-generated-permission-policy.md).

### 4.6 Mixed owned-artifact operation plans

The pure library module `operations::plan` composes the existing dedicated-file
and instruction planners into one complete installation operation. Its two new
value types derive `Debug`, `Clone`, `PartialEq` and `Eq`, without
serialization:

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
values. `ownership` supplies records and owners. The result reuses
`FileOperationPlan`, `PlannedFile`, `FileLocation` and `FileValue` from
`files::plan`; both physical forms therefore carry exact optional before and
after values.

A Set replaces the installation's complete desired set across both forms.
Omitted old files and contributions are removed through their respective
planners; a global Remove reaches every old form. Empty Sets require explicit
Remove. Duplicate changes, unknown removals, duplicate desired file locations or
contribution selectors, invalid owners and invalid consumers conflict. An
existing installation retains its source, item kind, source name and installed
alias even when the old and desired forms are disjoint. Its revision may change
coherently across all desired records. Any multi-owner record at an affected
location conflicts; untouched shared rows survive exactly.

Locations compare `(scope, raw root OsStr, portable relative path)`. Raw dot
components and project/user scopes remain distinct. Roots must be nonempty and
absolute; paths must use portable relative components. Supplied optional modes
must be within `0..=0o7777`. Every affected physical location requires exactly
one observation, including no-ops: the affected union includes every old row
belonging to a changed installation and every desired location. Missing,
duplicate or extra observations conflict, and all observations are validated
before membership checks.

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
contribution selectors. Final combined ownership is validated, including
revision coherence. Reordering changes, desired entries or observations does not
change the result. The API performs no I/O or input mutation; any failure
returns no successful partial plan. It may return the first contextual conflict
rather than all diagnostics.

This library prerequisite activates no CLI selector, native adapter, source
fetch, filesystem observation, lock, journal, schema-2 persistence or execution.
Captured-rule desired sets are implemented in §4.14. Remaining work includes
planner integration of the implemented reference authority and per-owner
consumer associations, immutable source evidence, target qualification,
filesystem safety, physical permission application and metadata qualification,
writer locking, durable state and apply/recovery. Pure
[permission selection](../design/file-permission-policy.md) is implemented as a
separate inactive supplied-value capability. It does not complete M1-A2, M1-A3
or M1. The public declarations are in
[`operations::plan`](https://github.com/driftsys/upskill/blob/main/src/operations/plan.rs).

### 4.7 Qualified scope roots and advisory writer locking

`filesystem::QualifiedScope::open(&InstallationRoot)` pins an existing absolute
scope directory on Linux and macOS. Parent components, missing or non-directory
roots and final symlinks are rejected. Dot components and trailing separators
are normalized before final symlink checks. Ancestor aliases are permitted
during explicit selection; native non-UTF-8 paths are preserved where the
underlying filesystem accepts them. Filesystem refusals remain contextual
errors.

`verify_binding()` revalidates the normalized selected spelling, canonical path
and directory device/inode identity. A renamed, replaced or symlink-substituted
root, or retargeted ancestor alias, invalidates the old binding. The held handle
does not freeze the path namespace or establish descendant containment.

`try_lock_writer()` uses a fresh close-on-exec directory description and a
nonblocking exclusive advisory lock. It revalidates binding before and after
acquisition. Same-process and cross-process writers to one physical directory
contend, including aliases and different Project/User labels; distinct
directories remain independent. Contention returns a busy error. Other
filesystem failures remain errors. An opaque borrowing `ScopeWriterGuard`
retains exclusion until drop or holder termination; ordinary exec does not
retain it. Temporary fork inheritance is possible. No fork-safe lease, fairness,
network-filesystem guarantee or exclusion against external editors is promised.

These operations create no files, directories or persistent state and preserve
existing bytes and permissions. The API exposes no raw descriptor, cloning or
write operation. Other operating systems explicitly report unsupported behavior.
This library prerequisite activates no CLI command, schema change, lifecycle
execution, descendant containment, observation or recovery. It does not complete
the M1 filesystem stage or Windows qualification.

See [AD-0028](../adr/0028-lock-qualified-scope-directories.md) and the
[qualified scope-lock design](../design/qualified-scope-locking.md) for
rationale, acceptance examples and platform evidence.

### 4.8 Qualified destination observations

`QualifiedScope::observe_destinations(&[FileLocation])` captures exact optional
`FileValue` values beneath the borrowed physical scope. Regular files return
binary bytes and Unix permission bits (`Some(mode & 0o7777)`); absent
destinations return `None`. Observations preserve the supplied native roots and
portable paths, sorted by scope, raw root spelling and relative path. Every
supplied root must independently qualify and bind to the same physical
directory, including ancestor-alias spellings and different Project/User labels.
Empty input still validates the scope and filesystem profile.

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
identity, type, links, size, mode, modification/change times (including
fractional seconds) and byte count around each read. Before returning, and again
from `verify_unchanged()`, it checks fresh bindings and directory identities,
exact stored spellings, mounts, exact bytes, file stamps and absence evidence.
These checks never refresh stale expectations. Timestamp-only changes, edits
with bytes and mtime restored but changed ctime, same-size edits with restored
mtime, chmod, identical-byte inode replacement, ancestor replacement,
missing-to-present transitions and root-alias retargeting invalidate evidence.
Access times and unrelated directory timestamp changes do not invalidate it.
Retained handles close on drop and returned-error paths; process abort or
termination does not guarantee Rust destructors run.

Observation creates no files, directories, locks or persistent state and
acquires no writer guard. Plain observations convey no execution or permission
authority. These sampled checks do not exclude external writers or hostile
ancestor relocation. A future executor must retain its writer guard and
independently establish complete lifecycle inputs, authority, permission policy,
immediate pre-effect checks and recovery. Existing commands and schema 1 remain
unchanged; Windows qualification and lifecycle execution remain pending. Hosted
Linux runtime qualification and Windows unsupported-boundary checks passed on
2026-10-13 for PR head `765e70b9b7ce5386530ffea1fe9814b228724428` (merge
checkout `242930ef9107aff6eff432484f474e47157983f2`) in
[Actions run 38330080515](https://github.com/driftsys/upskill/actions/runs/38330080515).
That evidence does not establish a qualified Windows backend or cover later
local timestamp and special-bit tests. Independent regression controls now cover
fractional mtime changes, metadata changes during acquisition, short reads with
unchanged descriptor metadata, unknown native profiles and different retained
mount bindings through the production qualification and verification paths.
Existing-file overlap and repeated stale verification are also pinned. Linux's
missing returned mount-ID mask production-path case passed natively on Ubuntu in
[Actions run 38334882688](https://github.com/driftsys/upskill/actions/runs/38334882688)
on 2026-10-13 for PR #273 head `2a50ca6a5163e11912cbcee7987593c68e0a4197`.
Ubuntu job `115092610337` passed 28 public destination cases; four native
profile/mask check and production-call omissions failed their intended
assertions. Each restored `qualification.rs` to SHA-256
`1c5b288ed575ae1ccdd99a10170c13aa9054f92004a0927a64fb1587bac40bd5`, followed by
22 restored private cases GREEN. Local APFS separately passed 28 public and 22
private cases plus 14 independent native omission controls. All 13 head checks
succeeded, including Windows unsupported-boundary job `115092610428`. This dated
evidence covers that head; hosted CI for the final head after temporary harness
removal remains required before merge. Injected query results and expected mount
bindings establish neither real privileged mount or fault behavior nor a
qualified Windows backend. See
[AD-0029](../adr/0029-capture-qualified-destination-evidence.md) and the
[destination observation design](../design/qualified-destination-observation.md).

### 4.9 Qualified source tree observations

The inactive library boundary exposes
`QualifiedScope::observe_source_tree(&self) -> anyhow::Result<SourceSnapshot<'_>>`.
An opaque, non-Clone snapshot borrows exactly one independently selected
qualified source scope. Its `manifest(&self) -> &SourceManifest` accessor
borrows plain captured values, and
`verify_unchanged(&self) -> anyhow::Result<()>` checks the original evidence
without refreshing it. There is no public constructor, mutable accessor,
serialization, write API or implicit writer lock. Debug output contains no
captured content. Unsupported hosts expose the type and return explicit errors.

Capture covers every descendant: hidden and ignored names, .git contents, binary
and empty files, and empty directories. Names must be UTF-8 and satisfy the
portable relative-path rules before recursion or reading. Hard-linked regular
source paths remain distinct manifest records; this does not change destination
single-link eligibility. Exact sorted directory membership, topology, identities
and stamps remain private evidence, including empty directories absent from the
manifest. Repeated directory identities, symlinks and special entries reject.

Every access is relative to a pinned directory descriptor and one exact stored
component. Inspection uses SYMLINK_NOFOLLOW; opens use NOFOLLOW and CLOEXEC,
DIRECTORY for directories and NONBLOCK for regular entries. Selected/opened
identity and type must agree. The shared native qualification policy applies to
the root and every opened descendant, and requires one unchanged source mount
binding. The source mount may differ from any destination mount.

File acquisition compares metadata before open, immediately before read and
after read, then requires exact byte count and nonnegative observed size to
agree. Stamps retain device/inode, complete native mode, link count, size and
nanosecond mtime/ctime; atime is ignored. Directory stamps must remain stable
across enumeration and descendant capture. Permission evidence is
`Some(mode & 0o7777)`. Canonical assembly shares the existing strict-manifest v1
hash framing and pure validation, preserving legacy capture signatures,
diagnostics and digests.

Before returning, capture performs full revalidation. Every later check verifies
retained descriptors, reopens the bound root, acquires a fresh complete tree and
compares membership, identities, stamps, exact bytes and modes with original
evidence. The fresh tree is discarded. Both checks after one mutation must
reject and leave the manifest unchanged. Root and ancestor-alias rebinding,
same-byte inode replacement, same-descendant directory replacement,
file/empty-directory addition or removal, rename, mode and fractional timestamp
changes reject. Binding is checked again after the walk.

This read-only sampled boundary changes no CLI, schema, native adapter, source
fetch, lifecycle state, permission policy, application or recovery behavior.
Recoverable failures retain operation/path context and original causes and
return no partial snapshot. Ordinary infallible allocation may abort; OS
termination is outside Result guarantees. Checks provide neither an atomic
snapshot nor exclusion against external editors, relocation or hostile ABA
changes. Native Linux/macOS qualification uses existing profiles; Windows
qualification, privileged mounts/devices and physical I/O or exhaustion faults
remain separate evidence limits. Final-head hosted Linux/Windows gates remain
required. See [AD-0030](../adr/0030-capture-qualified-source-evidence.md) and
the [source observation design](../design/qualified-source-observation.md).

Local APFS source coverage at `a393263` included 12 public acceptance tests and
31 private tests, with independent regular-descendant profile/mount
qualification before reading, complete permission-bit mapping, held-writer
capture/revalidation and non-content Debug checks. Independent link-count and
ctime omission isolation remains a coverage limit; both fields participate in
production stamps.

Scoped revalidation diagnostic tests retain source-root/operation context,
complete relative paths in tree comparisons and original causes, including
repeated public rejection without refreshing the manifest. Public root and
ancestor binding failures retain source-root/verification context and the
original scope cause. Retained nested-file diagnostics are independently pinned
with unchanged parent stamps and membership. A final-binding control now rebinds
an ancestor only after fresh capture/comparison and independently pins the last
revalidation check. Empty-root and directory-only positive fixtures exercise
qualified capture and revalidation of valid empty manifests.

On 2026-10-13,
[CI run 38346202172](https://github.com/driftsys/upskill/actions/runs/38346202172)
and
[CodeQL run 38346200154](https://github.com/driftsys/upskill/actions/runs/38346200154)
succeeded for PR head `a3932633e9cc9a269f8ba8d8b6dd2ec26d0b828e`. The tested
merge commit `604dcda92bbc3c08d4ce10b695b18f95fb4d5e83` and that PR head share
tree `48b716136a7822428e6ed163be2689beaaf9ddfa`. Native Linux job
[115128098194](https://github.com/driftsys/upskill/actions/runs/38346202172/job/115128098194)
executed the source missing mount-ID mask and non-UTF-8 cases. Windows job
[115128098069](https://github.com/driftsys/upskill/actions/runs/38346202172/job/115128098069)
executed scope rejection and compiled the opaque source API; its uncalled source
helper did not establish source-method runtime coverage. A new private
unsupported-host test now calls the source method and requires contextual
rejection. Its native Windows execution and all hosted checks for the eventual
final PR head remain merge gates. No qualified Windows backend is implemented.

### 4.10 Explicit file permission selection

`files::permissions::select_file_mode(destination, source, resource_execution)`
is an inactive pure library API returning `anyhow::Result<u32>`. Its
supplied-value inputs are `DestinationMode::{Absent, Present(Option<u32>)}`,
`PermissionSource::{Generated, Resource { captured_mode: Option<u32> }}` and
`ResourceExecutionPolicy::{Disabled, OwnerExecutable}`. These types derive
`Debug`, `Clone`, `Copy`, `PartialEq` and `Eq`; they carry no serialized state
or effect capability.

Every supplied destination and captured resource mode must be within
`0o0000..=0o7777`, including evidence unused by the selection branch. Errors
identify destination or captured resource evidence. When multiple inputs are
invalid, no particular diagnostic ordering is promised. `Present(None)` rejects
as unknown existing mode. Existing modes containing any `0o7000` special bit
reject as unsupported application policy; ordinary existing modes are returned
exactly, including `0o000`, `0o400`, `0o640`, `0o755` and `0o777`, under either
resource policy.

Absent generated output selects literal `0o600` under either policy. Absent
resources also select `0o600` when execution is `Disabled`, including unknown
captured mode. `OwnerExecutable` requires a known captured resource mode and
selects `0o700` only when its owner-execute bit `0o100` is set; otherwise it
selects `0o600`. Captured `0o755` and `0o4777` therefore select `0o700`;
`0o011`, `0o644`, `0o1000`, `0o2000` and `0o4000` select `0o600`. Source
group/other and special bits are never copied. Unknown captured resource mode
rejects only when selecting new resource execution; an existing ordinary
destination mode remains exact.

The function reads no environment or process umask and performs no I/O. Supplied
values and selected bits convey no source authenticity, ownership, physical
qualification or execution authority. Future writers must separately qualify
owner/group, ACLs, extended metadata, containment and atomic replacement and
persist actual resulting mode evidence. This API changes no compiler output,
planner validation, CLI behavior or lockfile schema. The opt-in executor in
§4.12 supplies file application, journals and recovery. Combined lifecycle
state, activation and hosted qualification remain pending; M1-A2, M1-A3 and M1
remain incomplete. See
[AD-0031](../adr/0031-select-explicit-file-permissions.md) and the
[permission design](../design/file-permission-policy.md).

### 4.11 Ordered recovery checkpoint model

`operations::recovery::plan_recovery(&[RecoveryAction])` is an inactive pure
library API. Supplied File and State actions contain Queued/Pending/Done
checkpoints and Before/After/Both/Neither relations to exact expected values.
Exactly one State must be final. Empty inputs reject; state-only operations are
valid. The complete checkpoint order must match `Done* Pending? Queued*` before
observations are classified. Structural errors identify the required State
condition or invalid position.

Done requires After or Both; Pending accepts Before, After or Both; Queued
requires Before or Both. Every observation is checked, including the suffix. The
first conflict returns Reconcile with its index and both prefixes.
`recorded_prefix` preserves the count of leading Done checkpoints even when
contradicted. `verified_post_prefix` counts consecutive leading Done post-state
observations, extended through one Pending After or Both, and stops at the first
mismatch. Queued never extends it.

When all actions are Done, Complete includes the final State. Otherwise Ready
returns one step at the first unfinished index and both prefix counts. Pending
Before and Queued Before propose Apply; Pending After or Both proposes
Acknowledge; Queued Both proposes CheckpointOnly. Apply carries no physical
write authority, and the function changes no checkpoint. Public model types
derive Debug, Clone, Copy, PartialEq and Eq.

Supplied classifications are not evidence capabilities. The API performs no I/O,
serialization, journal reading, qualification or effects. The opt-in executor in
§4.12 supplies qualified file application, receipts and recovery using this
model. No lifecycle caller or schema-2 writes are activated; M1-A2, M1-A3 and M1
remain incomplete. See
[AD-0032](../adr/0032-model-recovery-checkpoints-before-effects.md) and the
[recovery model](../design/file-recovery-model.md) for a public API example.

### 4.12 Qualified file execution and recovery

`filesystem::ExecutionRequest` supplies a complete ordered sequence of
`PlannedFile` values and one distinct final State file that remains present. Its
effect-free validator rejects invalid or cross-scope paths, duplicate or
ancestor destinations, reserved journal paths, unknown/special permission bits,
changed existing ordinary modes, and new-file modes other than selected 0o600 or
0o700. The caller remains responsible for ownership and complete planning.

`QualifiedScope::apply_operation` requires its matching live writer guard and
verifies all supplied source snapshots and exact destination preconditions
before the first output effect. `recover_operation` requires the same authority
and uses validated persisted payloads without fetching sources. Both are opt-in
library methods; current CLI commands and schema-1 lockfiles do not use them.
Unsupported operating systems reject explicitly. Supported Linux/macOS paths
retain the existing qualification requirements.

Effects replace files through exclusive sibling staging, file sync, atomic
rename and parent-directory sync, or unlink individual planned files and sync
their parents. Rechecks precede each effect. Unchanged files retain identity;
directories, extra files and empty shared aggregates are retained. Existing
ordinary modes are preserved. Replacement verifies unchanged UID/GID. ACLs,
special bits, Darwin file flags and unsupported extended attributes reject;
Darwin provenance is allowed only with an equal value on prior and staged files.

The private `<scope-root>/.upskill/state/` store binds root and directory
identities. Its directory is 0o700 and private files are 0o600. The root and
store parent must not allow group/other writes. Strict execution-journal schema
1 records the complete ordered actions, before/after digests, lengths and modes,
bound desired payloads, final State, generations and Queued/Pending/Done
receipts. Invalid paths, unsupported records, payload changes and mismatched
storage bindings reject without output effects. This journal schema does not
activate a new lifecycle lockfile schema.

Desired payloads are synced before initial journal publication. Each generation
uses staged-file sync, atomic publication and directory sync. Persist Pending
before each effect, sync and reobserve its post-state, then persist Done. A
Pending post-state is resynced, including its parent directory, before recovery
acknowledges it; it is not rewritten. A Queued post-state or unexpected value
requires reconciliation. No-op actions use receipt-only checkpoints. Final State
uses the same protocol. Reports distinguish the recorded Done prefix from the
currently verified post-state prefix.

Completed cleanup is durably marked before removing validated payloads. The
completed journal is removed last; conflicting or unknown files are preserved.
Read-only observation creates no execution artifacts. Recovery with no published
operation performs no output work. Interrupted initial setup may retain private
orphan files rather than deleting content without established authority.

Advisory locking coordinates cooperating writers, not all external edits;
rechecks cannot exclude every namespace race. Operations are not atomic across
files. Process interruption coverage does not establish hardware power-loss
durability. Hosted Linux/macOS qualification remains pending, so gate 4, M1-A2,
M1-A3 and M1 are not declared complete. See
[AD-0033](../adr/0033-qualified-file-execution-and-journals.md) and the
[qualified execution design](../design/qualified-file-execution.md).

### 4.13 Combined lifecycle state foundation

The opt-in `lifecycle::state` library validates a schema-2 installation and
reference graph without I/O. Direct references root the graph; bundle members
and dependencies must resolve, be reachable and contain no dependency cycle.
Installations retain requested surfaces even when no artifact is generated.
Duplicate identities and inconsistent source revision evidence are rejected.
Artifact owners and roots must match authoritative installations. Explicit
per-owner consumer associations must match every artifact selector exactly;
their union must equal the artifact consumers. The strict schema-2 codec
validates before parsing results are returned and before serialization. It emits
deterministic pretty JSON with one trailing newline without mutating input.
Typed schema-1 projection creates no owners and preserves historical rows,
labels and statuses in their original order; exact JSON bytes and unknown legacy
fields require the later migration backup.

This library does not change `lockfile::CURRENT_SCHEMA`, command dispatch or
production lockfiles. It does not establish filesystem ownership or native
support. See
[combined lifecycle state](https://github.com/driftsys/upskill/blob/main/docs/design/combined-lifecycle-state.md).

### 4.14 Captured rule desired sets

The opt-in `lifecycle::desired::compile_rule_desired_set` API validates a
complete captured rule source and its explicit installation/source/consumer
binding, then projects every existing compiler output into a complete desired
file set. Exact bytes, resource modes, aliases, raw roots and explicit consumers
survive. Requested surfaces remain recorded after audience exclusion, including
when the desired set is empty. Empty output is not an installation removal.

Consumers must exactly account for requested surfaces and match the explicitly
selected Claude, Copilot or OpenCode compiler family. No Codex binding or native
conformance claim is introduced. Active scoped OpenCode output is rejected
rather than silently losing nonempty SSOT `scope.paths`.

These are pure supplied-value results with no I/O, ownership adoption, lockfile
rewrite or production command activation. Physical qualification, permission
selection, reference/shared-owner reconciliation and application remain
separate. See
[captured rule desired sets](https://github.com/driftsys/upskill/blob/main/docs/design/captured-rule-desired-sets.md).

`compile_rule_desired_sets` returns all complete sets in request order only when
all requests compile and pass preflight. Otherwise `Blocked` contains contextual
request-index/installation-ID diagnostics and no successful subset. Duplicate
installation IDs and equal or ancestor/descendant selected paths under the same
scope and raw root spelling conflict, including equal bytes. Diagnostics sort by
request indices then message. Empty input returns `Ready([])`. Different raw
roots and scopes remain separate; physical aliases and case equivalence require
later qualification. Each failed compilation contributes one contextual error
rather than exhaustive diagnostics within that rule.

### 4.15 Authoritative lifecycle reference reconciliation

The inactive `lifecycle::reconcile::reconcile_references` API accepts validated
combined state and explicit installation/bundle upserts plus reference edits. It
returns graph records and removed IDs without I/O or input mutation. Only actual
direct roots and transitive bundle edges retain nodes; empty output or
artifact/consumer count does not remove a referenced installation.

All supplied nodes and edges are validated before iterative pruning, including
unreachable dangling edges and cycles. Explicit upserts must remain reachable.
Duplicate, absent, already-existing or overlapping reference edits conflict.
Existing installation IDs cannot change source/kind/name/alias or scope/raw
root; existing bundle IDs retain source/name. Valid evidence/revision and
requested surface updates remain subject to later complete artifact planning.

Surviving rows retain order; new nodes and references append in ID/enum order.
Removed IDs follow prior order. Graph results contain neither artifacts nor the
legacy snapshot and do not authorize deletion or state persistence. Later
reconciliation must preserve tagged legacy data and unrelated owners, then
validate the complete final combined state. No CLI or schema-1 behavior changes.
See
[lifecycle reference reconciliation](https://github.com/driftsys/upskill/blob/main/docs/design/lifecycle-reference-reconciliation.md).

## 5. Source format and authentication

Source format: see [§2.1](#21-add).

upskill never injects credentials into clone URLs. Clones use the bare
`https://<host>/...` URL and rely entirely on git's own configuration —
credential helpers, `url.<base>.insteadOf` rewrites, and SSH. A private
repository works whenever a manual `git clone <url>` would; configure git the
same way. upskill resolves no tokens and reads no token env vars.

Any https host is treated as an opaque git remote
(`https://git.mycompany.com/team/repo`), including projects nested under groups
to any depth (`https://git.mycompany.com/group/subgroup/team/repo`).

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
- Global `-q` / `--quiet` suppresses informational stdout. Errors on stderr and
  exit codes are unaffected.
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
- User guide:
  [Getting started](https://driftsys.github.io/upskill/getting-started.html),
  [Commands](https://driftsys.github.io/upskill/commands.html),
  [Recipes](https://driftsys.github.io/upskill/recipes.html)
- Agent Skills open standard: <https://agentskills.io>
- skills.sh ecosystem: <https://skills.sh>
