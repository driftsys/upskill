# Plugin-first setup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the packed `upskill` plugin the single setup path: renamed skill
and bundle, a setup hook that offers the CLI install, a CLI release check in
`update --notice`, and an installer that points to the plugin.

**Architecture:** Content changes in `skills/` (rename, new hook item); one new
Rust module `src/release.rs` for tag parsing and version comparison; the notice
cache gains a release state; `main.rs` wires the release check into
`notice_lines`; `fetch.rs` gains a tag listing. Publishing to metapowers is a
release step, not code.

**Tech Stack:** Rust 2024 (MSRV 1.85), no new dependency; POSIX sh for the hook
script and `install.sh`.

**Spec:** `docs/archive/specs/2026-11-25-plugin-first-setup-design.md`

## Global Constraints

- No new crate. No semver crate: parse `vMAJOR.MINOR.PATCH` by hand.
- `update --notice` exits 0 in every case after argument parsing, writes nothing
  to stderr, prints nothing on any network failure.
- Release check period: 24 hours (`notice::NETWORK_PERIOD_SECS`).
- Release check time limit: 3 seconds, separate from `NOTICE_TIME_LIMIT` (5 s).
- Opt-out: `UPSKILL_NO_UPDATE_CHECK` set to a nonempty value skips the release
  check.
- Release source URL: `https://github.com/driftsys/upskill`.
- Release line, exact text:
  `upskill <latest> is available (you have <current>): curl -fsSL https://raw.githubusercontent.com/driftsys/upskill/main/install.sh | sh`
- Missing-CLI message, exact text (two lines):
  `The upskill CLI is not installed. Install it with:` /
  `curl -fsSL https://raw.githubusercontent.com/driftsys/upskill/main/install.sh | sh`
- Only `main.rs` writes to stdout/stderr. Zero warnings (clippy on tests too:
  run `just sanity` before push).
- Integration tests use `common::upskill_cmd(&home)`.
- Every Markdown file: `just fmt` before commit. Commits: Conventional Commits,
  ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. A tag list with pre-release or odd tags (`v1.0.0-rc1`, `v1.2`, `latest`,
   `v1.0.0^{}`): only strict `vX.Y.Z` count; the peeled `^{}` duplicate is
   ignored. Pinned by Task 2 unit tests.
2. A development build newer than every tag (`0.10.0-dev` binary vs `v0.9.4`):
   no line. Pinned by Task 2 (`current_newer_than_latest_prints_nothing`).
3. A notice cache written by the previous version (no `release` key): loads with
   defaults, no crash, check runs. Pinned by Task 2
   (`old_cache_without_release_key_loads`).
4. Two concurrent `--notice` runs saving the cache: the release state must
   survive the reload-merge-save path. Pinned by Task 2
   (`merge_roots_keeps_newer_release_state`).
5. The setup script when `$HOME` cache dir is not writable: still prints the
   message (or nothing), exits 0. Pinned by Task 4
   (`setup_unwritable_cache_exits_zero`).

## Decisions made in this plan (not in the spec)

- **Hook matcher.** The hook format cannot give Claude `startup` and Copilot no
  matcher (a matcher map without a `copilot` key drops Copilot,
  `src/pack/hooks.rs:354-371`). The setup hook has **no matcher**. Claude Code
  then runs it also on resume, clear and compact. This is harmless: the notice
  prints each set once, and the missing-CLI message prints once a day.
- **Test URL rewrite.** Release-check integration tests rewrite
  `https://github.com/driftsys/upskill` to a local repo with the same
  `GIT_CONFIG_*` `insteadOf` variables `tests/cli_update_notice.rs` already
  uses. No new test-only env var.

---

### Task 1: Rename the skill and the bundle

**Files:**

- Move: `skills/upskill-cli/` → `skills/upskill-managing-content/` (`name:` in
  `SKILL.md` l.3 becomes `upskill-managing-content`)
- Move: `skills/prompt-engineering.bundle.yaml` → `skills/upskill.bundle.yaml`
  (`name: upskill`; item list entry renamed)
- Modify: every live mention: `skills/README.md`, `skills/NOTICE`,
  `skills/upskill-writing-bundles/SKILL.md`,
  `skills/upskill-evaluating-prompts/SKILL.md`,
  `skills/upskill-prompt-design/SKILL.md`, `AGENTS.md` (`<available_skills>`
  block), `docs/book/` (grep). Do **not** edit `CHANGELOG.md`, `docs/archive/`,
  `docs/adr/0007` (historical records).
- Test: `tests/cli_update.rs`

**Interfaces:** Produces the bundle name `upskill` and skill name
`upskill-managing-content`, used by Tasks 4 and 5.

