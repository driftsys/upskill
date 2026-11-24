# Command item kind (`COMMAND.md`): implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

Date: 2026-11-24. Story: [#632](https://github.com/driftsys/upskill/issues/632).

**Goal:** A bundle lists command items (`COMMAND.md`) under `items.commands`;
`lint`, `fmt` and `new` know the kind, `add` and `update` skip it with a count
on stderr, and `pack` renders each command, and the `init` command, as a skill
that only the user calls.

**Architecture:** A command is a new model type (`model::Command`), not a fourth
`pipeline::ItemKind`: `ItemKind` is serialized in the lockfile and selects
install output paths, and the install pipeline must not learn the kind. The
author commands (`lint`, `fmt`, `new`) and pipeline discovery learn the entry
file. `add` and `update` count the command items of the resolved bundles through
a `PackOnlyKind` enum, so story #631 adds `Hook` to that enum without a
redesign. `pack` reads the commands of the closure through a new pipeline
function and renders them with a new `generate::render_command`.

**Tech stack:** Rust 2024, `serde`, `serde_yaml_ng`, `anyhow`; integration tests
with `assert_cmd` through `common::upskill_cmd`.

**Spec:** [plugin MVP design](../specs/2026-11-24-plugin-mvp-design.md),
sections 1 (the `add`/`update` skip and the kind lists), 2, 3 and "Left to the
builder"; [AD-0046](../../adr/0046-plugin-mvp-components.md), Decisions 1, 2, 8
and 11.

## Global constraints

- Entry file `COMMAND.md`; bundle key `items.commands`.
- Frontmatter: `name` (required, equal to the directory name), `description`
  (required), `argument-hint` (optional).
- Claude Code output: `skills/<name>/SKILL.md` with the frontmatter keys `name`,
  `description`, `argument-hint` (when set), `disable-model-invocation: true`,
  in this order. Copilot CLI output: the same path with `name`, `description`,
  `disable-model-invocation: true`. Co-located resources are copied next to it.
- `init` is rendered by the same renderer to `skills/init/SKILL.md`.
  `commands/init.md` is no longer written. No fallback, no migration.
- `pack` errors: a command and a skill with one name in the closure; a command
  named `init` in a closure with rules.
- `pipeline::ItemKind`, the lockfile, `conflict.rs`, `support.rs` (the table)
  and `--kind` of `add` do not learn the kind.
- Only `main.rs` writes to stdout and stderr. Library code returns data.
- Zero warnings: `just verify` and `just sanity` pass.
- Comments, documentation and messages use plain, literal English. Markdown is
  wrapped at 80 columns.
- Do not edit accepted ADRs (AD-0044 still names `commands/init.md`; AD-0046
  amends it).

## Plan decisions

Each decision is marked "plan decision" where a task uses it.

1. `--kind command` on `add` keeps the existing usage error
   `unknown item kind: command` (exit 2). It is the simplest consistent
   behaviour: `--kind` selects installed kinds, and commands are never
   installed.
2. `Command` accepts only `schema`, `name`, `description`, `argument-hint`,
   `license`, `metadata` and `ignore` (`#[serde(deny_unknown_fields)]`). Fields
   such as `audience`, `requires` or `allowed-tools` would be dropped silently
   by `pack`, so they are a `frontmatter` lint error. `ignore` is kept because
   resources are copied "as for a skill", and a skill honours `ignore`.
3. Lint rule id `command-with-skill` (error), reported on the `COMMAND.md` path.
   The `name-collision` rule does not learn commands: `add` resolves a name that
   matches a bundle and a command directory as the bundle.
4. `new command <name>` may co-locate with `RULE.md` and `AGENT.md`, never with
   `SKILL.md`, in either order.
5. The skip line is
   ``skipped N command item(s): only `upskill pack` writes them``, with the
   literal `(s)`. Each pack-only kind is one enum value with a label that holds
   the full noun (`command item(s)`); the parts before the colon are joined with
   `,`, so story #634 gives for example
   ``skipped 2 command item(s), 1 LSP server(s): only `upskill pack` writes them``.
   The skip count covers the command items of the resolved bundles only. A
   whole-source `add` without a bundle ignores command directories without a
   line. No line is printed when the run has a kind restriction (the `init`
   command runs `add --kind rule`), or with `--quiet`.
6. `update` reports the skipped commands of every source it prepares, applied or
   not, also with `--dry-run`.
7. The named-command check runs only for the positional names of `add` with a
   directory source. It is gated on `!options.keep_recorded_excludes` (the flag
   that the existing docs define as "only the positional names of `add`";
   `update` sets it to `true`) AND `!discovery::is_bundle_file(&local_source)`.
   For a `*.bundle.yaml` source, `find_command_dir` would call `fs::read_dir` on
   the file and fail, while today
   `upskill add <reg>/team.bundle.yaml release-notes --project --claude` exits
   0.
8. `pack` name checks are made on the bundle lists of the closure, so they give
   the same result for every client flag: a command named by two bundles; a
   command and a skill with one name; a command or a skill named `init` when any
   bundle of the closure lists a rule.
9. `init` gets `name: init` and `disable-model-invocation: true` in both
   clients, because it goes through the command renderer.
10. The content format gets a new section 3.9 instead of a renumbered 3.5, so
    the anchors of `docs/specification/format-spec.md` stay valid.
11. The support table in `src/support.rs` gets no command row; the support
    levels page describes the kind in its "Bundle plugin output" section.
    `is_kind_dir` (`skills`, `rules`, `agents`) and `src/index.rs` do not learn
    the kind.
12. `--exclude <command-name>` stays an unknown exclude (exit 2): commands are
    never installed, so there is nothing to exclude.

## Review focus

1. `add <source> <bundle> --kind rule` (the command that `init` runs) prints no
   skip line (test in Task 2).
2. `add <source> alpha release-notes` with one skill name and one command name
   fails before any write (test in Task 2).
3. A command directory that also holds a `RULE.md` and a file matched by its
   `ignore`: neither is copied into the command skill (test in Task 3).
4. `@client` directive blocks in a command body are processed for each client
   (test in Task 3).
5. A `pack` that fails on a command name check writes nothing below `--out`
   (assertion in each Task 3 error test).

---

### Task 1: model, `lint`, `fmt` and `new command`

**Files:**

- Create: `src/model/command.rs`
- Modify: `src/model/mod.rs`, `src/model/bundle.rs` (`BundleItems`),
  `src/bundle.rs` (test helper literal), `src/parse/bundle.rs` (test),
  `src/lint.rs`, `src/fmt.rs`, `src/scaffold.rs`, `src/main.rs`
  (`print_scaffold_report`), `src/cli.rs` (`New` doc and examples)
- Test: `tests/cli_lint.rs`, `tests/cli_fmt.rs`, `tests/cli_new.rs`
- Docs: `docs/topics/0001-content-format.md`,
  `docs/specification/format-spec.md`, `docs/specification/specification.md`,
  `docs/book/commands.md`, `docs/book/conventions.md`, `docs/book/intro.md`,
  `AGENTS.md`

**Interfaces:**

- Produces `model::Command` (re-exported from `src/model/mod.rs`):

  ```rust
  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  #[serde(deny_unknown_fields)]
  pub struct Command {
      #[serde(default)]
      pub schema: SchemaVersion,
      pub name: String,
      pub description: String,
      #[serde(default, rename = "argument-hint", skip_serializing_if = "Option::is_none")]
      pub argument_hint: Option<String>,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub license: Option<License>,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub metadata: Option<Metadata>,
      #[serde(default, skip_serializing_if = "Vec::is_empty")]
      pub ignore: Vec<String>,
  }
  ```

- Produces `BundleItems.commands: Vec<String>` (same serde attributes as
  `agents`); `BundleItems::is_empty` also requires `commands` to be empty.
- Produces `scaffold::NewKind::Command` (`parse("command")`, entry
  `COMMAND.md`).

- [ ] **Step 1: Write the failing tests**

  `tests/cli_lint.rs` (each writes into a temporary registry and runs
  `upskill lint --strict <dir>`):

  - `lint_accepts_a_command_with_argument_hint`: `release-notes/COMMAND.md` with
    `schema: 1`, `name: release-notes`, `description: Write the release notes`,
    `argument-hint: "[version]"` and the body
    `\n## Steps\n\nWrite the notes for $ARGUMENTS.\n`; assert success.
  - `lint_flags_a_command_without_name_as_error`: no `name`; assert exit 1 and
    stdout contains `[frontmatter]` and `` missing field `name` ``.
  - `lint_flags_a_command_name_that_differs_from_its_directory`: `foo/` with
    `name: bar`; assert exit 1 and stdout contains
    `` command `name: bar` must equal directory `foo` ``.
  - `lint_flags_a_command_and_a_skill_in_one_directory`: `both/COMMAND.md` and
    `both/SKILL.md`, each valid; assert exit 1 and stdout contains
    `[command-with-skill]` and
    `` a COMMAND.md and a SKILL.md cannot share the directory `both` ``.
  - `lint_flags_an_unknown_command_field_as_error`: `audience: [claude]`; assert
    exit 1 and stdout contains `` unknown field `audience` ``.

  `tests/cli_fmt.rs`:

  - `fmt_orders_command_keys_canonically`: frontmatter written in the order
    `argument-hint`, `description`, `name`, `schema`; after `upskill fmt <dir>`
    the frontmatter lines start with `schema:`, `name:`, `description:`,
    `argument-hint:` in this order; a second run leaves the bytes unchanged.

  `tests/cli_new.rs`:

  - `new_command_creates_command_md`: `new command release-notes`; the file
    `release-notes/COMMAND.md` contains `\nschema: 1\n`,
    `\nname: release-notes\n` and `description:`; `upskill lint --strict` on the
    directory succeeds; `upskill fmt` leaves the bytes unchanged.
  - `new_command_colocates_with_a_rule`: `new rule x`, then `new command x`;
    both succeed and both files exist.
  - `new_command_refuses_a_directory_with_a_skill`: `new skill x`, then
    `new command x`; exit 1, stderr contains
    `a COMMAND.md and a SKILL.md cannot share one item directory`.
  - `new_skill_refuses_a_directory_with_a_command`: the reverse order; same
    assertion.
  - `new_rejects_an_unknown_kind_and_lists_four_kinds`: `new bundle x`; exit 2,
    stderr contains `expected one of: rule, skill, agent, command`.

  Unit tests: `src/parse/bundle.rs` `load_parses_items_commands` (YAML
  `items:\n  commands: [release-notes]` gives
  `items.commands == ["release-notes"]` and `!items.is_empty()`);
  `src/model/command.rs` `command_parses_the_three_fields` and
  `command_refuses_an_unknown_field`.

- [ ] **Step 2: Run `cargo test --test cli_lint --test cli_fmt --test cli_new`
      and confirm that the new tests fail**

- [ ] **Step 3: Implement the model**

  `src/model/command.rs` as under Interfaces, with a module doc
  (`//! Command items (§3.9).`). Add `commands` to `BundleItems`, update the doc
  of `is_empty`. In `src/bundle.rs` the test helper `items(..)` builds
  `BundleItems { .., commands: vec![] }`.

- [ ] **Step 4: Implement lint**

  In `src/lint.rs`: add `FileKind::Command` (`"COMMAND.md"`); replace the three
  literal lists in `discover_self_entrypoints` and `discover_items` with
  `const ENTRYPOINTS: [&str; 4] = ["RULE.md", "SKILL.md", "AGENT.md", "COMMAND.md"];`;
  `parse_kind` parses `Command` and returns `Some(name)`;
  `check_name_matches_dir` checks a command as a skill, with the message
  `` command `name: {name}` must equal directory `{dir}` ``; `check_file` pushes
  for a `COMMAND.md` whose directory also holds `SKILL.md` one finding with
  `rule_id: "command-with-skill"`, `Severity::Error` and the message
  ``a COMMAND.md and a SKILL.md cannot share the directory `{dir}`: both take its name``.
  Add the rule to the table of the module doc.
  `check_bundle_item_name_collisions` and `detect_item_kind` are unchanged (plan
  decision 3).

- [ ] **Step 5: Implement fmt**

  In `src/fmt.rs`: `EntryKind::Command` for `COMMAND.md`;
  `const COMMAND_KEY_ORDER: &[&str] = &["schema", "name", "description", "argument-hint", "license", "metadata", "ignore"];`;
  `canonicalise` calls `canonicalise_item::<Command>`.

- [ ] **Step 6: Implement `new command`**

  In `src/scaffold.rs`: `NewKind::Command`; `parse` error text
  ``unknown kind `{other}` — expected one of: rule, skill, agent, command``;
  `entry()` returns `COMMAND.md`; add `("COMMAND.md", true)` to
  `ENTRYPOINT_SKILL_MATCH`; give `validate_existing_item_dir_for_coloc` a third
  parameter `kind: NewKind` and refuse, before the other checks, a command into
  a directory with `SKILL.md` or a skill into a directory with `COMMAND.md`,
  with
  `{dir}: a COMMAND.md and a SKILL.md cannot share one item directory: both take the directory name`
  (plan decision 4). The name check that `("COMMAND.md", true)` enables uses its
  own text for a `COMMAND.md`, not the skill text at `src/scaffold.rs` lines
  157-163:
  `` {path}: existing command entrypoint declares `name: {existing_name}` — a command must have the name of its directory `{name}` ``.
  `render` for a command:

  ```text
  ---
  schema: 1
  name: {name}
  description: TODO describe what this command does when the user calls it.
  ---

  ## Overview

  TODO write the command body. Replace this scaffold before publishing.
  ```

  Update the module doc. In `src/main.rs`, `print_scaffold_report` maps
  `NewKind::Command` to `command`. In `src/cli.rs`, the `New` kind doc reads
  ``One of `rule`, `skill`, `agent`, `command`.`` and the examples add
  `upskill new command release-notes`.

- [ ] **Step 7: Update the documentation**

  - `docs/topics/0001-content-format.md`: §1 "**Item**: a rule, skill, or agent.
    The three kinds…" names four kinds and adds a **Command** definition (a
    procedure that the user calls by name; `upskill add` and `upskill update` do
    not install it). Statements about how a command is rendered wait for Task 3.
    §2.1: the tree and the constraint "at least one entrypoint file (`RULE.md`,
    `SKILL.md`, or `AGENT.md`)" and "Kind is determined by the entrypoint
    filename: …" add `COMMAND.md`; add the constraints that a command `name` is
    required and equals the directory name, and that a `COMMAND.md` and a
    `SKILL.md` MUST NOT share a directory. §3 intro "Item entrypoint files
    (RULE.md, SKILL.md, AGENT.md)" adds `COMMAND.md`. §3.1 "Common fields (all
    kinds and bundles)": state that a command requires `name` and rejects
    `audience` and `requires` (see §3.9). §3.1 field semantics and §8.1 "an item
    (rule, skill, or agent) without a `schema` field" add command. §3.7 table:
    new row `items.commands` (`string[]`, no, "Command names (§3.9).") and the
    example bundle stays. §3.8 key-order table: new row `Command` with the order
    of `COMMAND_KEY_ORDER`. New section `### 3.9 Command-specific fields` after
    §3.8 (plan decision 10): the example of design section 2, the fields table
    (`name` YES, `description` YES, `argument-hint` no), the accepted common
    fields (`schema`, `license`, `metadata`, `ignore`), that any other field is
    an error, and that `$ARGUMENTS` in the body stands for the text that the
    user gives.
  - `docs/specification/format-spec.md`: an anchor
    `<a id="39-command-specific-fields"></a>` and a link item after 3.8.
  - `docs/specification/specification.md`: §1.1 "Three item kinds" becomes "Item
    kinds" with a row for `Command` (purpose: "A procedure that the user calls
    by name.", entry file `COMMAND.md`); §1.3 "Holds the canonical `RULE.md` /
    `SKILL.md` / `AGENT.md` files" adds `COMMAND.md`; the `new` row of the §2
    table ("Scaffold a new rule, skill, or agent.") adds command; §2.5 "the
    `<kind>` is one of `rule`, `skill`, `agent`" adds `command`.
  - `docs/book/commands.md`: the `new` row of the top table; section
    `upskill new`: the `<kind>` list, one example line, and in "Co-location"
    that a command and a skill cannot share a directory; section `upskill lint`:
    "Seven rules" becomes "Eight rules" with a row for `command-with-skill`
    (severity error, source format-spec §2.1).
  - `docs/book/conventions.md`: "**Holds all three kinds.** … carries rules and
    agents as well" names all four kinds.
  - `docs/book/intro.md`: the list of SSOT items adds `COMMAND.md`.
  - `AGENTS.md`: module layout line of `model/` reads
    `Rule, Skill, Agent, Command, Bundle + common`.

