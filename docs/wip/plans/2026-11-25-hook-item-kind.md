# Hook item kind (`HOOK.md`): implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

Date: 2026-11-25. Story: [#631](https://github.com/driftsys/upskill/issues/631).

**Goal:** A bundle lists hook items (`HOOK.md`) under `items.hooks`; `lint`,
`fmt` and `new` know the kind, `add` and `update` skip it with a count on
stderr, and `pack` writes the context hooks as static JSON files and the command
hooks as copied scripts, all in one hook file per client next to the
update-notice hook.

**Architecture:** A hook is a new model type (`model::Hook`), not a fourth
`pipeline::ItemKind`, for the same reason as `model::Command`: `ItemKind` is
serialized in the lockfile and selects install output paths. A new `parse::hook`
module splits a `HOOK.md` body into event sections and checks a hook against the
lint rules; `lint` and `pack` both call it. `add` and `update` learn one more
`PackOnlyKind` value. `pack` gets a private submodule `src/pack/hooks.rs` that
reads the hooks of the closure, builds the context files and owns the typed hook
file of each client, into which the update-notice entry is parsed so that a
notice-only hook file keeps its exact bytes.

**Tech stack:** Rust 2024, `serde`, `serde_json`, `serde_yaml_ng`,
`pulldown-cmark` 0.11 (already a dependency), `anyhow`; integration tests with
`assert_cmd` through `common::upskill_cmd`.

**Spec:** [plugin MVP design](../specs/2026-11-24-plugin-mvp-design.md), section
1, "Left to the builder" and "Not verified";
[AD-0046](../../adr/0046-plugin-mvp-components.md), Decisions 1, 3 to 7, 11 and
12, and the amendment of AD-0044 Decision 5; the source convention of the
[hook item kind draft](../../archive/specs/2026-11-18-hook-item-kind-design.md),
sections 1 to 4;
[technote T4](../../technotes/t4-copilot-plugin-component-trials.md), trial 5.

**Starting point:** stories #633 and #634 and the three tasks of the
[command item kind plan](2026-11-24-command-item-kind.md) are merged. This plan
uses these names from that plan: `model::Command`, `BundleItems.commands`,
`PackOnlyKind` (`Command`, `LspServer`), `Bundle::pack_only`, `pack_only_items`,
`PreparedInstall::pack_only_skipped`, `print_pack_only_skipped` in
`src/main.rs`, `discovery::find_command_dir`, `pipeline::CommandResource`,
`pipeline::BundleCommand`, `BundleOutput.commands`, `check_command_names`,
`command_files` and the init skill at `skills/init/SKILL.md`. If a name differs
in the merged code, use the merged name and keep the behaviour of this plan.

## Global constraints

- Entry file `HOOK.md`; bundle key `items.hooks`. The bundle lists the effective
  name of a hook, as for a rule: the frontmatter `name`, or the directory name
  when `name` is absent.
- Frontmatter: `name` (optional, MAY differ from the directory name),
  `description` (required), `events` (required map, at least one key), `schema`,
  `license`, `metadata`, `ignore`. Any other field is an error.
- An event value is empty or a map with the keys `matcher`, `run`, `timeout`.
  `matcher` is a string or a map with the keys `claude` and `copilot`.
- The ten portable events and their client names, exactly:

  | Portable             | Claude Code         | Copilot CLI           | Context |
  | -------------------- | ------------------- | --------------------- | ------- |
  | `session-start`      | `SessionStart`      | `sessionStart`        | yes     |
  | `session-end`        | `SessionEnd`        | `sessionEnd`          | no      |
  | `prompt-submit`      | `UserPromptSubmit`  | `userPromptSubmitted` | yes     |
  | `pre-tool`           | `PreToolUse`        | `preToolUse`          | no      |
  | `post-tool`          | `PostToolUse`       | `postToolUse`         | yes     |
  | `permission-request` | `PermissionRequest` | `permissionRequest`   | no      |
  | `stop`               | `Stop`              | `agentStop`           | no      |
  | `subagent-start`     | `SubagentStart`     | `subagentStart`       | yes     |
  | `subagent-stop`      | `SubagentStop`      | `subagentStop`        | no      |
  | `pre-compact`        | `PreCompact`        | `preCompact`          | no      |

- A `session-start` context hook without `matcher` gets the Claude Code matcher
  `startup|clear|compact` and no matcher in Copilot CLI.
- Hook file: `hooks/hooks.json` (Claude Code), `hooks.json` (Copilot CLI).
  Context files: `hooks/context/<event>.json` in both outputs. Command hook
  scripts: `hooks/<name>/`.
- Order in each event array of the hook file: the update-notice entry (only when
  the client output has rules), then the context entries, then the command
  entries.
- A generated context hook does not call `upskill`.
- A notice-only hook file stays byte-identical to the notice plugin file
  (`plugins/upskill-claude/hooks/hooks.json`,
  `plugins/upskill-copilot/hooks.json`).
- `pipeline::ItemKind`, the lockfile, `conflict.rs`, `src/support.rs` (the
  table), `src/index.rs` and `--kind` of `add` do not learn the kind.
- Not verified, so no doc may claim it: the 10,000-character limit of the
  context text that the Claude Code documentation states, and any limit in
  Copilot CLI; whether Copilot CLI runs `sessionStart` again after compaction;
  the Copilot CLI `matcher` field (documented only for `notification`,
  `permissionRequest`, `postToolUse`, `preCompact`, `preToolUse` and
  `subagentStart`; no trial has used it) and the tool names it accepts; context
  text from `pre-tool` in Copilot CLI; the Claude Code hook facts, which come
  from its documentation and not from a trial; the generated PowerShell command;
  a command hook in Claude Code on Windows.
- Evidence citations: the working directory of a Copilot CLI plugin hook is the
  plugin directory in one local-install observation (technote T4, trial 5), not
  a general fact. For `subagent-start`, the Copilot CLI behaviour (the text
  reaches the subagent only) is from trial 5, and the Claude Code side is from
  its documentation.
- Only `main.rs` writes to stdout and stderr. Library code returns data.
- Zero warnings: `just verify` and `just sanity` pass.
- Comments, documentation and messages use plain, literal English. Markdown is
  wrapped at 80 columns.
- Do not edit accepted ADRs.

## Plan decisions

Each decision is marked "plan decision N" where a task uses it.

1. `model::Hook` uses `#[serde(deny_unknown_fields)]` and accepts only the
   fields of the global constraints. `description` is required, as for a rule.
   `name` is optional, as for a rule (design: "may differ from its directory
   name, as for a rule"). `ignore` is kept because command hooks copy the item
   directory, and a skill and a command honour `ignore` when they copy theirs.
2. The keys of `events` are `String` in the model. An unknown event name is a
   `hook-event` finding of `lint` and an error of `pack`, not a serde error, so
   that the message lists the ten names and `lint` still runs the other rules.
3. Lint rule ids: `hook-event`, `hook-text`, `hook-heading`, `hook-run`,
   `hook-name`, all errors. Besides the four errors of the draft and the design,
   they report: an empty `events` map; `timeout` without `run` (a context entry
   merges the text of several items, so it has no single timeout); a `matcher`
   map with neither `claude` nor `copilot` (it would write no hook in either
   client); a `matcher` on `prompt-submit` or `stop` (Claude Code documents no
   matcher for `UserPromptSubmit` and `Stop` and ignores it, so two such hooks
   would become two entries with no defined text order, the case that AD-0046
   Decision 5 merges texts to avoid); an event heading that appears twice; under
   `hook-name`, the name rule and the reserved name of plan decision 14; and,
   under the existing `directive` rule, an event text whose `@client` blocks are
   unbalanced although the whole body is balanced (a block that crosses an event
   heading).
4. `run` must be one file name: ASCII letters, digits, `.`, `_` and `-`, with a
   letter or digit first. This keeps the generated shell text safe without
   quoting rules and refuses a path or arguments.
5. The body is split with `pulldown-cmark`, so a `##` line inside a code fence
   is content and a setext level-2 heading counts. The text of an event is the
   trimmed text before the first event heading, then the trimmed section, joined
   by one blank line. A body without an event heading is one text for every
   event. Context text gets `@client` processing and trimming only, not the
   dprint formatter: it is JSON content, not a Markdown file.
6. `PackOnlyKind::Hook` sits between `Command` and `LspServer`
   (`ALL = [Command, Hook, LspServer]`), so items come before servers in the
   skip line. Its label is `hook item(s)`.
7. The named-hook error of `add` uses the gate of decision 7 of the command plan
   (`!options.keep_recorded_excludes` and not a bundle file source). For one
   name, the command check runs first.
8. The update-notice merge is in Task 3, not in the command-hook task: Task 3
   introduces the typed hook file, and the first hook file it writes for a
   bundle with rules already holds the notice entry. The notice entry is parsed
   from the plugin file into typed structs and serialized again; struct field
   order reproduces the plugin file bytes. The parse of the `include_str!` text
   returns a `Result` that `hook_files` propagates; no `expect` or `unwrap`. The
   notice stays its own entry (Copilot CLI) or matcher group (Claude Code), also
   when a context hook has the same matcher.
9. The event keys of a hook file are sorted by client event name (a `BTreeMap`).
   The design fixes the order inside an event array only.
10. Context entries are grouped for each client by the resolved matcher of that
    client. The first group of an event writes `<event>.json`, the n-th group
    `<event>-<n>.json`, numbered in closure order for each client. No portable
    event name ends in `-<digit>`, so the design format cannot collide with an
    event name and is kept.
11. A context hook whose text is empty for a client after `@client` processing
    writes no entry and no file for that client. A client output whose hook file
    would hold no entry has no hook file.
12. Shell text of the generated commands (left to the builder by the design):
    - Claude Code context entry:
      `cat "${CLAUDE_PLUGIN_ROOT}/hooks/context/<file>"`.
    - Copilot CLI context entry: `bash` `cat hooks/context/<file>` and
      `powershell`
      `Get-Content -Raw -Encoding UTF8 -LiteralPath hooks/context/<file>` (the
      plugin directory was the working directory of a plugin hook in one
      local-install observation, technote T4, trial 5).
    - Claude Code command entry:
      `UPSKILL_CLIENT=claude "${CLAUDE_PLUGIN_ROOT}/hooks/<name>/<run>"`.
    - Copilot CLI command entry: `bash`
      `UPSKILL_CLIENT=copilot ./hooks/<name>/<run>`.

    The variable is set by a shell prefix in both clients. Claude Code documents
    no `env` field of a hook entry. Copilot CLI documents one, but no trial has
    used it, and one form for both clients is simpler.
13. An item with at least one `run` event is copied to `hooks/<name>/` only in
    the output of a client that gets at least one command entry of that item (a
    matcher map can give one client no entry). The copied files are
    `iter_item_resources` of the item directory (which leaves out every entry
    file, also a co-located `RULE.md`) minus the `ignore` matches, with their
    modes. A `run` value that names no copied file is an error of `pack`.
14. Hook name rules: a hook named by two bundles of the closure is an error of
    `pack` (checked on the bundle lists). The name `context`, which would
    collide with `hooks/context/`, and an effective name outside the format-spec
    §2.1 rule (lowercase letters, digits and hyphens, no hyphen at the start or
    end, at most 64 characters) are `hook-name` problems of `check_hook`, so
    `lint` reports them and `pack` refuses them, because the name is written
    into a path and a shell command.
15. A command hook without `matcher` gets no matcher in either client; the
    `session-start` default is for context hooks only (design text).
16. Task 3 refuses an event with `run`, with a temporary error that Task 4
    removes, so that the end state of Task 3 never drops a command hook without
    a message. The branch is merged as a whole. This error is intended, and the
    brief of the Task 3 reviewer states it. No Task 3 doc sentence may say that
    `pack` renders every hook item: the Task 3 docs name context hooks only, and
    Task 4 widens them to command hooks.
17. The hook golden fixtures use two new bundles, `hookbase` and `hookdemo`. The
    `demo` fixtures stay unchanged, so `pack_hook_file_equals_the_notice_plugin`
    and `pack_copilot_hook_file_equals_the_notice_plugin` keep pinning the
    notice-only bytes.
18. The content format gets a new section 3.10 instead of renumbering, so the
    anchors of `docs/specification/format-spec.md` stay valid.
19. `src/support.rs` gets no hook row; the support levels page describes the
    kind in its "Bundle plugin output" section. `--kind hook` keeps the usage
    error `unknown item kind: hook` (exit 2); `--exclude <hook-name>` stays an
    unknown exclude (exit 2). `name-collision` and `src/index.rs` do not learn
    the kind.
20. The hook code of `pack` lives in a new private submodule `src/pack/hooks.rs`
    (`mod hooks;` in `src/pack.rs`), because `src/pack.rs` is already about 1000
    lines and the hook file is one responsibility.
21. A context JSON file escapes every non-ASCII character as `\uXXXX` (lowercase
    hex, UTF-16 units, a surrogate pair above U+FFFF), so the file is ASCII
    only. Reason: Windows PowerShell 5.1 writes stdout in the console code page,
    so `Get-Content` or a `cat` alias in a PowerShell fallback can corrupt
    non-ASCII text; ASCII bytes pass through any code page. A non-ASCII
    character occurs in a JSON document only inside a string, so the escape is
    applied to the output of `serde_json::to_vec_pretty`. The hook file is not
    escaped: the notice-only file must keep the bytes of the plugin file.

## Review focus

1. Context text with `"`, `\`, a newline and a non-ASCII letter: the context
   file is valid JSON and parses back to the same text (test in Task 3).
2. A context hook whose text is only inside a `@client:claude` block: the
   Copilot output has no entry, no context file and, without rules, no hook file
   (test in Task 3).
3. A bundle with rules and no hook: the hook file bytes still equal the notice
   plugin file (existing tests, kept green in Task 3; unit round trip in Task
   3).
4. A `##` line inside a code fence, and a `###` heading inside an event section,
   are content and not event headings (tests in Task 1).
5. A `run` file that `ignore` matches: `pack` fails with exit 1 and writes
   nothing below `--out` (test in Task 4). Each `pack` error test of Tasks 3 and
   4 also asserts that `--out` does not exist.

---

### Task 1: model, body sections, `lint`, `fmt` and `new hook`

**Files:**

- Create: `src/model/hook.rs`, `src/parse/hook.rs`
- Modify: `src/model/mod.rs`, `src/model/bundle.rs` (`BundleItems`),
  `src/bundle.rs` (test helper literal), `src/parse/mod.rs`,
  `src/parse/bundle.rs` (test), `src/lint.rs`, `src/fmt.rs`, `src/scaffold.rs`,
  `src/main.rs` (`print_scaffold_report`), `src/cli.rs` (`New` summary, kind doc
  and examples)
- Test: `tests/cli_lint.rs`, `tests/cli_fmt.rs`, `tests/cli_new.rs`
- Docs: `docs/topics/0001-content-format.md`,
  `docs/specification/format-spec.md`, `docs/specification/specification.md`,
  `docs/book/commands.md`, `docs/book/conventions.md`, `docs/book/intro.md`,
  `AGENTS.md`

**Interfaces:**

- Produces in `src/model/hook.rs`, re-exported from `src/model/mod.rs` (`Hook`,
  `HookEvent`, `HookSettings`, `Matcher`, `ClientMatcher`):

  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
  pub enum HookEvent { SessionStart, SessionEnd, PromptSubmit, PreTool, PostTool,
      PermissionRequest, Stop, SubagentStart, SubagentStop, PreCompact }
  impl HookEvent {
      pub const ALL: [HookEvent; 10]; // table order of the global constraints
      pub fn parse(name: &str) -> Option<HookEvent>; // portable name
      pub fn name(self) -> &'static str;             // "session-start"
      pub fn claude_name(self) -> &'static str;      // "SessionStart"
      pub fn copilot_name(self) -> &'static str;     // "sessionStart"
      pub fn accepts_context(self) -> bool;          // the four "yes" rows
  }

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  #[serde(deny_unknown_fields)]
  pub struct Hook {
      #[serde(default)] pub schema: SchemaVersion,
      #[serde(default, skip_serializing_if = "Option::is_none")] pub name: Option<String>,
      pub description: String,
      pub events: BTreeMap<String, Option<HookSettings>>,
      #[serde(default, skip_serializing_if = "Option::is_none")] pub license: Option<License>,
      #[serde(default, skip_serializing_if = "Option::is_none")] pub metadata: Option<Metadata>,
      #[serde(default, skip_serializing_if = "Vec::is_empty")] pub ignore: Vec<String>,
  }

  #[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
  #[serde(deny_unknown_fields)]
  pub struct HookSettings {
      #[serde(default, skip_serializing_if = "Option::is_none")] pub matcher: Option<Matcher>,
      #[serde(default, skip_serializing_if = "Option::is_none")] pub run: Option<String>,
      #[serde(default, skip_serializing_if = "Option::is_none")] pub timeout: Option<u32>,
  }

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  #[serde(untagged)]
  pub enum Matcher { All(String), PerClient(ClientMatcher) }

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  #[serde(deny_unknown_fields)]
  pub struct ClientMatcher {
      #[serde(default, skip_serializing_if = "Option::is_none")] pub claude: Option<String>,
      #[serde(default, skip_serializing_if = "Option::is_none")] pub copilot: Option<String>,
  }

  /// Plan decision 4.
  pub fn is_run_file_name(value: &str) -> bool;
  ```

  An empty YAML value (`session-start:`) is `None`; settings are read with
  `.clone().unwrap_or_default()`.

- Produces in `src/parse/hook.rs` (`pub mod hook;` in `src/parse/mod.rs`):

  ```rust
  pub struct HookSection { pub event: String, pub line: usize, pub text: String }
  pub struct HookBody {
      /// Text before the first event heading; the whole body without one.
      pub preamble: String,
      /// One entry for each level-2 heading whose text is a key of `events`,
      /// in body order. `text` is the section without its heading line.
      pub sections: Vec<HookSection>,
      /// Level-2 headings that are not a key of `events`, with their body
      /// line. Collected only when `sections` is not empty.
      pub stray_headings: Vec<(usize, String)>,
  }
  impl HookBody {
      /// Plan decision 5: trimmed preamble and the trimmed first section of
      /// `event`, joined by "\n\n"; either part may be empty.
      pub fn text(&self, event: &str) -> String;
  }
  pub fn split_hook_body(body: &str, events: &[&str]) -> HookBody;

  pub struct HookProblem { pub rule_id: &'static str, pub line: Option<usize>, pub message: String }
  /// Every problem of a parsed hook; `dir` is its item directory, whose last
  /// component is the effective name when `hook.name` is `None`.
  pub fn check_hook(hook: &Hook, body: &str, dir: &Path) -> Vec<HookProblem>;
  /// Format-spec §2.1 effective-name rule (plan decision 14).
  pub fn is_valid_hook_name(name: &str) -> bool;
  ```

  Lines are 1-based lines of the body, as `body-h1` reports them.

- `check_hook` messages, exactly (`{e}` is the event key):
  - `hook-event`: `` a hook needs at least one event in `events` ``
  - `hook-event`:
    ``unknown event `{e}`; expected one of: session-start, session-end, prompt-submit, pre-tool, post-tool, permission-request, stop, subagent-start, subagent-stop, pre-compact``
  - `hook-event`:
    ``event `{e}` has no `run`, and a context hook is valid only on session-start, prompt-submit, post-tool and subagent-start``
  - `hook-event`:
    ``event `{e}` sets `timeout` without `run`: only a command hook has a timeout``
  - `hook-event`:
    `` event `{e}` has a `matcher` map with neither `claude` nor `copilot` ``
  - `hook-event` (for `prompt-submit` and `stop`, string or map):
    ``event `{e}` cannot have a `matcher`: Claude Code ignores a matcher on this event, so the texts of several hooks would have no defined order``
  - `hook-name` (`{name}` is the effective name, the frontmatter `name` or the
    directory name):
    ``hook name `{name}` must consist of lowercase letters, digits and hyphens, without a hyphen at the start or end, and with at most 64 characters (format-spec §2.1)``
  - `hook-name`:
    ``the hook name `context` is reserved: `pack` writes the context files to hooks/context/``
  - `hook-text`:
    ``context hook `{e}` has no text: write a `## {e}` section or text before the first event heading``
  - `hook-heading` (with line):
    ``level-2 heading `{text}` is not an event of this hook; use a level-3 heading for content``
  - `hook-heading` (with line): ``event heading `{e}` appears more than once``
  - `hook-run`:
    `` `run: {run}` must be the name of one file in the item directory, without a path or arguments ``
  - `hook-run`:
    ``event `{e}`: `run: {run}` names no file in the item directory``
  - `directive`: `event`{e}`:` followed by the error of `directives::process` on
    the event text; reported only when the whole body processes without error
    (plan decision 3).

  An unknown event gets no other check. The text checks use the raw text, before
  `@client` processing.

- Produces `BundleItems.hooks: Vec<String>` (same serde attributes as
  `commands`); `BundleItems::is_empty` also requires `hooks` to be empty.
- Produces `scaffold::NewKind::Hook` (`parse("hook")`, entry `HOOK.md`).

- [ ] **Step 1: Write the failing unit tests**

  `src/model/hook.rs`:

  - `hook_event_maps_each_portable_name_to_both_clients`: for each row of the
    table, `HookEvent::parse(portable)` is `Some(e)`, `e.name() == portable`,
    `e.claude_name()` and `e.copilot_name()` equal the row;
    `HookEvent::parse("on-save")` is `None`; `HookEvent::ALL.len() == 10`.
  - `hook_event_accepts_context_only_on_four_events`: the events with
    `accepts_context()` are exactly `SessionStart`, `PromptSubmit`, `PostTool`,
    `SubagentStart`.
  - `hook_parses_empty_and_map_event_settings`: YAML
    `description: d\nevents:\n  session-start:\n  post-tool: { matcher: edit, run: check.sh, timeout: 10 }\n`
    gives `events["session-start"] == None` and `events["post-tool"]` with
    `matcher == Some(Matcher::All("edit"))`, `run == Some("check.sh")`,
    `timeout == Some(10)`; `name == None`.
  - `matcher_accepts_a_client_map`: `{ claude: "Edit|Write", copilot: edit }`
    gives `Matcher::PerClient` with both keys.
  - `matcher_map_refuses_an_unknown_client_key`: `{ codex: x }` fails to parse.
  - `hook_refuses_an_unknown_field`: `audience: [claude]` fails with
    `` unknown field `audience` ``; `events` absent fails with
    `` missing field `events` ``.
  - `is_run_file_name_accepts_one_file_name_only`: true for `check.sh`,
    `run_1-a.py`; false for `""`, `.hidden`, `-x`, `a/b.sh`, `../a.sh`,
    `a b.sh`, `a;b`.

  `src/parse/hook.rs`, with `events = ["session-start", "prompt-submit"]` unless
  named:

  - `split_hook_body_gives_the_whole_body_to_every_event_without_event_headings`:
    body `"Text.\n\n## Overview\n\nMore.\n"`; `sections` and `stray_headings`
    are empty; `text("prompt-submit") == "Text.\n\n## Overview\n\nMore."`.
  - `split_hook_body_prepends_the_preamble_to_each_event`: body
    `"All.\n\n## session-start\n\nStart.\n\n## prompt-submit\n\nPrompt.\n"`;
    `text("session-start") == "All.\n\nStart."`,
    `text("prompt-submit") == "All.\n\nPrompt."`; `sections[1].line == 7`.
  - `split_hook_body_keeps_level_3_headings_as_content`: a `### Details` heading
    and its text inside the `session-start` section are in
    `text("session-start")`.
  - `split_hook_body_ignores_a_heading_inside_a_code_fence`: a fenced `text`
    block that holds `## prompt-submit` inside the `session-start` section;
    `sections.len() == 1` and the fence is in `text("session-start")`.
  - `split_hook_body_reports_a_level_2_heading_that_is_not_an_event`: body
    `"## session-start\n\nA.\n\n## Details\n\nB.\n"`;
    `stray_headings == [(5, "Details")]`; `text("session-start")` contains `B.`.
  - `split_hook_body_counts_a_setext_heading`: `"session-start\n---\n\nA.\n"`
    gives one section `session-start`.
  - `check_hook_reports_each_rule` (one temporary item directory with
    `check.sh`, one assertion for each message of the Interfaces list, built
    from small hooks; each asserts the `rule_id` and the exact message).
  - `check_hook_accepts_a_valid_hook`: `session-start` context with text and
    `post-tool: { matcher: { claude: Edit }, run: check.sh, timeout: 5 }` gives
    no problem.
  - `is_valid_hook_name_follows_the_name_rule`: true for `a`, `edit-checks`,
    `a1`; false for `""`, `-a`, `a-`, `Upper`, `a_b`, `a.b`, and a 65-letter
    name.

  `src/parse/bundle.rs`: `load_parses_items_hooks` (YAML
  `items:\n  hooks: [guard]` gives `items.hooks == ["guard"]` and
  `!items.is_empty()`).

- [ ] **Step 2: Write the failing CLI tests**

  `tests/cli_lint.rs` (each writes into a temporary registry and runs
  `upskill lint --strict <dir>`; every `HOOK.md` has `schema: 1` and
  `description: Test hook`):

  - `lint_accepts_a_hook_with_a_context_and_a_command_event`: `guard/HOOK.md`
    with
    `events: { session-start: , post-tool: { matcher: edit, run: check.sh } }`
    (block YAML), body
    `## session-start\n\nRead the guide.\n\n## post-tool\n\nChecks the file.\n`,
    and `guard/check.sh`; assert success.
  - `lint_accepts_a_hook_name_that_differs_from_its_directory`: `checks/` with
    `name: edit-checks`; assert success.
  - `lint_flags_an_unknown_hook_event`: event `on-save`; assert exit 1, stdout
    contains `[hook-event]` and ``unknown event `on-save`; expected one of:``.
  - `lint_flags_a_context_hook_without_text`: events `session-start` and
    `prompt-submit`, body `## session-start\n\nText.\n`; stdout contains
    `[hook-text]` and ``context hook `prompt-submit` has no text``.
  - `lint_flags_a_context_hook_on_an_event_without_context`: event `stop:` with
    text; stdout contains `[hook-event]` and
    ``event `stop` has no `run`, and a context hook is valid only on``.
  - `lint_flags_a_level_2_heading_that_is_not_an_event`: body
    `## session-start\n\nA.\n\n## Details\n\nB.\n`; stdout contains
    `[hook-heading]` and `use a level-3 heading for content`.
  - `lint_flags_a_run_file_that_does_not_exist`: `run: missing.sh`; stdout
    contains `[hook-run]` and `names no file in the item directory`.
  - `lint_flags_a_run_value_with_a_path`: `run: ../check.sh`; stdout contains
    `[hook-run]` and `without a path or arguments`.
  - `lint_flags_a_timeout_without_run`: `session-start: { timeout: 5 }`; stdout
    contains `` sets `timeout` without `run` ``.
  - `lint_flags_a_matcher_on_prompt_submit`: `prompt-submit: { matcher: edit }`
    with text; stdout contains `[hook-event]` and
    `` event `prompt-submit` cannot have a `matcher` ``.
  - `lint_flags_a_hook_name_outside_the_name_rule`: `upper/` with `name: Upper`;
    stdout contains `[hook-name]` and
    ``hook name `Upper` must consist of lowercase letters``.
  - `lint_flags_a_hook_named_context`: `context/HOOK.md` without `name`; stdout
    contains `[hook-name]` and ``the hook name `context` is reserved``.
  - `lint_flags_an_unknown_hook_field`: `audience: [claude]`; stdout contains
    `[frontmatter]` and `` unknown field `audience` ``.

  `tests/cli_fmt.rs`:

  - `fmt_orders_hook_keys_canonically`: frontmatter written in the order
    `events` (with two indented event lines), `description`, `name`, `schema`;
    after `upskill fmt <dir>` the top-level keys are `schema`, `name`,
    `description`, `events` in this order, the two event lines follow `events:`
    unchanged; a second run leaves the bytes unchanged.

  `tests/cli_new.rs`:

  - `new_hook_creates_hook_md`: `new hook team-reminder`; the file
    `team-reminder/HOOK.md` contains `\nschema: 1\n`, `\nname: team-reminder\n`,
    `\nevents:\n  session-start:\n` and `\n## session-start\n`;
    `upskill lint --strict` on the directory succeeds; `upskill fmt` leaves the
    bytes unchanged.
  - `new_hook_colocates_with_a_skill`: `new skill x`, then `new hook x`; both
    succeed and both files exist.
  - Rename `new_rejects_an_unknown_kind_and_lists_four_kinds` to
    `new_rejects_an_unknown_kind_and_lists_five_kinds`; stderr contains
    `expected one of: rule, skill, agent, command, hook`.

- [ ] **Step 3: Run `cargo test --lib hook` and
      `cargo test --test cli_lint --test cli_fmt --test cli_new`, and confirm
      that the new tests fail**

- [ ] **Step 4: Implement the model**

  `src/model/hook.rs` as under Interfaces, with the module doc
  `//! Hook items (§3.10).`. Add `hooks` to `BundleItems` and to the doc of
  `is_empty`; the test helper `items(..)` in `src/bundle.rs` builds
  `BundleItems { .., hooks: vec![] }`.

- [ ] **Step 5: Implement `src/parse/hook.rs`**

  `split_hook_body`: walk
  `pulldown_cmark::Parser::new(body).into_offset_iter()`; at
  `Event::Start(Tag::Heading { level: HeadingLevel::H2, .. })` record the byte
  range and collect the `Text` and `Code` events until
  `Event::End(TagEnd::Heading(_))`; the trimmed result is the heading text. A
  heading whose text is in `events` starts a section that ends at the start of
  the next such heading. The line of a heading is the count of `\n` before its
  start, plus 1. `check_hook` builds the problems in the order of the Interfaces
  list, for each event in `events` order.

- [ ] **Step 6: Implement lint**

  In `src/lint.rs`: `FileKind::Hook` (`"HOOK.md"`);
  `const ENTRYPOINTS: [&str; 5] = ["RULE.md", "SKILL.md", "AGENT.md", "COMMAND.md", "HOOK.md"];`;
  `parse_kind` parses `Hook` and returns its `name`; `check_name_matches_dir`
  treats `Hook` as `Rule` (no check). In `check_file`, after a successful parse
  of a `HOOK.md`, push one `Finding` for each `check_hook(..)` problem
  (`Severity::Error`, its `rule_id`, `line` and `message`; the directory is the
  parent of the file made absolute, as `item_dir_name` does). Add the five rule
  ids to the table of the module doc and `HOOK` to the discovery sentence. The
  doc comments of `discover` and `discover_self_entrypoints` that list the entry
  files add `HOOK.md`.

- [ ] **Step 7: Implement fmt**

  In `src/fmt.rs`: `EntryKind::Hook` for `HOOK.md`;
  `const HOOK_KEY_ORDER: &[&str] = &["schema", "name", "description", "events", "license", "metadata", "ignore"];`;
  `canonicalise` calls `canonicalise_item::<Hook>`.

- [ ] **Step 8: Implement `new hook`**

  In `src/scaffold.rs`: `NewKind::Hook`; `parse` error text
  ``unknown kind `{other}` — expected one of: rule, skill, agent, command, hook``;
  `entry()` returns `HOOK.md`; add `("HOOK.md", false)` to
  `ENTRYPOINT_SKILL_MATCH` (a hook name may differ, as a rule name). `render`
  for a hook:

  ```text
  ---
  schema: 1
  name: {name}
  description: TODO describe the text that this hook adds to each session.
  events:
    session-start:
  ---

  ## session-start

  TODO write the text that the model receives at the start of each session.
  ```

  Update the module doc. `print_scaffold_report` maps `NewKind::Hook` to `hook`.
  In `src/cli.rs`, the `New` summary reads
  `Scaffold a new rule, skill, agent, command, or hook.`, the `New` kind doc
  reads ``One of `rule`, `skill`, `agent`, `command`, `hook`.`` and the examples
  add `upskill new hook team-reminder`.

- [ ] **Step 9: Update the documentation**

  - `docs/topics/0001-content-format.md`, Abstract ("rules, skills, agents, and
    commands"), Audience ("rules, skills, agents, commands"), the `schema`
    sentence of §3.1 and §8.1 ("an item (rule, skill, agent, or command)"): each
    adds hooks.
  - `docs/topics/0001-content-format.md`: §1 "**Item**: a rule, skill, agent, or
    command. The four kinds…" names five kinds and adds a **Hook** definition
    (text for the model's context or a script, run on a client event;
    `upskill add` and `upskill update` do not install hook items). §2.1: the
    tree adds `<name-hook>/HOOK.md`; "at least one entrypoint file" and "Kind is
    determined by the entrypoint filename" add `HOOK.md`; the effective-name
    list states that a hook `name:` MAY differ from the directory name, as for a
    rule. §3 intro adds `HOOK.md`. §3.1: a hook rejects `audience` and
    `requires` (see §3.10). §3.7 table: row `items.hooks` (`string[]`, no, "Hook
    names (§3.10)."). §3.8 key-order table: row `Hook` with the order of
    `HOOK_KEY_ORDER`. New section `### 3.10 Hook-specific fields` after §3.9
    (plan decision 18): an example with a `session-start` context event and a
    `post-tool` command event; the fields table (`name` no, `description` YES,
    `events` YES); the event settings table (`matcher`: string or map with
    `claude` and `copilot`, not allowed on `prompt-submit` and `stop`; `run`:
    one file name of the item directory; `timeout`: seconds, only with `run`);
    the hook name rule and the reserved name `context`; the ten portable event
    names; context hook (no `run`, only the four context events) and command
    hook (`run`; the text is documentation); the body rules of plan decision 5
    (event sections, preamble, level-3 headings are content, a level-2 heading
    that is not an event is an error when the body has event sections); the
    accepted common fields. No sentence about `pack` output yet.
  - `docs/specification/format-spec.md`: an anchor
    `<a id="310-hook-specific-fields"></a>` and a link item after 3.9.
  - `docs/specification/specification.md`: §1.1 row `Hook` (purpose: "Text for
    the model's context, or a script, on a client event.", entry `HOOK.md`);
    §1.3 adds `HOOK.md`; the `new` row of the §2 table and §2.5 add `hook`.
  - `docs/book/commands.md`: the `new` row of the top table; section
    `upskill new`: the `<kind>` list, one example line, and in "Co-location"
    that a hook may share a directory with any kind; section `upskill lint`:
    "Eight rules" becomes "Thirteen rules" with rows for `hook-event`,
    `hook-text`, `hook-heading`, `hook-run` and `hook-name` (severity error,
    source format-spec §3.10).
  - `docs/book/conventions.md` and `docs/book/intro.md`: the lists of kinds and
    entry files add the hook and `HOOK.md`.
  - `AGENTS.md`: the `model/` line reads
    `Rule, Skill, Agent, Command, Hook, Bundle + common`; the `parse/` line adds
    `HOOK.md` body sections.

- [ ] **Step 10: Run `cargo test` and confirm that all tests pass**

  The implementer does not commit. The orchestrator runs `just fmt`,
  `just verify` and `just sanity`, and commits with
  `feat(lint): add the hook item kind to lint, fmt and new (#631)`.

### Task 2: `add` and `update` skip hook items

**Files:**

- Modify: `src/model/bundle.rs` (`PackOnlyKind`, `Bundle::pack_only`, unit
  tests), `src/pipeline/discovery.rs`, `src/pipeline/prepared.rs`
- Test: `tests/cli_pack_only_items.rs`
- Docs: `docs/book/commands.md`, `docs/specification/specification.md`,
  `docs/topics/0001-content-format.md`, `docs/book/recipes.md`,
  `skills/upskill-writing-bundles/SKILL.md`, `skills/upskill-cli/SKILL.md`

**Interfaces:**

- Consumes: `BundleItems.hooks`, `model::Hook` (Task 1).
- Produces `PackOnlyKind::Hook`; `PackOnlyKind::ALL` becomes
  `[Command, Hook, LspServer]` (declaration order the same, so the derived `Ord`
  agrees); `Hook.label()` returns `"hook item(s)"`; `Bundle::pack_only(Hook)`
  returns `items.hooks` (plan decision 6). `print_pack_only_skipped` in
  `src/main.rs` does not change.
- Produces in `src/pipeline/discovery.rs`:
  `pub(super) fn find_hook_dir(root: &Path, name: &str) -> Result<Option<PathBuf>>`:
  the first directory of `iter_item_dirs(root)` that holds `HOOK.md` and whose
  effective hook name equals `name`. The effective name is the frontmatter
  `name`, else the folder name; a `HOOK.md` that cannot be read or parsed counts
  with its folder name, as `probe_effective_name` does. Task 3 uses it.
- Error texts:
  ``"`{name}` is a hook item; `upskill add` does not install hook items. Use `upskill pack` to build a plugin that holds it"``
  and
  ``source {shown} is a hook item directory (it holds HOOK.md); `upskill add` does not install hook items. Use `upskill pack` to build a plugin that holds it``.

- [ ] **Step 1: Write the failing tests in `tests/cli_pack_only_items.rs`**

  Extend the registry helper with `guard/HOOK.md` (one `session-start` context
  hook), `checks/HOOK.md` (`name: edit-checks`, one `session-start` context
  hook) and `hooked.bundle.yaml` (`skills: [alpha]`, `commands: [standup]`,
  `hooks: [guard]`, and
  `lsps: { ra: { command: ra, extensions: { .rs: rust } } }`). The existing
  bundles and `LINE` do not change.

  - `add_reports_skipped_hook_items_between_commands_and_lsp_servers`:
    `add <registry> hooked`; success; stderr contains, exactly,
    ``skipped 1 command item(s), 1 hook item(s), 1 LSP server(s): only `upskill pack` writes them``;
    no path under the consumer contains `guard`; `.upskill-lock.json` does not
    contain `guard`.
  - `add_reports_only_hook_items_when_the_bundle_has_no_other_pack_only_kind`:
    `hookonly.bundle.yaml` with `skills: [alpha]`, `hooks: [guard]`; stderr
    contains ``skipped 1 hook item(s): only `upskill pack` writes them``.
  - `add_of_a_named_hook_item_is_an_error_that_names_pack`:
    `add <registry> guard`; exit 1; stderr contains the named-hook text above
    with `guard`; `.upskill-lock.json` does not exist.
  - `add_of_a_hook_by_its_frontmatter_name_is_an_error_that_names_pack`:
    `add <registry> edit-checks`; exit 1; the same text with `edit-checks`.
  - `add_of_a_skill_and_a_hook_by_name_fails_before_any_write`:
    `add <registry> alpha guard`; exit 1; `.claude` does not exist.
  - `add_of_a_hook_item_directory_as_source_is_an_error_that_names_pack`:
    `add <registry>/skills/guard`; exit 1; stderr contains
    `is a hook item directory (it holds HOOK.md)` and `upskill pack`.
  - `add_kind_hook_is_a_usage_error`: `add <registry> hooked --kind hook`; exit
    2; stderr contains `unknown item kind: hook`.
  - `add_with_a_kind_restriction_prints_no_hook_count`:
    `add <registry> hooked --kind skill`; stderr does not contain `hook item`.
  - `update_reports_the_skipped_hook_items_of_a_recorded_bundle`: after
    `add <registry> hooked`, `update` and `update --dry-run` each succeed and
    print the exact line of the first test.

  Unit tests: in `src/model/bundle.rs`, change the assertion of
  `pack_only_items_sorts_and_deduplicates_lsp_servers_by_kind_and_name` on
  `PackOnlyKind::ALL` to `[Command, Hook, LspServer]` and add
  `pack_only_items_lists_hooks_between_commands_and_lsp_servers` (one bundle
  with `commands: [c]`, `hooks: [h]` and one LSP server `l` gives
  `[(Command, "c"), (Hook, "h"), (LspServer, "l")]`; `Hook.label()` is
  `"hook item(s)"`). In `src/pipeline/discovery.rs`:
  `find_hook_dir_matches_the_frontmatter_name_or_the_folder` (flat and category
  layouts; a `HOOK.md` with `name: other` is found by `other` and not by its
  folder; a skill-only folder is `None`),
  `is_item_dir_is_true_for_a_hook_only_directory` and
  `iter_item_resources_excludes_hook_md` (a nested `HOOK.md` is kept).

- [ ] **Step 2: Run `cargo test --test cli_pack_only_items` and
      `cargo test --lib pack_only discovery`, and confirm that the new tests
      fail**

- [ ] **Step 3: Implement discovery**

  `ENTRYPOINTS` gets `"HOOK.md"` (5 entries; `is_item_dir` and `is_entrypoint`
  use it). Add `find_hook_dir`. In `refuse_item_dir_source`, after the command
  check, bail with the hook-directory text when the source holds `HOOK.md` and
  none of `RULE.md`, `SKILL.md`, `AGENT.md`, `COMMAND.md`. `has_matching_items`,
  `scan_source_items`, `detect_item_entrypoint`, `effective_name` and
  `is_kind_dir` stay on the three installable kinds. Update the doc comments
  that list the entry files: `is_item_dir`, `find_registry_root`,
  `iter_item_resources` and `is_entrypoint` add `HOOK.md`.

- [ ] **Step 4: Implement the count and the named-hook error**

  `src/model/bundle.rs`: `PackOnlyKind::Hook` as under Interfaces. In
  `src/pipeline/prepared.rs`, in the loop of the named-command check (plan
  decision 7), after the command condition, bail with the named-hook text for a
  name that no scanned item has, that `find_bundle_by_name` does not find, and
  for which `find_hook_dir` returns `Some`. Update the doc of
  `pack_only_skipped` ("command, hook or LSP server").

- [ ] **Step 5: Update the documentation**

  - `docs/book/commands.md`, section `upskill add`: the paragraph "**Bundle
    command items.**" becomes "**Bundle command and hook items.**": `add`
    installs no entry of `items.commands` or `items.hooks`; the example line
    shows
    ``skipped 2 command item(s), 1 hook item(s): only `upskill pack` writes them``;
    the count of LSP servers comes last; a name or source directory of a hook
    item fails as for a command; `--kind hook` is a usage error. Section
    `upskill update`: the skip-line paragraph names hook items. Section
    `upskill pack`: the sentence "They report their count in the same stderr
    skip line as command items" names hook items too.
  - `docs/specification/specification.md`: the §2.6 "**Items.**" bullet and the
    §2.2 `update` sentence name `items.hooks`; the §2.8 LSP sentence as in
    `commands.md`.
  - `docs/topics/0001-content-format.md` §2.3: the list of canonical entry files
    adds `HOOK.md` (this task makes `is_entrypoint` stop copying it as a
    resource of a co-located skill).
  - `docs/book/recipes.md`, `skills/upskill-cli/SKILL.md` and
    `skills/upskill-writing-bundles/SKILL.md`: the sentences "in the same stderr
    skip line as command items" name hook items; in `upskill-writing-bundles`,
    the YAML example adds `hooks:` with the comment
    `# optional — add and update do not install them` and the step "Add items"
    names hook names.

- [ ] **Step 6: Run `cargo test` and confirm that all tests pass**

  The implementer does not commit. The orchestrator runs `just fmt`,
  `just verify` and `just sanity`, and commits with
  `feat(add): skip the hook items of a bundle in add and update (#631)`.

### Task 3: `pack` writes context hooks and the merged hook file

**Precondition, checked by the orchestrator before dispatch:** Task 3 of the
command plan (#632) is merged on this branch: `pack` renders `init` as
`skills/init/SKILL.md`, and the only `include_str!` of a hook file left in
`build` is the notice hook file.

**Note for the implementer and the task reviewer (plan decision 16):** the
temporary error for an event with `run` is intended. The Task 3 docs describe
context hooks only; no Task 3 doc sentence may say that `pack` renders every
hook item.

**Files:**

- Create: `src/pack/hooks.rs`
- Modify: `src/pack.rs` (`mod hooks;`, `build`), `src/pipeline/bundle_plan.rs`,
  `src/pipeline/mod.rs` (re-exports)
- Modify: `tests/cli_pack.rs`
- Create fixtures:
  - `tests/fixtures/pack/registry/hookbase.bundle.yaml`
  - `tests/fixtures/pack/registry/hookdemo.bundle.yaml`
  - `tests/fixtures/pack/registry/items/reminder/HOOK.md`
  - `tests/fixtures/pack/registry/items/checks/HOOK.md`
  - `tests/fixtures/pack/expected/hookdemo-claude/.claude-plugin/plugin.json`
  - `tests/fixtures/pack/expected/hookdemo-claude/skills/init/SKILL.md`
  - `tests/fixtures/pack/expected/hookdemo-claude/hooks/hooks.json`
  - `tests/fixtures/pack/expected/hookdemo-claude/hooks/context/session-start.json`
  - `tests/fixtures/pack/expected/hookdemo-claude/hooks/context/prompt-submit.json`
  - the same five paths below `hookdemo-copilot/`, with `plugin.json` and
    `hooks.json` at the root instead of `.claude-plugin/plugin.json` and
    `hooks/hooks.json`
- Docs: `docs/book/commands.md`, `docs/specification/specification.md`,
  `docs/topics/0001-content-format.md`, `docs/book/support-levels.md`,
  `docs/book/recipes.md`, `skills/upskill-cli/SKILL.md`,
  `skills/upskill-writing-bundles/SKILL.md`, `AGENTS.md`

`git check-ignore` reports none of these paths (checked on 2026-11-25; the
fixtures contain no `.mcp.json`, which `.gitignore` ignores).

**Interfaces:**

- Consumes: `model::{Hook, HookEvent, HookSettings, Matcher}`,
  `parse::hook::{split_hook_body, check_hook, HookBody}` (Task 1),
  `discovery::find_hook_dir` (Task 2).
- Produces in `src/pipeline/bundle_plan.rs`, re-exported from
  `src/pipeline/mod.rs`:

  ```rust
  pub struct BundleHook { pub name: String, pub bundle: String, pub root: PathBuf }
  pub struct HookSource { pub hook: Hook, pub path: PathBuf, pub dir: PathBuf, pub body: String }
  pub fn read_hook(root: &Path, name: &str) -> Result<HookSource>;
  ```

  `BundleOutput.hooks: Vec<BundleHook>` in closure order (required bundles
  first, then the order of `items.hooks`), collected as Task 3 of the command
  plan collects `BundleOutput.commands`. `read_hook` error:
  ``no directory with a HOOK.md for the hook `{name}` in {root}``; a parse error
  gets the context `reading {path}`.
- Produces in `src/pack/hooks.rs` (private to `pack`):

  ```rust
  pub(super) struct PackHook { pub name: String, pub source: HookSource, pub body: HookBody }
  /// Plan decision 14: a hook named by two bundles of the closure. The name
  /// rule and the reserved name are `check_hook` problems (Task 1).
  pub(super) fn check_hook_names(bundles: &[Bundle]) -> Result<()>;
  /// `read_hook`, `split_hook_body` and `check_hook` for each hook; the first
  /// problem is an error `{path}: {message}`; each error gets the context
  /// `` hook `{name}` of bundle `{bundle}` ``.
  pub(super) fn read_pack_hooks(hooks: &[BundleHook]) -> Result<Vec<PackHook>>;
  /// The context files, then the hook file; empty when the hook file would
  /// hold no entry (plan decision 11). `notice` is true when the client
  /// output has rules.
  pub(super) fn hook_files(hooks: &[PackHook], notice: bool, client: PackClient) -> Result<Vec<PackFile>>;
  /// `None`: no hook for this client. `Some(None)`: a hook without matcher.
  fn resolve_matcher(settings: &HookSettings, event: HookEvent, context: bool, client: PackClient) -> Option<Option<String>>;
  ```

  Hook file types, private, with `#[serde(deny_unknown_fields)]` and these field
  orders, which reproduce the notice plugin files:

  ```rust
  struct ClaudeHookFile { hooks: BTreeMap<String, Vec<ClaudeGroup>> }
  struct ClaudeGroup { matcher: Option<String> /* skipped when None */, hooks: Vec<ClaudeCommand> }
  struct ClaudeCommand { r#type: String /* "type" */, command: String, timeout: Option<u32> /* skipped when None */ }
  struct CopilotHookFile { version: u32, hooks: BTreeMap<String, Vec<CopilotCommand>> }
  struct CopilotCommand { r#type: String, matcher: Option<String>, bash: String,
      powershell: Option<String>, timeout_sec: Option<u32> /* "timeoutSec" */ } // Options skipped when None
  ```

  The notice files are read with
  `include_str!("../../plugins/upskill-claude/hooks/hooks.json")` and
  `include_str!("../../plugins/upskill-copilot/hooks.json")`; the two
  `include_str!` calls in `build` are removed. Their parse returns a `Result`
  that `hook_files` propagates with `?` (plan decision 8). A file is written
  with `serde_json::to_vec_pretty` and one trailing `\n`. Copilot `version` is
  `1`. A context file is then passed through
  `fn ascii_escape(json: &[u8]) -> Vec<u8>` (plan decision 21), which replaces
  each non-ASCII character with `\uXXXX` escapes; the hook file is not.
- Error text of `check_hook_names`:
  `` hook `{name}` is named by both bundles `{first}` and `{second}` ``.
  Temporary error of `read_pack_hooks` (plan decision 16, removed in Task 4):
  ``event `{event}` has `run`: this version of `pack` does not write command hooks``.

**Algorithm of `hook_files` for one client** (the code step needs it; the
signatures do not fix it):

```text
groups = []                              # (event, matcher, texts), creation order
for hook in hooks (closure order):
    for (key, settings) in hook.source.hook.events (map order):
        event = HookEvent::parse(key)    # checked by read_pack_hooks
        matcher = resolve_matcher(settings, event, context = true, client)
        if matcher is None: continue
        text = directives::process(hook.body.text(key), generator)?.trim()
                                         # generator: Client::Claude for
                                         # PackClient::Claude, Client::Copilot
                                         # for PackClient::Copilot, as in build
        if text is empty: continue       # plan decision 11
        append text to the group (event, matcher), or create that group
for each group, numbered n = 1, 2, ... within its event:
    file = "<event.name()>.json" if n == 1 else "<event.name()>-<n>.json"
    write hooks/context/<file>; add one entry to the event array
event arrays: the notice entries first (when notice), then the context entries
```

`resolve_matcher`: no `matcher` gives `Some(Some("startup|clear|compact"))` for
a Claude Code `session-start` context hook and `Some(None)` otherwise;
`Matcher::All(m)` gives `Some(Some(m))`; `Matcher::PerClient` gives the key of
the client wrapped in `Some`, or `None` when the key is absent.

**Fixtures.** `hookbase.bundle.yaml`: `schema: 1`, `name: hookbase`,
`description: Hook base bundle`, `items: { hooks: [reminder] }` (block YAML).
`hookdemo.bundle.yaml`: `name: hookdemo`, `description: Hook demo bundle`,
`items: { rules: [style], hooks: [edit-checks] }`,
`requires: [{ name: hookbase }]`. `items/reminder/HOOK.md`:

```markdown
---
schema: 1
name: reminder
description: Remind the model of the team conventions
events:
  session-start:
  prompt-submit:
---

Follow the team conventions.

## session-start

Read `CONVENTIONS.md` before you edit a file.

## prompt-submit

Answer in English.
```

`items/checks/HOOK.md`: `name: edit-checks`,
`description: Check each edited
file`, `events:` with `session-start:` only,
body `## session-start\n\nRun the checks before you commit.\n`. Write both files
in the form that `upskill fmt` leaves unchanged.

**Golden hook files** (the notice entries are those of the plugin files,
unchanged; `…` stands for them only in this plan).
`hookdemo-claude/hooks/hooks.json`:

```json
{
  "hooks": {
    "SessionStart": [
      { "matcher": "startup", "hooks": [ … the notice command, "timeout": 30 … ] },
      {
        "matcher": "startup|clear|compact",
        "hooks": [
          {
            "type": "command",
            "command": "cat \"${CLAUDE_PLUGIN_ROOT}/hooks/context/session-start.json\""
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cat \"${CLAUDE_PLUGIN_ROOT}/hooks/context/prompt-submit.json\""
          }
        ]
      }
    ]
  }
}
```

`hookdemo-claude/hooks/context/session-start.json`:

```json
{
  "hookSpecificOutput": {
    "hookEventName": "SessionStart",
    "additionalContext": "Follow the team conventions.\n\nRead `CONVENTIONS.md` before you edit a file.\n\nRun the checks before you commit."
  }
}
```

`prompt-submit.json` has `"hookEventName": "UserPromptSubmit"` and
`"additionalContext": "Follow the team conventions.\n\nAnswer in English."`.
`hookdemo-copilot/hooks.json`:

```json
{
  "version": 1,
  "hooks": {
    "sessionStart": [
      { … the notice entry: "type", "bash", "timeoutSec": 30 … },
      {
        "type": "command",
        "bash": "cat hooks/context/session-start.json",
        "powershell": "Get-Content -Raw -Encoding UTF8 -LiteralPath hooks/context/session-start.json"
      }
    ],
    "userPromptSubmitted": [
      {
        "type": "command",
        "bash": "cat hooks/context/prompt-submit.json",
        "powershell": "Get-Content -Raw -Encoding UTF8 -LiteralPath hooks/context/prompt-submit.json"
      }
    ]
  }
}
```

The Copilot context files hold only `{"additionalContext": "<text>"}` with the
same texts, pretty-printed.

- [ ] **Step 1: Write the failing tests**

  `tests/cli_pack.rs`, new tests. Error tests write their files into the
  temporary registry and assert exit 1, the text on stderr, and that `f.out`
  does not exist. Each test that adds an item writes a new bundle that lists it;
  "pack X" means `f.command_for_clients("X")` (both clients).

  - `pack_writes_the_hook_plugin_directories`: pack `hookdemo`; the trees of
    `hookdemo-claude` and `hookdemo-copilot` equal
    `tests/fixtures/pack/expected/hookdemo-{claude,copilot}`.
  - `pack_merges_the_context_hooks_of_one_event_in_closure_order`: pack
    `hookdemo`; in both outputs the `additionalContext` of
    `hooks/context/session-start.json` equals
    ``"Follow the team conventions.\n\nRead `CONVENTIONS.md` before you edit a file.\n\nRun the checks before you commit."``.
  - `pack_gives_session_start_the_default_matcher_in_claude_code_only`: pack
    `hookdemo`; the second `SessionStart` group of the Claude hook file has
    `matcher == "startup|clear|compact"`; the second `sessionStart` entry of the
    Copilot hook file has no `matcher` key.
  - `pack_writes_one_context_file_for_each_distinct_matcher`:
    `items/after-edit/HOOK.md` with `post-tool: { matcher: edit }` and the text
    `After an edit.`, `items/after-shell/HOOK.md` with
    `post-tool: { matcher: shell }` and the text `After a shell command.`;
    bundle `twogroups` (`hooks: [after-edit, after-shell]`, no rules). In both
    outputs `hooks/context/post-tool.json` holds `After an edit.` and
    `hooks/context/post-tool-2.json` holds `After a shell command.`; the Claude
    `PostToolUse` groups have the matchers `edit` then `shell`, the second with
    the command `cat "${CLAUDE_PLUGIN_ROOT}/hooks/context/post-tool-2.json"`;
    the Copilot `postToolUse` entries have the matchers `edit` then `shell`, the
    second with `bash` `cat hooks/context/post-tool-2.json`. (Copilot CLI
    documents no matcher on `sessionStart`, so this test does not use
    `session-start`.)
  - `pack_writes_a_string_matcher_for_both_clients`:
    `post-tool: { matcher: edit }` with text; Claude
    `PostToolUse[0].matcher == "edit"`; Copilot
    `postToolUse[0].matcher == "edit"`.
  - `pack_writes_a_map_matcher_only_for_the_named_client`:
    `post-tool: { matcher: { claude: Edit } }` with text; Claude
    `PostToolUse[0].matcher == "Edit"` and the file
    `hooks/context/post-tool.json` exists; the Copilot hook file has no
    `postToolUse` key and `hooks/context/post-tool.json` does not exist in the
    Copilot output.
  - `pack_of_a_bundle_with_hooks_and_no_rules_writes_a_hook_file_without_the_notice`:
    pack `hookbase`; both hook files exist and do not contain `upskill`; the
    Claude `SessionStart` array has one group; `skills/init` does not exist.
  - `pack_context_hooks_do_not_call_upskill`: pack `hookbase`; no file of either
    output contains the text `upskill`.
  - `pack_processes_client_directives_in_context_text`: a `session-start` text
    with a `<!-- @client:claude -->` block `Claude only` and a
    `<!-- @client:copilot -->` block `Copilot only`; the Claude context text
    contains only the first, the Copilot text only the second.
  - `pack_writes_no_entry_for_a_client_whose_context_text_is_empty`: a bundle
    without rules whose only hook text is inside a `@client:claude` block; the
    Claude hook file exists; the Copilot output has no `hooks.json` and no
    `hooks/` directory (Review focus 2).
  - `pack_escapes_context_text_as_json`: the text
    `Say "hi" to C:\temp and to Zoë.` on two lines (a newline between `hi"` and
    `to`); the bytes of both context files are ASCII only (plan decision 21) and
    the parsed `additionalContext` of both clients equals the text (Review focus
    1).
  - `pack_refuses_an_invalid_hook`: `items/bad/HOOK.md` with a `stop:` context
    event and text, bundle `badhook`; stderr contains
    `` hook `bad` of bundle `badhook` `` and `` event `stop` has no `run` ``.
  - `pack_refuses_a_listed_hook_without_a_hook_md`: bundle lists
    `hooks: [missing]`; stderr contains
    `` no directory with a HOOK.md for the hook `missing` ``.
  - `pack_refuses_one_hook_named_by_two_bundles`: bundle `dup`
    (`requires: hookbase`, `hooks: [reminder]`); stderr contains
    `` hook `reminder` is named by both bundles `hookbase` and `dup` ``.
  - `pack_refuses_a_hook_named_context`: `items/context/HOOK.md`, bundle lists
    `hooks: [context]`; stderr contains ``the hook name `context` is reserved``
    (a `check_hook` problem, plan decision 14).
  - `pack_refuses_a_hook_name_outside_the_name_rule`: `items/upper/HOOK.md` with
    `name: Upper`, bundle lists `hooks: [Upper]`; stderr contains
    ``hook name `Upper` must consist of lowercase letters``.
  - `pack_refuses_a_command_hook_in_this_task`: `post-tool: { run: check.sh }`
    with `check.sh`; stderr contains
    ``this version of `pack` does not write command hooks`` (Task 4 replaces
    this test).

  Keep `pack_hook_file_equals_the_notice_plugin` and
  `pack_copilot_hook_file_equals_the_notice_plugin` unchanged (Review focus 3).

  Unit tests in `src/pack/hooks.rs`:
  `notice_hook_files_round_trip_byte_for_byte` (parse each notice plugin file
  into its typed struct, serialize it as `hook_files` does, compare with the
  `include_str!` bytes); `resolve_matcher_follows_the_design` (the four cases of
  the algorithm, for both clients, and `context = false` without matcher on
  `session-start` gives `Some(None)` in Claude Code);
  `hook_files_is_empty_without_notice_and_without_entries`;
  `context_file_is_ascii_only`: a context hook with the text `café — ✓` gives,
  for both clients, a context file whose bytes are all ASCII
  (`bytes.is_ascii()`), that contains `café — ✓`, and whose parsed
  `additionalContext` equals `café — ✓`;
  `ascii_escape_writes_a_surrogate_pair_above_u_ffff`: the JSON string of `😀`
  becomes `"😀"` and parses back to `😀`.

- [ ] **Step 2: Run `cargo test --test cli_pack` and `cargo test --lib pack`,
      and confirm that the new tests fail**

- [ ] **Step 3: Implement the hook reading**

  In `src/pipeline/bundle_plan.rs`: collect `BundleOutput.hooks`; add
  `read_hook` (uses `discovery::find_hook_dir`, then
  `frontmatter::parse::<Hook>`). Update the doc of `BundleOutput`.

- [ ] **Step 4: Implement `src/pack/hooks.rs` and `build`**

  `src/pack/hooks.rs` as under Interfaces and the algorithm. In
  `src/pack.rs::build`: after `check_command_names`, call
  `hooks::check_hook_names(&output.bundles)?` and
  `let hooks = hooks::read_pack_hooks(&output.hooks)?;`; the rules branch no
  longer pushes the hook file; after it,
  `files.extend(hooks::hook_files(&hooks, !output.rules.is_empty(), client)?)`.
  Remove `hook_path` and `hook` from the tuple of `build`.

- [ ] **Step 5: Write the golden fixtures**

  Write the ten expected files with the bytes that `pack` writes and compare
  them with the golden examples above. Check that both notice entries are
  byte-identical to the plugin files.

- [ ] **Step 6: Update the documentation**

  - Add to §3.10 of `docs/topics/0001-content-format.md` the sentence "A client
    without a key gets no hook for that event." (it describes `pack` output; it
    was removed from Task 1).
  - `docs/book/commands.md` (section `upskill pack`) and §2.8 of
    `docs/specification/specification.md`, which hold the same text: the row
    "Notice hook, when rules target this client" becomes "Hook file, when rules
    target this client or hook items produce an entry" (same paths); a new row
    "Context hook texts" with `hooks/context/<event>.json` in both columns. A
    paragraph states: the context hooks of the hook items of the closure; the
    client event names (link to §3.10 of the content format); the merge for each
    event and distinct matcher in closure order with one blank line; the `-2`,
    `-3` file names; the JSON of each client, with every non-ASCII character
    escaped (plan decision 21); the shell commands of plan decision 12 for
    context entries (the Copilot CLI commands read the file by a path relative
    to the plugin directory, which was the working directory of a plugin hook in
    one local-install observation, technote T4, trial 5); that the generated
    hook does not call `upskill`; the matcher forms and the `session-start`
    default in Claude Code; the order notice, then context entries; that a hook
    file without hook items is the notice plugin file unchanged; the errors of
    `check_hook_names`, `read_hook` and `check_hook`, each exit 1 before any
    write. Replace "The hook is copied unchanged from the corresponding upskill
    notice plugin" and "An output with no rules for its client has neither init
    nor the notice hook" so that they stay true. Say nothing about command hooks
    (plan decision 16). "Known limits":
    - "The Claude Code documentation states a limit of 10,000 characters for the
      context text of a hook; no trial has checked it; no limit is known for
      Copilot CLI."
    - Copilot CLI documents `matcher` only for `notification`,
      `permissionRequest`, `postToolUse`, `preCompact`, `preToolUse` and
      `subagentStart`; no trial has used it.
    - The PowerShell command and a new `sessionStart` after compaction in
      Copilot CLI are not verified.
    - The text of a `subagent-start` hook reaches the subagent only: for Copilot
      CLI from technote T4, trial 5; for Claude Code from its documentation.
    - A user who installs the plugins of a bundle and of a bundle that it
      requires gets the context text two times (AD-0046, Consequences).

    Remove "general hook authoring" from the out-of-scope list and name hooks
    outside a plugin instead.
  - `docs/topics/0001-content-format.md` §3.10: the client event names (the
    table of the global constraints) and one paragraph on the context output
    that points to the `pack` section; §1 **Hook**: only `upskill pack` writes
    the context hooks of a hook item.
  - `docs/specification/specification.md` §1.1: the `Hook` purpose adds "Only
    `pack` writes its context hooks."
  - `docs/book/support-levels.md`, "Bundle plugin output": the context hooks of
    hook items are written only by `pack`, are experimental with plugin output,
    have no row in the table or in `doctor` (plan decision 19), and no smoke
    test has observed a generated hook yet.
  - `docs/book/recipes.md`, "Publish a bundle as client plugins", and
    `skills/upskill-cli/SKILL.md`, "Bundle plugin output and project rules": the
    plugin holds the context hooks of the hook items.
  - `skills/upskill-writing-bundles/SKILL.md`: the comment of `hooks:` becomes
    `# optional — only upskill pack renders them`.
  - `AGENTS.md`: under `pack.rs`, add a line for `pack/hooks.rs` ("Hook file and
    context files of the plugin output").

- [ ] **Step 7: Run `cargo test` and confirm that all tests pass**

  The implementer does not commit. The orchestrator runs `just fmt`,
  `just verify` and `just sanity`, and commits with
  `feat(pack): write the context hooks of hook items into plugins (#631)`.

### Task 4: `pack` writes command hooks

**Files:**

- Modify: `src/pack/hooks.rs`, `src/pipeline/bundle_plan.rs` (`HookSource`,
  `read_hook`), `tests/cli_pack.rs`
- Modify fixtures: `tests/fixtures/pack/registry/items/checks/HOOK.md`,
  `tests/fixtures/pack/expected/hookdemo-claude/hooks/hooks.json`,
  `tests/fixtures/pack/expected/hookdemo-copilot/hooks.json`
- Create fixtures: `tests/fixtures/pack/registry/items/checks/check.sh`,
  `tests/fixtures/pack/registry/items/checks/lib/common.sh`,
  `tests/fixtures/pack/expected/hookdemo-{claude,copilot}/hooks/edit-checks/check.sh`,
  `tests/fixtures/pack/expected/hookdemo-{claude,copilot}/hooks/edit-checks/lib/common.sh`
- Docs: `docs/book/commands.md`, `docs/specification/specification.md`,
  `docs/topics/0001-content-format.md`, `docs/book/support-levels.md`,
  `docs/book/recipes.md`, `skills/upskill-cli/SKILL.md`, `docs/roadmap.md`,
  `docs/wip/README.md`

`git check-ignore` reports none of the new paths (checked on 2026-11-25).

**Interfaces:**

- Consumes: everything of Task 3; `pipeline::CommandResource` and
  `filter_ignored` / `iter_item_resources` / `source_mode` as Task 3 of the
  command plan uses them in `read_command`.
- Produces: `HookSource.resources: Vec<CommandResource>` (the doc of
  `CommandResource` becomes "A resource file of a command or hook item
  directory"); `read_hook` fills it for every hook (plan decision 13 decides
  what is copied, not what is read).
- Removes the temporary error of plan decision 16.
- Produces in `src/pack/hooks.rs`:
  `fn command_entry(name: &str, run: &str, matcher: Option<String>, timeout: Option<u32>, client: PackClient)`
  returning a `ClaudeGroup` or `CopilotCommand` (one private enum or two
  functions; the implementer chooses), with the shell text of plan decision 12
  and `powershell: None`.
- Error text, wrapped in the context `` hook `{name}` of bundle `{bundle}` ``:
  ``event `{event}`: `run: {run}` names no copied file of the item directory (it is missing, an entry file, matched by `ignore`, or a symbolic link)``.

**Algorithm addition of `hook_files`:** after the context groups, for each hook
in closure order and each event with `run` in map order:
`resolve_matcher(settings, event, context = false, client)`; `None` skips; else
append one command entry to the event array. For each hook that got at least one
command entry for this client (plan decision 13), push `hooks/<name>/<rel>` for
each copied resource, with its mode. The `run` check runs for every hook with a
`run` event, in both clients, so the error does not depend on the client flags.
The event array order is notice, context, command.

**Fixture change.** `items/checks/HOOK.md` gets the event
`post-tool: { matcher: { claude: "Edit|Write", copilot: edit }, run: check.sh, timeout: 10 }`
and the section ``## post-tool\n\nRuns `check.sh` on the edited file.\n``.
`check.sh`:

```sh
#!/bin/sh
# Reads the hook input of the client that UPSKILL_CLIENT names.
. "$(dirname "$0")/lib/common.sh"
exit 0
```

`lib/common.sh`: `# Shared shell functions.` and a newline.

**Golden additions.** The Claude hook file gets, before `SessionStart`:

```json
"PostToolUse": [
  {
    "matcher": "Edit|Write",
    "hooks": [
      {
        "type": "command",
        "command": "UPSKILL_CLIENT=claude \"${CLAUDE_PLUGIN_ROOT}/hooks/edit-checks/check.sh\"",
        "timeout": 10
      }
    ]
  }
],
```

The Copilot hook file gets, before `sessionStart`:

```json
"postToolUse": [
  {
    "type": "command",
    "matcher": "edit",
    "bash": "UPSKILL_CLIENT=copilot ./hooks/edit-checks/check.sh",
    "timeoutSec": 10
  }
],
```

- [ ] **Step 1: Write the failing tests**

  `tests/cli_pack.rs`. Replace `pack_refuses_a_command_hook_in_this_task` with
  the tests below. Error tests assert exit 1, the text, and that `f.out` does
  not exist.

  - `pack_writes_the_hook_plugin_directories` (Task 3) now compares with the
    updated golden trees, which hold `hooks/edit-checks/check.sh` and
    `hooks/edit-checks/lib/common.sh` with the source bytes.
  - `pack_writes_a_command_hook_entry_that_sets_upskill_client`: pack
    `hookdemo`; the Claude `PostToolUse[0]` has `matcher == "Edit|Write"` and
    `hooks[0].command ==
    "UPSKILL_CLIENT=claude \"${CLAUDE_PLUGIN_ROOT}/hooks/edit-checks/check.sh\""`
    and `timeout == 10`; the Copilot `postToolUse[0]` has `matcher == "edit"`,
    `bash == "UPSKILL_CLIENT=copilot ./hooks/edit-checks/check.sh"`,
    `timeoutSec == 10` and no `powershell` key.
  - `pack_orders_notice_then_context_then_command_hooks`: `items/starter/` with
    `HOOK.md` (`session-start: { run: start.sh }`) and `start.sh`; bundle
    `ordered` (`requires: hookdemo`, `hooks: [starter]`); the Claude
    `SessionStart` groups are, in order: matcher `startup` (notice), matcher
    `startup|clear|compact` (context), no `matcher` key with a command that
    contains `/hooks/starter/start.sh`; the Copilot `sessionStart` entries are,
    in order: `bash` contains `upskill update --notice`, `bash` equals
    `cat hooks/context/session-start.json`, `bash` equals
    `UPSKILL_CLIENT=copilot ./hooks/starter/start.sh`.
  - `pack_of_a_bundle_with_only_a_command_hook_writes_a_hook_file`: a bundle
    without rules that lists `starter` only; both hook files hold one entry; no
    `hooks/context` directory exists.
  - `pack_does_not_copy_a_hook_without_a_command_hook`: pack `hookdemo`;
    `hooks/reminder` does not exist in either output.
  - `pack_copies_neither_an_entry_file_nor_an_ignored_file_of_a_command_hook`:
    `items/tool/` with `HOOK.md` (`ignore: [draft.md]`, a `run: go.sh` event),
    `RULE.md`, `draft.md`, `go.sh`; a bundle lists `hooks: [tool]` only; the
    files below `hooks/tool/` are exactly `go.sh`.
  - `pack_refuses_a_run_file_matched_by_ignore`: `ignore: [go.sh]` and
    `run: go.sh`; stderr contains ``hook `tool` of bundle`` and
    `` `run: go.sh` names no copied file `` (Review focus 5).
  - `pack_keeps_the_mode_of_a_command_hook_script` (`#[cfg(unix)]`): set mode
    `0o755` on `items/checks/check.sh` in the temporary registry; pack
    `hookdemo`; the mode of `hooks/edit-checks/check.sh` in both outputs, masked
    with `0o777`, is `0o755`.
  - `pack_writes_a_command_hook_only_for_the_client_of_its_map_matcher`:
    `post-tool: { matcher: { copilot: edit }, run: go.sh }` in `items/tool/`
    with `go.sh`, a bundle without rules that lists `hooks: [tool]`; the Copilot
    hook file has one `postToolUse` entry and the Copilot output holds
    `hooks/tool/go.sh`; the Claude output has no hook file and no `hooks/tool`
    directory (plan decision 13: the directory is copied only for a client that
    gets an entry).

- [ ] **Step 2: Run `cargo test --test cli_pack` and confirm that the new tests
      fail**

- [ ] **Step 3: Implement**

  `read_hook` reads the resources; `hook_files` writes the command entries and
  copies the resources as in the algorithm addition; check that each `run` of a
  copied hook is the `rel` of one resource, else the error above. Remove the
  temporary error and its test.

- [ ] **Step 4: Update the golden fixtures**

  Write the fixture changes and the expected files with the bytes that `pack`
  writes; compare the two hook files with the golden additions above.

- [ ] **Step 5: Update the documentation**

  - `docs/book/commands.md` (section `upskill pack`) and §2.8 of
    `docs/specification/specification.md`: a row "Command hook scripts" with
    `hooks/<name>/` in both columns; widen the Task 3 sentences that name
    context hooks only ("the context hooks of the hook items of the closure") to
    all hook items; a paragraph: the item directory of a hook with a command
    hook is copied, without its entry files and its `ignore` matches and with
    the file modes, into the output of each client that gets at least one
    command entry of that item; one entry for each command hook in closure
    order, after the context entries; the shell commands of plan decision 12
    (the variable is set by a shell prefix); `UPSKILL_CLIENT` is `claude` or
    `copilot`; `timeout` becomes `timeout` or `timeoutSec`; the Copilot CLI
    entry has a `bash` command only; a command hook has no default matcher; the
    `run` error. "Known limits": a command hook script must be executable, must
    read `UPSKILL_CLIENT` and must handle the input and output format of each
    client; a command hook does not run in PowerShell in Copilot CLI; in Claude
    Code on Windows, a command hook needs Git Bash (Claude Code documentation;
    not tested).
  - `docs/topics/0001-content-format.md` §3.10: one sentence on the command hook
    output that points to the `pack` section; §1 **Hook**: "only `upskill pack`
    writes the context hooks of a hook item" becomes "only `upskill pack`
    renders a hook item".
  - `docs/specification/specification.md` §1.1: the `Hook` purpose becomes "…
    Only `pack` renders it."; `docs/book/support-levels.md`,
    `docs/book/recipes.md` and `skills/upskill-cli/SKILL.md`: the Task 3
    sentences about context hooks name command hooks too.
  - `docs/roadmap.md`, stage 2: the sentence that lists implemented stories
    names #631; the "Hook item kind" bullet ends with "Implemented in #631:
    `pack` writes context hooks as static JSON files and copies command hook
    scripts; `add` and `update` skip hook items."
  - `docs/wip/README.md`: story #631 is implemented, and this plan is listed as
    an active document until it is gardened.

- [ ] **Step 6: Run `cargo test` and confirm that all tests pass**

  The implementer does not commit. The orchestrator runs `just fmt`,
  `just verify` and `just sanity`, and commits with
  `feat(pack): copy command hook scripts and write their entries (#631)`.

## Self-review

- **Spec coverage.** Criterion 1 (`items.hooks`, discovery, content format,
  fmt): Tasks 1 and 2. Criterion 2 (lint errors): Task 1. Criterion 3
  (`new hook`): Task 1. Criterion 4 (static JSON for each event and matcher, no
  `upskill`): Task 3. Criterion 5 (copy and `UPSKILL_CLIENT`): Task 4. Criterion
  6 (order, hook file without rules): Tasks 3 and 4. Criterion 7 (matcher forms,
  default): Task 3. Criterion 8 (skip and named error): Task 2. Criterion 9
  (format spec, command guide, support levels): Tasks 1 to 4. Design "Name":
  plan decisions 1 and 14. Design "Kind lists": `src/pipeline/discovery.rs`
  (Task 2), `src/scaffold.rs` (Task 1), content format (Task 1);
  `src/pipeline/mod.rs` holds only `ItemKind::entry`, which does not learn the
  kind. AD-0046 Decision 11 (closure): `BundleOutput.hooks` in closure order.
  Decision 12: the Copilot hook file stays at the root.
- **Type consistency.** `HookSource`, `BundleHook`, `PackHook`,
  `resolve_matcher`, `hook_files` and `read_hook` keep one signature across
  Tasks 3 and 4; Task 4 adds only `HookSource.resources`.
- **Review focus.** Each of the five items has a named test in its task.