- [ ] **Step 1: Write the failing test**
      `update_removes_an_item_renamed_in_the_source` in `tests/cli_update.rs`:
      local source with skill `old-name`; `add --project --claude`; rename the
      folder and `name:` to `new-name`; `update --project`. Assert
      `.claude/skills/old-name` is gone, `.claude/skills/new-name/SKILL.md`
      exists, and the lockfile lists `new-name` and not `old-name`.
- [ ] **Step 2: Run it.**
      `cargo test --test cli_update update_removes_an_item_renamed`. Expected:
      PASS or FAIL. If it passes, the behaviour already exists and the test pins
      it; if it fails, stop and report — fixing `update` is out of scope for
      this plan.
- [ ] **Step 3: Do the renames and mention updates** (`git mv`, then edit).
- [ ] **Step 4: Verify.**
      `grep -rn 'upskill-cli\|prompt-engineering' skills AGENTS.md docs/book README.md`
      → no output. `cargo run -q -- lint skills` → `0 findings`.
- [ ] **Step 5: Commit**
      `refactor(skills): rename upskill-cli to upskill-managing-content and the bundle to upskill`.

### Task 2: Release check core (no wiring)

**Files:**

- Create: `src/release.rs` (declare in `src/lib.rs`)
- Modify: `src/notice.rs` (cache state, merge, decide), `src/fetch.rs` (tag
  listing)
- Test: unit tests in `src/release.rs`, `src/notice.rs`, `src/fetch.rs`

**Interfaces (produced):**

- `release::Version` —
  `#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)] pub struct Version { pub major: u64, pub minor: u64, pub patch: u64 }`
  with `Display` as `X.Y.Z`.
- `release::parse_version(s: &str) -> Option<Version>` — accepts `X.Y.Z` with an
  optional leading `v`; anything else (`-rc1`, two parts, text) → `None`.
- `release::latest_tag(ls_remote_output: &str) -> Option<Version>` — reads lines
  `<sha>\trefs/tags/<tag>`, skips `^{}` lines, returns the greatest parsed
  version.
- `release::RELEASE_URL: &str = "https://github.com/driftsys/upskill"`;
  `release::TIME_LIMIT: Duration = 3 s`;
  `release::OPT_OUT_ENV: &str = "UPSKILL_NO_UPDATE_CHECK"`.
- `fetch::ls_remote_tags(url: &str, deadline: Instant) -> Result<String, RunError>`
  — `git ls-remote --tags -- <url>`, same env hygiene and `run_until` as
  `ls_remote`.
- `notice::ReleaseState` —
  `#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)] #[serde(default)] pub struct ReleaseState { pub checked_at: u64, pub latest: Option<String>, pub reported: Option<String> }`.
- `NoticeCache` gains `pub release: ReleaseState` (serde default).
- `notice::needs_release_check(cache: &NoticeCache, now: u64) -> bool` — true
  when `now >= checked_at + NETWORK_PERIOD_SECS` or `now < checked_at`.
- `notice::record_release(cache: &mut NoticeCache, latest: Version, now: u64)`.
- `notice::decide_release(cache: &mut NoticeCache, current: Version, repeat: bool) -> Option<String>`
  — returns the exact release line when `latest > current` and (`repeat` or
  `reported != latest`); then sets `reported = latest`.
- `merge_roots` keeps the `release` state with the greater `checked_at` (ties:
  `mine`).

- [ ] **Step 1: Write failing unit tests** in `src/release.rs`:
      `parse_accepts_v_prefix_and_plain` (`"v0.9.4"`, `"0.9.4"` →
      `Version{0,9,4}`); `parse_rejects_prerelease_and_short` (`"v1.0.0-rc1"`,
      `"v1.2"`, `"latest"` → `None`); `latest_tag_ignores_peeled_and_odd_tags`
      (input with `v0.9.3`, `v0.9.4`, `v0.9.4^{}`, `v1.0.0-rc1`, `latest` →
      `0.9.4`); `latest_tag_orders_numerically` (`v0.9.10` > `v0.9.9`);
      `latest_tag_of_empty_output_is_none`.
- [ ] **Step 2: Write failing unit tests** in `src/notice.rs`:
      `old_cache_without_release_key_loads` (JSON with only `scopes` → default
      `release`); `release_check_due_after_24h_and_on_clock_skew`;
      `decide_release_prints_once_then_silent` (latest 0.10.0, current 0.9.4 →
      exact line, second call `None`); `decide_release_repeat_prints_again`;
      `current_newer_than_latest_prints_nothing` (latest 0.9.4, current 0.10.0 →
      `None`); `equal_versions_print_nothing`;
      `merge_roots_keeps_newer_release_state`.
- [ ] **Step 3: Write failing unit test** in `src/fetch.rs`:
      `ls_remote_tags_command_lists_tags` — the built command's args are
      `["ls-remote", "--tags", "--", url]` and `GIT_TERMINAL_PROMPT=0` is set
      (mirror the existing `ls_remote_command` test).