- [ ] **Step 8: Run `cargo test` and confirm that all tests pass**

  The implementer does not commit. The orchestrator runs `just fmt`,
  `just verify` and `just sanity`, and commits with
  `feat(lint): add the command item kind to lint, fmt and new (#632)`.

### Task 2: `add` and `update` skip command items

**Files:**

- Modify: `src/model/bundle.rs`, `src/model/mod.rs`,
  `src/pipeline/discovery.rs`, `src/pipeline/prepared.rs`,
  `src/pipeline/report.rs`, `src/pipeline/mod.rs` (`install_with_lockfile`),
  `src/pipeline/lifecycle.rs` (`update`), `src/main.rs` (`run_add`,
  `run_update`)
- Create: `tests/cli_pack_only_items.rs`
- Docs: `docs/book/commands.md`, `docs/specification/specification.md`,
  `docs/topics/0001-content-format.md`,
  `skills/upskill-writing-bundles/SKILL.md`, `AGENTS.md`

**Interfaces:**

- Consumes: `BundleItems.commands` (Task 1).
- Produces in `src/model/bundle.rs`, re-exported from `src/model/mod.rs`:

  ```rust
  /// An item kind that `upskill pack` renders and `add` and `update` skip.
  #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
  pub enum PackOnlyKind { Command }
  impl PackOnlyKind {
      pub const ALL: [PackOnlyKind; 1] = [PackOnlyKind::Command];
      pub fn label(self) -> &'static str; // "command item(s)"
  }
  impl BundleItems { pub fn pack_only(&self, kind: PackOnlyKind) -> &[String]; }
  /// Each `(kind, name)` of `bundles` once, sorted.
  pub fn pack_only_items(bundles: &[Bundle]) -> Vec<(PackOnlyKind, String)>;
  ```