- [ ] **Step 4: Run** `cargo test --lib release notice fetch` → the new tests
      FAIL to compile/assert.
- [ ] **Step 5: Implement** the interfaces above.
- [ ] **Step 6: Run** `cargo test --lib` → PASS; `just lint` → clean.
- [ ] **Step 7: Commit**
      `feat(notice): add the CLI release check state and tag parsing`.

### Task 3: Wire the release check into `update --notice`

**Files:**

- Modify: `src/main.rs` (`notice_lines`, l.1302-1358)
- Modify: `docs/book/commands.md` (`#### update --notice` section), `AGENTS.md`
  (the `update --notice` paragraph under Install layout)
- Test: `tests/cli_update_notice.rs`

**Interfaces:** Consumes everything Task 2 produces. `current` is
`release::parse_version(env!("CARGO_PKG_VERSION"))`.

Order inside `notice_lines`, after the content lines are printed and before the
cache reload: if `OPT_OUT_ENV` is unset or empty and `needs_release_check`, call
`fetch::ls_remote_tags(RELEASE_URL, Instant::now() + release::TIME_LIMIT)`; on
`Ok` with `latest_tag(..) = Some(v)` call `record_release`; on any error or
`None` record nothing. Then, unless `style::is_quiet()`, print
`decide_release(..)` if `Some`. A busy lock still returns before all of this.

- [ ] **Step 1: Write failing tests** in `tests/cli_update_notice.rs`, using a
      local upstream repo with tags and the `insteadOf` rewrite of
      `https://github.com/driftsys/upskill`:
      `release_line_prints_once_for_newer_tag` (tag `v99.0.0` → stdout contains
      the exact release line with `99.0.0` and the current version; second run →
      no release line); `release_line_repeats_with_repeat_flag`;
      `no_release_line_for_equal_or_older_tag` (tags `v0.0.1` and the current
      version); `release_check_skipped_with_opt_out`
      (`UPSKILL_NO_UPDATE_CHECK=1`, tag `v99.0.0` → no line, and no
      `release.checked_at` in the cache);
      `release_check_network_failure_is_silent` (rewrite to a missing path →
      stdout empty, stderr empty, exit 0, `release.checked_at` stays 0);
      `release_check_runs_at_most_once_a_day` (after a first run, add tag
      `v100.0.0`; second run within 24 h does not report `100.0.0`; after
      setting `release.checked_at` back 25 h, it does).
- [ ] **Step 2: Run** `cargo test --test cli_update_notice release` → FAIL.
- [ ] **Step 3: Implement** the wiring above. Ensure existing tests that do not
      set up the release repo set `UPSKILL_NO_UPDATE_CHECK=1` in the shared
      `command()` helper only if they would otherwise hit the network; prefer
      the helper always rewriting the release URL to a missing local path, so no
      test reaches github.com.
- [ ] **Step 4: Run** `cargo test --test cli_update_notice` → all PASS;
      `cargo test` → all PASS.
- [ ] **Step 5: Update docs** (commands.md: release check, its cache key, its
      3-second limit, the opt-out, that `update --check` does not include it;
      AGENTS.md: one sentence in the `update --notice` paragraph). `just fmt`.
- [ ] **Step 6: Commit**
      `feat(notice): report a newer upskill release in update --notice`.

### Task 4: Setup hook item

**Files:**

- Create: `skills/upskill-setup/HOOK.md`,
  `skills/upskill-setup/upskill-setup.sh` (mode 755)
- Modify: `skills/upskill.bundle.yaml` (`items.hooks: [upskill-setup]`)
- Test: create `tests/plugins_setup_hook.rs` (`#![cfg(unix)]`), extend
  `tests/cli_pack.rs`

**Interfaces:** Consumes the bundle from Task 1. The script reads
`UPSKILL_CLIENT` (`claude` | `copilot`) and `XDG_CACHE_HOME`/`HOME`.

`HOOK.md` frontmatter:

```yaml
schema: 1
name: upskill-setup
description: Offer to install the upskill CLI and report upskill updates at session start
events:
  session-start:
    run: upskill-setup.sh
    timeout: 30
```

Body: one `## session-start` section stating what the script does.

Script behaviour:

- If `COPILOT_PROJECT_DIR` is set, `cd` there (as the current Copilot plugin).
- No `upskill` on `PATH`: marker `<cache>/upskill/setup-offered` holds
  `date +%Y-%m-%d`; if it equals today, exit 0 silently; else print the
  missing-CLI message as client JSON, then try to write the marker (failure
  ignored).
- `upskill` present: `upskill update --notice` (claude) or
  `upskill update --notice --repeat` (copilot); empty or failed → exit 0
  silently; else print as client JSON.