- Produces `InstallReport.pack_only_skipped: Vec<(PackOnlyKind, String)>` and
  `UpdateReport.pack_only_skipped: Vec<(PackOnlyKind, String)>`.
- Produces
  `PreparedInstall::pack_only_skipped(&self) -> Vec<(PackOnlyKind, String)>`:
  empty when `options.kinds` is not empty; else `pack_only_items` over the
  bundles of `resolved_bundles`, or over `report.bundles` when
  `resolved_bundles` is empty.
- Produces in `src/pipeline/discovery.rs`:
  `pub(super) fn find_command_dir(root: &Path, name: &str) -> Result<Option<PathBuf>>`,
  the first directory of `iter_item_dirs(root)` whose folder name is `name` and
  that holds `COMMAND.md`. Task 3 uses it.

- [ ] **Step 1: Write the failing tests in `tests/cli_pack_only_items.rs`**

  Header doc comment: ATDD tests for the command items that `add` and `update`
  skip. A helper writes the registry `<tmp>/registry/skills/` with
  `alpha/SKILL.md`, `style/RULE.md`, `release-notes/COMMAND.md`,
  `standup/COMMAND.md` and `team.bundle.yaml` (`skills: [alpha]`,
  `rules: [style]`, `commands: [release-notes, standup]`), and a consumer
  `<tmp>/consumer` with a `.git` directory. Every command runs through
  `common::upskill_cmd(&home)` with `--project`. `LINE` is
  ``skipped 2 command item(s): only `upskill pack` writes them``.

  - `add_of_a_bundle_skips_its_command_items_and_reports_the_count`:
    `add <registry> team`; success; stderr contains `LINE`;
    `.claude/skills/alpha/SKILL.md` exists; no path under the consumer contains
    `release-notes` or `standup`; `.upskill-lock.json` does not contain
    `release-notes`.
  - `add_reports_one_skipped_command_item`: a second bundle `solo.bundle.yaml`
    with `skills: [alpha]`, `commands: [standup]`; stderr contains
    ``skipped 1 command item(s): only `upskill pack` writes them``.
  - `add_of_a_bundle_file_with_a_positional_name_still_succeeds`:
    `add <registry>/skills/team.bundle.yaml release-notes --claude`; success
    (exit 0, as on main at 924e0fe); stderr does not contain `is a command item`
    (plan decision 7).
  - `add_with_quiet_prints_no_skip_line`: `--quiet`; stderr does not contain
    `skipped`.
  - `add_with_a_kind_restriction_prints_no_skip_line`: `--kind rule`; stderr
    does not contain `command item`.
  - `add_of_a_named_command_item_is_an_error_that_names_pack`:
    `add <registry> release-notes`; exit 1; stderr contains
    ``"`release-notes` is a command item; `upskill add` does not install command items. Use `upskill pack` to build a plugin that holds it"``;
    `.upskill-lock.json` does not exist.
  - `add_of_a_skill_and_a_command_by_name_fails_before_any_write`:
    `add <registry> alpha release-notes`; exit 1; same text; `.claude` does not
    exist.
  - `add_of_a_command_item_directory_as_source_is_an_error_that_names_pack`:
    `add <registry>/skills/release-notes`; exit 1; stderr contains
    `is a command item directory (it holds COMMAND.md)` and `upskill pack`.
  - `add_kind_command_is_a_usage_error`: `add <registry> team --kind command`;
    exit 2; stderr contains `unknown item kind: command` (plan decision 1).
  - `add_of_a_whole_source_ignores_command_directories`: a registry without a
    bundle file (`alpha/SKILL.md`, `release-notes/COMMAND.md`);
    `add <registry>`; success; stderr does not contain `command item`.
  - `update_reports_the_skipped_command_items_of_a_recorded_bundle`: after
    `add <registry> team`, `update` and `update --dry-run` each succeed and
    print `LINE` on stderr (plan decision 6).
  - `update_of_a_kind_restricted_bundle_prints_no_skip_line`: after
    `add <registry> team --kind rule`, `update` prints no `command item`.

  Unit tests: `src/model/bundle.rs`
  `pack_only_items_lists_each_command_once_in_sorted_order` (bundles with
  `commands: [b, a]` and `[a]` give `[(Command, "a"), (Command, "b")]`);
  `src/pipeline/discovery.rs`
  `find_command_dir_finds_flat_and_category_layouts`,
  `is_item_dir_is_true_for_a_command_only_directory` and
  `iter_item_resources_excludes_command_md`.