- Client JSON and escaping: copy exactly from
  `plugins/upskill-claude/hooks/hooks.json` (claude: `systemMessage` +
  `hookSpecificOutput.additionalContext`) and
  `plugins/upskill-copilot/hooks.json` (copilot: `additionalContext` prefixed
  with `Tell the user at the start of your first reply:`).
- Unknown or empty `UPSKILL_CLIENT`: exit 0 silently.
- Every path exits 0.

- [ ] **Step 1: Write failing tests** in `tests/plugins_setup_hook.rs` (pattern
      of `tests/plugins_notice_hook.rs`: `env_clear`, fake `upskill` in
      `<tmp>/bin`, `PATH=<bin>:/usr/bin:/bin`):
      `setup_missing_cli_prints_install_message_for_claude` (valid JSON;
      `systemMessage` contains both exact message lines);
      `setup_missing_cli_prints_install_message_for_copilot`;
      `setup_missing_cli_prints_once_a_day` (second run same day → empty stdout;
      marker with yesterday's date → prints);
      `setup_unwritable_cache_exits_zero` (cache dir read-only → exit 0);
      `setup_passes_notice_through_for_claude` (fake prints one line → JSON with
      that line; args file shows `update --notice`);
      `setup_uses_repeat_for_copilot` (args `update --notice --repeat`);
      `setup_silent_when_notice_empty_or_fails`;
      `setup_unknown_client_is_silent`.
- [ ] **Step 2: Write failing test** in `tests/cli_pack.rs`:
      `pack_of_the_upskill_bundle_writes_skills_and_setup_hook` — copy the
      repo's `skills/` to a tempdir registry,
      `pack upskill --source driftsys/upskill@v1 --out <tmp>/out`; assert
      `out/upskill-claude/skills/` holds the seven skill folders (including
      `upskill-managing-content`), Claude `hooks/hooks.json` has a
      `SessionStart` entry whose command ends with
      `hooks/upskill-setup/upskill-setup.sh`, and Copilot `hooks.json` has a
      `sessionStart` entry with `./hooks/upskill-setup/upskill-setup.sh`.
- [ ] **Step 3: Run** both test files → FAIL.
- [ ] **Step 4: Implement** `HOOK.md`, the script, the bundle entry.
- [ ] **Step 5: Run** `cargo test --test plugins_setup_hook --test cli_pack` →
      PASS; `shellcheck skills/upskill-setup/upskill-setup.sh` → clean;
      `cargo run -q -- lint skills` → `0 findings`.
- [ ] **Step 6: Commit** `feat(skills): add the upskill-setup session hook`.

### Task 5: Installer, docs and decision record

**Files:**

- Modify: `install.sh` (after the final `--version` line)
- Modify: `README.md`, `docs/book/getting-started.md` (plugin first;
  `install.sh` + `upskill add` for opencode and no-plugin users; the
  duplicate-skills note with `upskill remove`)
- Modify: `AGENTS.md` (`plugins/` paragraph: no longer published standalone;
  `pack` embeds it for bundles with rules; the published plugin is generated by
  `pack` from `skills/upskill.bundle.yaml`)
- Modify: `CONTRIBUTING.md` (release step: check out the tag, run
  `upskill pack upskill --source driftsys/upskill@vX.Y.Z --out ../metapowers/plugins --catalog ../metapowers --force`,
  commit in metapowers)
- Create: `docs/adr/0047-plugin-first-setup.md` (amends AD-0044; records
  approach 1 and its rejected alternatives from the spec); add to
  `docs/adr/README.md`
- Test: the existing installer test under `scripts/tests/` if one covers
  `install.sh` output; otherwise `shellcheck install.sh` only

Installer text, exact, on stderr:

```text
Set up upskill in your AI client (skills and update notices):
  claude  plugin marketplace add driftsys/metapowers && claude  plugin install upskill@metapowers
  copilot plugin marketplace add driftsys/metapowers && copilot plugin install upskill@metapowers
opencode: upskill add driftsys/upskill --global --opencode
```

- [ ] **Step 1:** If `scripts/tests/` has an `install.sh` test, add an assertion
      for the first line of the text above and run it → FAIL.
- [ ] **Step 2:** Edit `install.sh`. Run `shellcheck install.sh` → clean; the
      test from Step 1 → PASS.
- [ ] **Step 3:** Edit the docs and write the ADR. `just fmt`.
- [ ] **Step 4:** `just verify` → PASS; `just sanity` → PASS.
- [ ] **Step 5: Commit**
      `docs: make the upskill plugin the setup path and record the decision`.

### After merge (not part of the branch)

- Release, then run the `CONTRIBUTING.md` pack step into metapowers and open a
  PR there.
- On the user's machine: `upskill remove` the seven skills from the global scope
  once the new plugin is installed.
- Garden `docs/wip/` (sdd-gardening) before the PR to `main` lands.