- [ ] **Step 2: Run `cargo test --test cli_pack_only_items` and confirm that the
      new tests fail**

- [ ] **Step 3: Implement discovery**

  In `src/pipeline/discovery.rs`: one
  `const ENTRYPOINTS: [&str; 4] = ["RULE.md", "SKILL.md", "AGENT.md", "COMMAND.md"];`
  used by `is_item_dir` and `is_entrypoint`; add `find_command_dir`. In
  `refuse_item_dir_source`, when the source holds `COMMAND.md` and none of the
  three other entry files, bail with
  ``source {shown} is a command item directory (it holds COMMAND.md); `upskill add` does not install command items. Use `upskill pack` to build a plugin that holds it``.
  `has_matching_items`, `scan_source_items`, `detect_item_entrypoint`,
  `effective_name` and `is_kind_dir` stay on the three installable kinds. Update
  the doc comments that list the entry files.

- [ ] **Step 4: Implement the count and the named-command error**

  - `src/model/bundle.rs`: `PackOnlyKind`, `pack_only`, `pack_only_items` as
    under Interfaces.
  - `src/pipeline/prepared.rs`: in `prepare_install`, right after
    `scan_source_items` and only when `!options.keep_recorded_excludes` and
    `!discovery::is_bundle_file(&local_source)` (plan decision 7), bail for the
    first name of `items` that no scanned item has, that `find_bundle_by_name`
    does not find, and for which `find_command_dir` returns `Some`, with
    ``"`{name}` is a command item; `upskill add` does not install command items. Use `upskill pack` to build a plugin that holds it"``.
    Add `pack_only_skipped`.
  - `src/pipeline/mod.rs` (`install_with_lockfile`): read
    `prepared.pack_only_skipped()` before `commit` and store it in
    `report.pack_only_skipped`.
  - `src/pipeline/lifecycle.rs` (`update`): after the `prepared.excluded()`
    loop, push each item of `prepared.pack_only_skipped()` that
    `report.pack_only_skipped` does not contain.
  - `src/main.rs`:
    `fn print_pack_only_skipped(items: &[(PackOnlyKind, String)])`, called in
    `run_add` after `print_excluded` and in `run_update` after
    `print_excluded_items`. It prints nothing when `items` is empty or
    `style::is_quiet()`. Otherwise one `eprintln!` (plan decision 5): the text
    `skipped`, then one part `{n} {label}` for each kind of `PackOnlyKind::ALL`
    with a count above 0, joined with `,`, then
    ``: only `upskill pack` writes them``. `PackOnlyKind::label` returns the
    full noun with the literal `(s)`: `"command item(s)"`. A later kind (hook in
    #631, LSP server in #634) is one enum value plus its label; this function
    does not change.

- [ ] **Step 5: Update the documentation**

  - `docs/book/commands.md`, section `upskill add`: after "**Bundle MCP
    servers.**", a paragraph "**Bundle command items.**": `add` installs no
    entry of `items.commands`, prints the stderr line above (not with `--quiet`,
    not with `--kind`), and `add <source> <name>` with the name of a command
    item, or a source that is a command item directory, fails with exit 1 and
    names `upskill pack`. Section `upskill update`: in the paragraph "The plugin
    and MCP steps of a bundle source run when …", add that `update` prints the
    same line for the command items of the bundles it prepares, also with
    `--dry-run`.
  - `docs/specification/specification.md`: §2.6 bullet "**Items.** Every entry
    in `items.rules`, `items.skills`, `items.agents` is resolved … and
    installed" adds that entries of `items.commands` are not installed and are
    counted on stderr; §2.2 "Known limits: the plugin and MCP steps …" gets the
    `update` sentence.
  - `docs/topics/0001-content-format.md` §2.3 "canonical entrypoint (`SKILL.md`,
    `RULE.md` or `AGENT.md`)": add `COMMAND.md` (this task makes `is_entrypoint`
    stop copying it as a resource).
  - `skills/upskill-writing-bundles/SKILL.md`: the YAML example adds `commands:`
    with the comment `# optional — add and update do not install them`; step
    "Add items — list skill/rule/agent names" adds command names. Statements
    about rendering wait for Task 3.
  - `AGENTS.md`: add `cli_pack_only_items` to the CLI test list.

- [ ] **Step 6: Run `cargo test` and confirm that all tests pass**

  The implementer does not commit. The orchestrator runs `just fmt`,
  `just verify` and `just sanity`, and commits with
  `feat(add): skip the command items of a bundle in add and update (#632)`.

### Task 3: `pack` renders commands and `init` as user-only skills

Before this task starts, the orchestrator rebases this branch on main and
updates the expected file counts and fixture lists of this task. (Story #634,
branch `feat/pack-lsp-servers`, adds `.lsp.json` and `lsp.json` to the pack
fixtures and edits `src/pack.rs`, `tests/cli_pack.rs` and
`tests/fixtures/pack/registry/base.bundle.yaml`; the counts below are those of
main at 924e0fe.)

**Files:**

- Modify: `src/generate/mod.rs`, `src/generate/claude.rs`,
  `src/generate/copilot.rs`, `src/pipeline/bundle_plan.rs`,
  `src/pipeline/install.rs` (`source_mode` becomes `pub(super)`),
  `src/pipeline/mod.rs` (re-exports), `src/pack.rs`
- Modify: `tests/cli_pack.rs`, `tests/fixtures/pack/registry/base.bundle.yaml`
  (`commands: [release-notes]` under `items`)
- Create: `tests/fixtures/pack/registry/items/release-notes/COMMAND.md`,
  `tests/fixtures/pack/registry/items/release-notes/template.md`,
  `tests/fixtures/pack/expected/demo-{claude,copilot}/skills/init/SKILL.md`,
  `tests/fixtures/pack/expected/demo-{claude,copilot}/skills/release-notes/SKILL.md`,
  `tests/fixtures/pack/expected/demo-{claude,copilot}/skills/release-notes/template.md`
- Delete: `tests/fixtures/pack/expected/demo-{claude,copilot}/commands/init.md`
- Docs: `docs/book/commands.md`, `docs/specification/specification.md`,
  `docs/topics/0001-content-format.md`, `docs/book/support-levels.md`,
  `docs/book/recipes.md`, `skills/upskill-cli/SKILL.md`,
  `skills/upskill-writing-bundles/SKILL.md`, `docs/roadmap.md`,
  `docs/wip/README.md`

`git check-ignore` reports none of the new fixture paths as ignored.

**Interfaces:**

- Consumes: `model::Command` (Task 1), `discovery::find_command_dir` (Task 2).
- Produces in `src/generate`:
  `pub fn render_command(command: &Command, body: &str, client: Client) -> Result<String>`
  (directives, frontmatter, `format::format_markdown`, as `render_skill`);
  `claude::command_frontmatter(command: &Command) -> Result<String>` and
  `copilot::command_frontmatter(command: &Command) -> Result<String>` with the
  keys of the global constraints. `Client::OpenCode` bails with
  `command items have no opencode output`.
- Produces in `src/pipeline/bundle_plan.rs`, re-exported from
  `src/pipeline/mod.rs`:

  ```rust
  pub struct BundleCommand { pub name: String, pub bundle: String, pub root: PathBuf }
  pub struct CommandResource { pub rel: PathBuf, pub bytes: Vec<u8>, pub mode: Option<u32> }
  pub struct CommandSource { pub command: Command, pub body: String, pub resources: Vec<CommandResource> }
  pub fn read_command(root: &Path, name: &str) -> Result<CommandSource>;
  ```

  `BundleOutput.commands: Vec<BundleCommand>` in closure order (required bundles
  first, then the order of `items.commands`); `root` is
  `closure.source_roots[&resolved.source_label]`. `read_command` errors:
  ``no directory `{name}` with a COMMAND.md in {root}``, and
  `` {path}: `name: {declared}` must equal the directory name `{name}` ``.
  Resources are `filter_ignored(iter_item_resources(dir), &command.ignore)` with
  `source_mode`.
- Private in `src/pack.rs`:

  ```rust
  fn check_command_names(bundles: &[Bundle]) -> Result<()>;
  /// Paths `skills/<name>/SKILL.md` and `skills/<name>/<rel>`.
  fn command_files(command: &Command, body: &str, resources: &[CommandResource], client: Client) -> Result<Vec<PackFile>>;
  /// `name: "init"`, description `Install the rules of the {bundle} bundle into this repository`.
  fn init_command(bundle: &str) -> Command;
  /// Replaces `init_text`; its text without the frontmatter.
  fn init_body(bundle: &str, source: &str, bundles: &[Bundle], client: PackClient) -> String;
  ```
- Error texts of `check_command_names` (plan decision 8):
  `` command `{name}` is named by both bundles `{first}` and `{second}` ``;
  ``command `{name}` and skill `{name}` have one name in the bundle closure; both would be written to skills/{name}/SKILL.md``;
  ``{kind} `init` cannot be packed with rules: `pack` writes the init command to skills/init/SKILL.md``
  with `{kind}` `command` or `skill`. `build` wraps a `read_command` error with
  `` command `{name}` of bundle `{bundle}` ``.

- [ ] **Step 1: Write the failing tests**

  Fixture: `items/release-notes/COMMAND.md` holds the example of design section
  2 (`name: release-notes`,
  `description: Write the release notes of the current branch`,
  `argument-hint: "[version]"`) with the body
  `Write the release notes for version $ARGUMENTS. Use the [template](template.md).`;
  `template.md` holds `Version, date, changes.`.

  `tests/cli_pack.rs`, new tests (error tests write their bundle into the
  temporary registry and assert exit 1, the text on stderr, and that `f.out`
  does not exist):

  - `pack_renders_a_command_as_a_user_only_skill_for_both_clients`: pack `demo`
    for both clients; the frontmatter keys of
    `demo-claude/skills/release-notes/SKILL.md` are, in order, `name`,
    `description`, `argument-hint`, `disable-model-invocation`, with
    `argument-hint == "[version]"` and `disable-model-invocation == true`; those
    of `demo-copilot/…` are `name`, `description`, `disable-model-invocation`;
    both bodies contain `$ARGUMENTS`; both `template.md` files equal the source
    bytes.
  - `pack_writes_no_commands_directory`: no path of either output starts with
    `commands/`.
  - `pack_renders_init_as_a_user_only_skill`: `demo-claude/skills/init/SKILL.md`
    has `name: init`,
    `description: Install the rules of the demo bundle into this repository`,
    `disable-model-invocation: true`, and its body contains
    `upskill add driftsys/demo@v1 demo --kind rule --project --claude`.
  - `pack_refuses_a_command_and_a_skill_with_one_name_in_the_closure`: bundle
    `clash` requires `base` and lists `commands: [beta]`.
  - `pack_refuses_a_command_named_init_in_a_closure_with_rules`: bundle
    `initcmd` requires `base` and lists `commands: [init]`.
  - `pack_refuses_a_skill_named_init_in_a_closure_with_rules`: a skill
    `items/init/SKILL.md` and bundle `initskill` that requires `base` and lists
    `skills: [init]`.
  - `pack_accepts_a_command_named_init_in_a_closure_without_rules`:
    `items/init/COMMAND.md` with `description: Author init`, bundle `ownit` with
    `commands: [init]`; success; `ownit-claude/skills/init/SKILL.md` has
    `description: Author init`.
  - `pack_refuses_one_command_named_by_two_bundles`: bundle `dup` requires
    `base` and lists `commands: [release-notes]`; text
    `` command `release-notes` is named by both bundles `base` and `dup` ``.
  - `pack_refuses_a_listed_command_without_a_command_md`: bundle lists
    `commands: [missing]`; stderr contains ``command `missing` of bundle`` and
    ``no directory `missing` with a COMMAND.md``.
  - `pack_refuses_a_command_whose_name_differs_from_its_directory`:
    `items/misnamed/COMMAND.md` with `name: other`.
  - `pack_copies_neither_a_co_located_rule_nor_an_ignored_file`: `items/notes/`
    with `COMMAND.md` (`ignore: [draft.md]`), `RULE.md`, `draft.md` and
    `ref.md`; a bundle lists `commands: [notes]` only; the files below
    `skills/notes/` are exactly `SKILL.md` and `ref.md`.
  - `pack_processes_client_directives_in_a_command_body`: a `COMMAND.md` body
    with a `<!-- @client:claude -->` block holding `Claude only` and a
    `<!-- @client:copilot -->` block holding `Copilot only`; the Claude skill
    contains only the first text, the Copilot skill only the second.

  Change the existing tests that name `commands/init.md` or `commands`
  (`pack_of_a_bundle_without_rules_writes_no_init_command_and_no_hook`,
  `pack_gives_no_init_command_to_a_client_that_no_rule_targets`,
  `pack_init_lists_plugins_and_no_mcp_server_for_both_clients`,
  `pack_of_a_bundle_with_mcp_servers_and_no_rules_writes_mcp_json_only`) to use
  `skills/init/SKILL.md`. In
  `pack_without_a_client_flag_writes_both_directories`, the expected line
  `Wrote {} (8 files)` becomes `(10 files)` for both clients: init keeps one
  file, and `release-notes` adds `SKILL.md` and `template.md`.

  Unit tests in `src/generate/mod.rs`:
  `render_command_writes_the_claude_keys_in_order`,
  `render_command_omits_argument_hint_for_copilot`,
  `render_command_has_no_opencode_output`. In `src/pack.rs`, the `init_*` tests
  compare `render_command(&init_command("demo"), &init_body(..),
  client)` with
  the new `skills/init/SKILL.md` fixtures.

- [ ] **Step 2: Run `cargo test --test cli_pack` and `cargo test --lib pack` and
      confirm that the new tests fail**

- [ ] **Step 3: Implement the renderer and the command reading**

  `render_command` and the two `command_frontmatter` functions as under
  Interfaces. In `src/pipeline/bundle_plan.rs`, collect `BundleOutput.commands`
  from the resolved bundles before they are mapped to `model::Bundle`, and add
  `read_command` (uses `discovery::find_command_dir`). Update the doc of
  `BundleOutput.files`.

- [ ] **Step 4: Implement `pack`**

  In `build`: after `plan_bundle_output`, call
  `check_command_names(&output.bundles)?`; for each `BundleCommand`, call
  `read_command` (with the context above) and push `command_files(..)`; when
  `output.rules` is not empty, push
  `command_files(&init_command(bundle),
  &init_body(..), &[], generator)`
  instead of `commands/init.md`, then the hook file as today. Remove
  `init_text`.

- [ ] **Step 5: Update the golden fixtures**

  Write the new expected files with the bytes that `pack` writes, delete the two
  `commands/init.md` files. Check that the Claude `release-notes` skill has
  `argument-hint` and the Copilot one does not, and that the init text below the
  frontmatter equals the old `commands/init.md` body.

- [ ] **Step 6: Update the documentation**

  - `docs/book/commands.md` (section `upskill pack`) and §2.8 of
    `docs/specification/specification.md`, which hold the same text: in the
    table, the row "Init, when rules target this client" gets
    `skills/init/SKILL.md` in both columns, and a new row "Commands" gets
    `skills/<name>/SKILL.md` and resources; a paragraph states the command
    rendering (the frontmatter of each client, `disable-model-invocation: true`,
    the call as `/<plugin>:<name>`, the closure) and the four errors of
    `check_command_names` plus the two `read_command` errors, each exit 1 before
    any write; the init paragraph states that init is written as the user-only
    skill `skills/init/SKILL.md`; "The bundle schema is unchanged." is replaced
    by "`pack` is the only command that renders `items.commands`." In "Known
    limits": the `init` skill has `disable-model-invocation: true`, so it cannot
    be started in Copilot CLI prompt mode (`-p`), which does not process slash
    commands (technote T4, trial 1); the user starts it in an interactive
    session.
  - `docs/topics/0001-content-format.md`: §3.9 adds the output of each client;
    the **Command** definition of §1 adds that only `pack` renders a command.
  - `docs/specification/specification.md` §1.1: the purpose of the `Command` row
    adds "Only `pack` renders it."
  - `skills/upskill-writing-bundles/SKILL.md`: the comment of `commands:` in the
    YAML example becomes `# optional — only upskill pack renders them`.
  - `docs/book/support-levels.md`, section "Bundle plugin output": command items
    are rendered only by `pack`, are experimental with plugin output, and have
    no row in the table or in `doctor` (plan decision 11).
  - `docs/book/recipes.md`, "Publish a bundle as client plugins": the plugin
    holds the command items as user-only skills, and the user calls init as
    `/<plugin>:init`.
  - `skills/upskill-cli/SKILL.md`, "Bundle plugin output and project rules":
    "The plugin carries skills, agents and the MCP servers" adds command items;
    add that `add` and `update` skip command items and print a count.
  - `docs/roadmap.md`, stage 2: "Only the item "MCP servers in the plugin" is
    implemented (#633)." names #632 as well; the "Command item kind" bullet ends
    with "Implemented in #632: …"; the "Init command" bullet states the path
    `skills/init/SKILL.md`.
  - `docs/wip/README.md`: stories #632 and #633 are implemented, five stories
    are planned work, and this plan is listed as an active document until it is
    gardened.

- [ ] **Step 7: Run `cargo test` and confirm that all tests pass**

  The implementer does not commit. The orchestrator runs `just fmt`,
  `just verify` and `just sanity`, and commits with
  `feat(pack): render command items and init as user-only skills (#632)`.
