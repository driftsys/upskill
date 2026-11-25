# Several bundles and catalog generation in `pack`: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

Date: 2026-11-25. Story: [#635](https://github.com/driftsys/upskill/issues/635).

**Goal:** `upskill pack` accepts several bundle names or `--all`, and with
`--catalog <root>` it writes `<root>/.claude-plugin/marketplace.json` and
`<root>/.github/plugin/marketplace.json` after the plugin directories are in
place.

**Architecture:** Task 1 moves the `pack` arguments into a clap `Args` struct,
selects the bundles with a new `pack::select_bundles`, and calls the existing
`pack::build` once for each bundle and client before one call of
`pack::write_plugin_dirs`, so the run stays all-or-nothing for the plugin
directories. Task 2 adds a new module `src/catalog.rs`: it checks the paths and
reads the existing marketplace files before any build, renders the new file
bytes before any write, and writes them after `write_plugin_dirs` returns. No
new dependency.

**Tech stack:** Rust 2024, `clap` 4 derive, `serde`, `serde_json` (without the
`preserve_order` feature), `thiserror`, `anyhow`; integration tests with
`assert_cmd` through `common::upskill_cmd`.

**Spec:** [plugin MVP design](../specs/2026-11-24-plugin-mvp-design.md),
sections 6 and 7 and "Left to the builder";
[AD-0046](../../adr/0046-plugin-mvp-components.md), Decisions 13 and 14 and the
amendments to AD-0044 Decisions 1 and 8;
[AD-0044](../../adr/0044-plugin-output-of-a-bundle.md) for the current `pack`.

## Starting point

This plan starts after story #631 (hooks) and #632 Task 3 (commands and `init`
as skills) are on the branch. It uses only these two functions of `src/pack.rs`:

- `pub fn build(registry_root: &Path, bundle: &str, source: &str, client: PackClient) -> Result<PluginDir>`
- `pub fn write_plugin_dirs(out: &Path, dirs: &[PluginDir], force: bool) -> Result<Vec<(PathBuf, usize)>>`

and the field `PluginDir::dir_name` (`<bundle>-claude` or `<bundle>-copilot`).
If either signature differs when a task starts, adapt the calls in `run_pack`
and change nothing else in `build`. No test of this plan asserts a file count or
the content of `init`, because #631 and #632 change both.

## Global constraints

- `pack <bundle>...` accepts several bundle names; `--all` selects every bundle
  of the source registry, which is the current directory (AD-0044, Decision 8,
  as amended).
- An entry holds `name` (the bundle name), `source` (the relative path from the
  catalog root to the plugin directory, with the prefix `./`) and `description`.
  It holds no `version`. Entries are sorted by name.
- An existing file keeps its top-level fields and every entry whose name is not
  a packed bundle. With `--all`, an entry whose `source` is inside `--out` and
  names no packed plugin directory is removed. `pack` deletes no plugin
  directory.
- A new file needs `--catalog-name` and `--catalog-owner`; without them `pack`
  stops with a usage error (exit 2).
- `--out` must be inside `--catalog`. A client that the client flags exclude
  gets no marketplace file.
- The marketplace files are written after every plugin directory is in place.
- Output is byte stable: two runs on the same input write equal bytes.
- Usage errors exit 2; other errors exit 1. Only `main.rs` writes to stdout and
  stderr.
- Zero warnings: `just verify` and `just sanity` pass.
- Integration tests build the command with `common::upskill_cmd(&home)`.
- Pre-1.0: no backward compatibility, no deprecated argument form.
- Comments, documentation and messages use plain, literal English. Markdown is
  wrapped at 80 columns.

## Plan decisions

Each decision fills a point that the design and AD-0046 leave open.

1. **The `pack` arguments move into
   `#[derive(clap::Args)] pub struct
   PackArgs`** (Task 1). Reason: with the
   catalog flags `run_pack` would take ten parameters, over the clippy
   `too_many_arguments` limit (AGENTS.md).
2. **Bundle names are sorted and repeated names are removed.** Reason: the
   output and the stdout lines then have one order for named bundles and for
   `--all` (`parse::bundle::discover` sorts by name), and a repeated name would
   give two `PluginDir` values with one target, which `write_plugin_dirs` cannot
   swap.
3. **Every bundle is built before any directory is written, with one call of
   `write_plugin_dirs`.** Reason: AD-0044 Decision 2 makes the write of the
   selected directories all-or-nothing; one call keeps that for several bundles.
4. **`--all` on a registry without bundles fails with exit 1**
   (`no bundle in <root>`). Reason: an empty run is almost always a wrong
   current directory, and with `--catalog` it would rewrite the files with no
   packed entry.
5. **With `--all`, a registry bundle whose name fails `validate_bundle_name`
   fails the run with exit 1.** Reason: the name comes from the registry, not
   from the command line, so it is not a usage error; the name is written into a
   command line that a model runs (AD-0044, Decision 8).
6. **`--out` is compared with `--catalog` lexically**: both are joined to the
   current directory, `.` and `..` are removed without reading the file system,
   and `--out` must equal `--catalog` or start with all its components. Another
   `--out` is a usage error (exit 2) whose message ends with "the paths are
   compared as written, without following symbolic links". Reason: `--out` may
   not exist yet, and the `source` path is computed from the same two lexical
   paths. A mix of a symbolic link path and its target (for example `/tmp` and
   `/private/tmp` on macOS) is refused; the user passes both paths in one form.
7. **`--out` equal to `--catalog` is accepted**; the `source` is then
   `./<bundle>-<client>`. Reason: the design requires "inside" and gives no
   reason to refuse the root itself. Consequence, stated in the command guide
   and the specification: with `--out` equal to `--catalog`, `--all` removes
   every entry with a relative-path `source` that is not packed in the run.
8. **`source` joins the path components with `/` on every platform.** Reason:
   the marketplace formats use URL-style relative paths, and the output must be
   byte stable across platforms.
9. **`--claude` alone reads and writes only the Claude Code file, `--copilot`
   alone only the Copilot CLI file; no flag or both flags handle both.** Reason:
   acceptance criterion "A client that the client flags exclude gets no file";
   the excluded file is not read, checked or changed.
10. **`--catalog-name` and `--catalog-owner` are used only for a file that does
    not exist.** An existing file keeps its `name` and `owner` unchanged when
    the flags are given. Reason: one run can create one file and update the
    other (for example a first Copilot output), and the design says that an
    existing file keeps its top-level fields. Both flags require `--catalog` and
    a non-empty value.
11. **An entry of a packed bundle replaces the whole earlier entry with that
    name**, including fields such as `version`. Reason: the design says that
    `pack` replaces the entries of the packed bundles, and an entry holds no
    `version`.
12. **Key order of the written file**: the top-level fields other than `plugins`
    in sorted key order, then `plugins`. Every entry, packed or kept, is written
    as `name`, `source`, `description`, then its other keys in sorted order.
    Nested objects (for example an object `source` or `owner`) have sorted keys.
    Pretty-printed with two spaces and one trailing newline. Reason: the crate
    does not enable the `serde_json` feature `preserve_order`, and enabling it
    would change the key order of every other JSON file that upskill writes. A
    new file then has the order `name`, `owner`, `plugins`, and an entry with
    exactly `name`, `source`, `description`, as in the hand-written files of
    `driftsys/metapowers`, is not rewritten.
13. **The `description` of an entry is `Bundle::description` of the packed
    bundle**, taken from `select_bundles`. Reason: it is the value that `build`
    writes into the plugin manifest, and `PluginDir` does not carry it, so the
    catalog does not depend on the internals of `build`.
14. **"Inside `--out`" for an existing `source`**: the `source` is a string; a
    leading `./` is removed, empty and `.` components are ignored, and the
    remaining components must start with all components of `--out` relative to
    the root and have at least one more component. A `source` with a `..`
    component, an absolute path or a value that is not a string (for example
    `{"source": "github", ...}`) is never inside `--out`. "Names no packed
    plugin directory" compares the normalized `source` with the `source` of each
    packed directory of the same client as the file. Reason: each file lists the
    outputs of its own client (design section 6).
15. **The marketplace files are read and checked before any build, and their
    bytes are computed before `write_plugin_dirs`.** A file that is not a JSON
    object, whose `plugins` is not an array, or with an entry without a string
    `name` fails with exit 1 and nothing is written. A file without `plugins` is
    read as an empty list. Reason: an error found after the plugin directories
    are replaced would leave the catalog different from them.
16. **The marketplace files do not need `--force`.** Reason: the merge keeps
    every foreign entry and top-level field; the CI command of design section 7
    passes `--force` for the plugin directories only.
17. **stdout**: after the `Wrote <dir> (<n> files)` lines, for each written
    marketplace file (Claude Code first), one line `Removed <name> from <path>`
    for each removed entry in name order, then `Wrote <path>`. `--quiet`
    suppresses all of them. Reason: a removal changes a file that a person
    edits, so it is reported.
18. **Each marketplace file is written to a temporary file in its own directory
    (`marketplace.json.tmp`) and renamed into place**, after `create_dir_all` of
    its parent, as `src/lockfile.rs` (`Lockfile::save`) and `src/notice.rs` (the
    cache save) do; on a failure the temporary file is removed. The
    `Wrote <dir> (<n> files)` lines are printed before `write_updates` is
    called. A failure while writing a marketplace file exits 1 and leaves the
    plugin directories in place. Reason: a failed write then cannot leave a
    truncated, invalid file, which decision 15 would refuse in the next run; the
    earlier file stays until the rename, and a new run writes the files again.
    The directory lines tell the user which directories are already in place
    when the catalog write fails.
19. **The heading of the command guide stays
    `` `upskill pack <bundle>... --source <source> --out <dir>` ``.** Reason:
    mdBook and GitHub drop the dots, so the anchor
    `#upskill-pack-bundle---source-source---out-dir` that `docs/book/recipes.md`
    and `docs/archive/README.md` use stays valid.
20. **`src/pipeline/discovery.rs::normalize_lexically` becomes `pub(crate)` and
    is re-exported as `crate::pipeline::normalize_lexically`.** Reason:
    `src/catalog.rs` needs the same function; a copy would duplicate it.
21. **The catalog tests go into `tests/cli_pack.rs`.** Reason: they reuse its
    `Fixture` and `tree` helpers; AGENTS.md prefers extending the matching file.

## Notes on the records

- **Keep rule.** An existing entry is kept by name: every entry whose name is
  not a packed bundle stays. With `--all`, an entry is also removed by the
  location of its `source`: inside `--out` and naming no plugin directory packed
  in the run. A marketplace file records no origin of its entries, so AD-0046
  Decision 13's "the entries that `pack` did not write" can only mean this rule;
  the design and the acceptance criteria state it in this form.
- **`--out` stays required.** AD-0046 Decision 14 and design section 7 give the
  CI command `upskill pack --all --force --catalog .`, which cannot run, because
  `--source` and `--out` are required (design section 6 and the code). Task 2
  corrects the command in both records to
  `upskill pack --all --force --source <source> --out <dir> --catalog .`. This
  corrects the command and does not change the decision.

## Review focus

1. `--out` and `--catalog` given as relative paths below the registry, as in the
   corrected CI command of design section 7 (here `--out plugins --catalog .`):
   the `source` is `./plugins/<bundle>-<client>`, and a second run with
   `--force` writes equal bytes (test in Task 2; the plugin part in Task 1).
2. The `--all` removal rule against hand-written entries: an entry whose
   `source` is inside `--out` and names no directory packed in the run is
   removed and reported; an entry whose `source` is outside `--out`, or is an
   object, is kept (test in Task 2). A real marketplace keeps hand-written
   plugins outside `--out` (the recipe of Task 2).
3. A failure in the second bundle (unknown name, existing non-empty directory)
   writes no directory of the first bundle (tests in Task 1).
4. An existing marketplace file that is invalid JSON: exit 1, no plugin
   directory written, the file unchanged (test in Task 2).
5. `--out` that shares a string prefix with `--catalog` but is not below it
   (`<tmp>/market2/plugins` with `--catalog <tmp>/market`): exit 2 (unit test in
   Task 2).

---

### Task 1: several bundle names and `--all`

**Files:**

- Modify: `src/cli.rs` (`Commands::Pack` and a new `PackArgs`)
- Modify: `src/main.rs` (dispatch of `Commands::Pack`, `run_pack`)
- Modify: `src/pack.rs` (new `SelectedBundle`, `select_bundles`, unit test)
- Modify: `tests/cli_pack.rs`
- Modify: `docs/book/commands.md` (heading and section of `upskill pack`),
  `docs/specification/specification.md` (section 2.8), `docs/book/recipes.md`
  (section "Publish a bundle as client plugins"), `skills/upskill-cli/SKILL.md`
  (section "Bundle plugin output and project rules")

**Interfaces:**

- Produces, in `src/cli.rs`:

  ```rust
  #[derive(clap::Args, Debug)]
  pub struct PackArgs {
      /// Bundles to build. Repeated names are built once.
      #[arg(required_unless_present = "all", conflicts_with = "all")]
      pub bundles: Vec<String>,
      /// Build every bundle of the source registry in the current directory.
      #[arg(long)]
      pub all: bool,
      #[arg(long)]
      pub source: String,
      #[arg(long)]
      pub out: PathBuf,
      #[arg(long)]
      pub force: bool,
      /// Write the Claude Code plugin directory.
      #[arg(long)]
      pub claude: bool,
      /// Write the GitHub Copilot CLI plugin directory.
      #[arg(long)]
      pub copilot: bool,
  }
  ```

  `Commands::Pack(PackArgs)` keeps `display_order = 13`. Its doc comment
  becomes: first line `Build bundles as client plugin directories.`, then the
  three existing lines (the mangen test asserts
  `Without a client flag, writes both directories.`).
- Produces, in `src/pack.rs`:

  ```rust
  /// A bundle that `pack` builds, with the description of its manifest.
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct SelectedBundle {
      pub name: String,
      pub description: String,
  }

  pub fn select_bundles(
      registry_root: &Path,
      names: &[String],
      all: bool,
  ) -> Result<Vec<SelectedBundle>>
  ```

  It calls `crate::parse::bundle::discover(registry_root)`. With `all`, it
  returns every discovered bundle; it fails with `no bundle in <registry_root>`
  when there is none, and it fails on the first name that `validate_bundle_name`
  refuses, with the context `bundle '<name>'`. Without `all`, it removes
  repeated names, and fails on the first name (in sorted order) that no
  discovered bundle has, with exactly
  `no bundle named '<name>' in <registry_root>` (the text of
  `plan_bundle_output`). The result is sorted by name. `name` and `description`
  come from the discovered `Bundle`.
- `run_pack(args: &PackArgs) -> i32` in `src/main.rs`: validate `--source` and
  each name in `args.bundles` (exit 2, as today); then `select_bundles` (exit
  1); then for each selected bundle, `build` for Claude Code when
  `args.claude || !args.copilot`, then for Copilot CLI when
  `args.copilot || !args.claude`; then one `write_plugin_dirs` call; then one
  `Wrote <dir> (<n> files)` line for each directory, as today.

- [ ] **Step 1: Write the failing tests in `tests/cli_pack.rs`**

  Add a `Fixture` method `fn pack(&self, args: &[&str]) -> assert_cmd::Command`
  that runs `pack <args...> --source driftsys/demo@v1 --out <f.out>` in
  `f.registry`. The fixture registry has the bundles `base` (`Base bundle`),
  `demo` (`Demo
  bundle`, requires `base`) and `norules`
  (`Bundle without rules`).

  - `pack_of_several_bundles_writes_every_directory_in_name_order`: run
    `f.pack(&["norules", "demo"])`; assert success; assert that the stdout
    lines, in order, start with `Wrote <out>/demo-claude (`,
    `Wrote <out>/demo-copilot (`, `Wrote <out>/norules-claude (`,
    `Wrote <out>/norules-copilot (`, and that there are 4 lines; assert that
    `demo-claude` and `demo-copilot` equal their golden trees in
    `tests/fixtures/pack/expected/`; assert that
    `norules-claude/.claude-plugin/plugin.json` and
    `norules-copilot/plugin.json` are files.
  - `pack_all_writes_every_bundle_of_the_registry`: run `f.pack(&["--all"])`;
    assert success; assert that the sorted names in `f.out` are exactly
    `base-claude`, `base-copilot`, `demo-claude`, `demo-copilot`,
    `norules-claude`, `norules-copilot`.
  - `pack_of_a_repeated_bundle_name_writes_it_once`: run
    `f.pack(&["demo", "demo", "--claude"])`; assert success and exactly one
    stdout line, which starts with `Wrote <out>/demo-claude (`.
  - `pack_of_several_bundles_writes_nothing_when_one_is_unknown`: run
    `f.pack(&["demo", "nope"])`; assert exit 1, stderr contains
    `no bundle named 'nope'`, and `f.out` does not exist.
  - `pack_of_several_bundles_checks_every_directory_before_it_writes_one`:
    create `f.out/norules-copilot/keep`; run `f.pack(&["demo", "norules"])`;
    assert exit 1, stderr contains the `norules-copilot` path and `--force`,
    `f.out/demo-claude` does not exist, and the tree of `f.out` is unchanged.
  - `pack_refuses_all_with_a_bundle_name_and_requires_one_of_them`: assert exit
    2 for `f.pack(&["--all", "demo"])` and for `f.pack(&[])`; `f.out` does not
    exist.
  - `pack_refuses_an_unsafe_name_among_several_bundles`: run
    `f.pack(&["demo", "a;b"])`; assert exit 2 and `f.out` does not exist.
  - `pack_all_of_a_directory_without_bundles_fails`: run the command with
    `--all` in an empty directory; assert exit 1 and stderr contains
    `no bundle in`.
  - `pack_all_refuses_a_registry_bundle_with_an_unsafe_name`: write
    `f.registry/bad name.bundle.yaml` with
    `schema: 1\nname: bad name\ndescription: d\nitems: {}\n`; run
    `f.pack(&["--all"])`; assert exit 1, stderr contains `bundle 'bad name'` and
    `bundle name must be nonempty`, and `f.out` does not exist.
  - `pack_all_below_the_registry_twice_writes_equal_bytes`: run the command in
    `f.registry` with the arguments
    `pack --all --source driftsys/demo@v1 --out plugins`; record
    `tree(f.registry/plugins)`; run it again with `--force`; assert success and
    an equal tree.

- [ ] **Step 2: Write the failing unit test in `src/pack.rs`**

  `select_bundles_sorts_and_removes_repeated_names`: in a temporary directory,
  write `b.bundle.yaml` and `a.bundle.yaml` (`schema: 1`, `items: {}`,
  descriptions `B` and `A`); assert that
  `select_bundles(dir, &["b".into(), "a".into(), "b".into()], false)` returns
  `[SelectedBundle { name: "a", description: "A" }, SelectedBundle { name:
  "b", description: "B" }]`,
  and that `select_bundles(dir, &[], true)` returns the same list.

- [ ] **Step 3: Run `cargo test --test cli_pack` and `cargo test --lib pack` and
      confirm that the new tests fail**

- [ ] **Step 4: Implement**

  - `src/cli.rs`: add `PackArgs` and change the variant to `Pack(PackArgs)` as
    specified under Interfaces.
  - `src/pack.rs`: add `SelectedBundle` and `select_bundles`.
  - `src/main.rs`: dispatch `Commands::Pack(args) => run_pack(&args)` and change
    `run_pack` as specified under Interfaces. The existing tests of
    `tests/cli_pack.rs` keep passing without change.

- [ ] **Step 5: Run `cargo test` and confirm that all tests pass**

- [ ] **Step 6: Update the documentation**

  `docs/book/commands.md` (section `upskill pack`) and section 2.8 of
  `docs/specification/specification.md` hold the same text; change both in the
  same way:

  - Command guide heading:
    `` ### `upskill pack <bundle>... --source <source> --out <dir>` `` (plan
    decision 19).
  - First sentence: `pack` builds plugin directories from one or more named
    bundles of the current source registry, or with `--all` from every bundle of
    it. State that the names are built in name order and that a repeated name is
    built once; that one `--source` is written into the `init` command of every
    bundle; that every bundle is built and every selected directory is checked
    before any directory is written; that an unknown name fails with exit 1
    before any write; that `--all` without bundles fails with exit 1; that
    `--all` together with a bundle name is a usage error (exit 2).
  - Add the example `upskill pack --all --source owner/repo@v1 --out dist` after
    the existing example.
  - The sentence about the bundle name: a name from `--all` that fails the rule
    stops the run with exit code 1.
  - Change any other sentence of these two sections that this task makes false.
    Do not describe `--catalog`; Task 2 does.
  - `docs/book/recipes.md`: after the existing example, add that `--all` builds
    every bundle of the registry and that several names can be given.
  - `skills/upskill-cli/SKILL.md`: in "Bundle plugin output and project rules",
    write the command as
    `upskill pack <bundle>... --source <source>
    --out <dir>` and add one
    sentence on several names and `--all`.

- [ ] **Step 7: Hand over**

  The implementer does not commit; the orchestrator runs `just fmt`,
  `just verify`, `just sanity` and commits with
  `feat(pack): build several bundles or every bundle with --all (#635)`.

---

### Task 2: `--catalog` and the two marketplace files

**Files:**

- Create: `src/catalog.rs`
- Modify: `src/lib.rs` (`pub mod catalog;`)
- Modify: `src/pipeline/discovery.rs` (`normalize_lexically` becomes
  `pub(crate)`), `src/pipeline/mod.rs`
  (`pub(crate) use discovery::normalize_lexically;`)
- Modify: `src/cli.rs` (`PackArgs`), `src/main.rs` (`run_pack`)
- Modify: `tests/cli_pack.rs`, `tests/cli_mangen.rs`
- Create: `tests/fixtures/pack/catalog/new-claude.json`,
  `tests/fixtures/pack/catalog/new-copilot.json`,
  `tests/fixtures/pack/catalog/existing-claude.json`,
  `tests/fixtures/pack/catalog/merged-claude.json`,
  `tests/fixtures/pack/catalog/merged-all-claude.json`
- Modify: `docs/book/commands.md`, `docs/specification/specification.md`
  (section 2.8), `docs/book/recipes.md`, `skills/upskill-cli/SKILL.md`,
  `docs/roadmap.md` (stage 2), `AGENTS.md` (module layout), `docs/wip/README.md`
  (paragraph of the plugin MVP design)
- Modify: `docs/adr/0046-plugin-mvp-components.md` (Decision 14),
  `docs/wip/specs/2026-11-24-plugin-mvp-design.md` (section 7): the CI command
  only

**Interfaces:**

- Consumes from Task 1: `PackArgs`, `pack::select_bundles`,
  `pack::SelectedBundle { name, description }`, `run_pack(args: &PackArgs)`.
- Produces, in `src/cli.rs`, three fields of `PackArgs`:

  ```rust
  /// Write the marketplace files below this root directory. `--out` must be
  /// inside it.
  #[arg(long, value_name = "ROOT")]
  pub catalog: Option<PathBuf>,
  /// The `name` of a marketplace file that does not exist yet.
  #[arg(long, value_name = "NAME", requires = "catalog",
        value_parser = clap::builder::NonEmptyStringValueParser::new())]
  pub catalog_name: Option<String>,
  /// The `owner.name` of a marketplace file that does not exist yet.
  #[arg(long, value_name = "NAME", requires = "catalog",
        value_parser = clap::builder::NonEmptyStringValueParser::new())]
  pub catalog_owner: Option<String>,
  ```

- Produces, in `src/catalog.rs`:

  ```rust
  /// The path of the marketplace file of `client` below the catalog root:
  /// `.claude-plugin/marketplace.json` or `.github/plugin/marketplace.json`.
  pub fn marketplace_path(client: PackClient) -> &'static str;

  /// A usage error of `--catalog`; `main.rs` maps it to exit code 2.
  #[derive(Debug, thiserror::Error)]
  pub enum CatalogUsageError {
      #[error("--out {out} is not inside --catalog {root}; the paths are compared as written, without following symbolic links")]
      OutsideRoot { out: String, root: String },
      #[error("{path} does not exist: pass --catalog-name and --catalog-owner to create it")]
      NeedsHeader { path: String },
  }

  /// The `name` and `owner.name` of a new marketplace file.
  pub struct Header<'a> {
      pub name: Option<&'a str>,
      pub owner: Option<&'a str>,
  }

  /// One plugin directory that this run packs.
  pub struct PackedPlugin {
      pub client: PackClient,
      pub name: String,
      pub description: String,
      pub dir_name: String,
  }

  /// The new bytes of one marketplace file and the names of the entries that
  /// `--all` removed, in name order.
  pub struct CatalogUpdate {
      pub path: PathBuf,
      pub bytes: Vec<u8>,
      pub removed: Vec<String>,
  }

  /// The checked paths and the parsed existing files of one run.
  pub struct Catalog { /* private fields */ }

  impl Catalog {
      pub fn open(
          cwd: &Path,
          root: &Path,
          out: &Path,
          clients: &[PackClient],
          header: Header<'_>,
      ) -> Result<Catalog>;
      pub fn render(&self, plugins: &[PackedPlugin], prune: bool) -> Result<Vec<CatalogUpdate>>;
  }

  pub fn write_updates(updates: &[CatalogUpdate]) -> Result<()>;
  ```

  - `open` joins `root` and `out` to `cwd`, normalizes both with
    `crate::pipeline::normalize_lexically`, and returns
    `CatalogUsageError::OutsideRoot` (inside an `anyhow::Error`) when the
    normalized `out` does not start with the normalized `root` (plan decisions 6
    and 7). In the message, `out` and `root` are the paths as given. For each
    client in `clients` (Claude Code first), it reads
    `root.join(marketplace_path(client))`. A missing file without both
    `header.name` and `header.owner` returns `CatalogUsageError::NeedsHeader`,
    with `path` = the root as given joined with the relative path. An existing
    file is parsed into a `serde_json::Map`; these errors exit 1, with the path
    as given:
    - not JSON: context `parsing <path>`;
    - `<path>: the top level is not a JSON object`;
    - `<path>: plugins is not an array`;
    - `<path>: plugins[<i>] has no string name` (`<i>` counts from 0).
  - `render` computes, for each opened file, the entries of `plugins` whose
    `client` matches, with `source` = `./` followed by the components of `out`
    relative to `root` and `dir_name`, joined with `/` (plan decision 8). It
    applies plan decisions 10 to 14, sorts all entries by `name` with a stable
    sort, and serializes as in plan decision 12. With `prune` (true for
    `--all`), it removes the entries of plan decision 14 and lists their names.
  - `write_updates` creates the parent directory of each path, writes the bytes
    to `<path>.tmp` (`path.with_extension("json.tmp")`) and renames it to the
    path; on a write or rename failure it removes the temporary file (plan
    decision 18).
- `run_pack` in `src/main.rs`, in this order: validate `--source` and names
  (exit 2); when `args.catalog` is set, `Catalog::open` with the selected
  clients (exit 2 when the error downcasts to `CatalogUsageError`, else exit 1);
  `select_bundles` (exit 1); `build` of each bundle and client (exit 1);
  `Catalog::render(&plugins, args.all)` (exit 1); `write_plugin_dirs` (exit 1);
  the `Wrote <dir> (<n> files)` lines; `write_updates` (exit 1); then the
  `Removed` and `Wrote <path>` lines of plan decision 17.

**Golden fixtures** (bytes are exact; each file ends with one newline).

`new-claude.json`, written by
`pack demo norules --out <market>/plugins --catalog <market> --catalog-name
demo-market --catalog-owner driftsys`:

```json
{
  "name": "demo-market",
  "owner": {
    "name": "driftsys"
  },
  "plugins": [
    {
      "name": "demo",
      "source": "./plugins/demo-claude",
      "description": "Demo bundle"
    },
    {
      "name": "norules",
      "source": "./plugins/norules-claude",
      "description": "Bundle without rules"
    }
  ]
}
```

`new-copilot.json` is the same with `demo-copilot` and `norules-copilot`.

`existing-claude.json`, the input of the merge tests:

```json
{
  "name": "market",
  "owner": {
    "name": "driftsys",
    "email": "maintainer@example.com"
  },
  "metadata": {
    "description": "Hand-written"
  },
  "plugins": [
    {
      "name": "upskill",
      "source": "./plugins/upskill-claude",
      "description": "Notice plugin",
      "version": "1.0.0"
    },
    {
      "name": "demo",
      "source": "./old/demo",
      "description": "Old demo",
      "version": "0.1.0"
    },
    {
      "name": "remote",
      "source": { "source": "github", "repo": "example/remote" },
      "description": "Remote plugin"
    },
    {
      "name": "tool",
      "source": "./vendor/tool",
      "description": "Vendored tool"
    }
  ]
}
```

`merged-claude.json`, written by `pack demo --claude` with the catalog flags and
no header flags:

```json
{
  "metadata": {
    "description": "Hand-written"
  },
  "name": "market",
  "owner": {
    "email": "maintainer@example.com",
    "name": "driftsys"
  },
  "plugins": [
    {
      "name": "demo",
      "source": "./plugins/demo-claude",
      "description": "Demo bundle"
    },
    {
      "name": "remote",
      "source": {
        "repo": "example/remote",
        "source": "github"
      },
      "description": "Remote plugin"
    },
    {
      "name": "tool",
      "source": "./vendor/tool",
      "description": "Vendored tool"
    },
    {
      "name": "upskill",
      "source": "./plugins/upskill-claude",
      "description": "Notice plugin",
      "version": "1.0.0"
    }
  ]
}
```

`merged-all-claude.json`, written by `pack --all --claude` with the catalog
flags and no header flags:

```json
{
  "metadata": {
    "description": "Hand-written"
  },
  "name": "market",
  "owner": {
    "email": "maintainer@example.com",
    "name": "driftsys"
  },
  "plugins": [
    {
      "name": "base",
      "source": "./plugins/base-claude",
      "description": "Base bundle"
    },
    {
      "name": "demo",
      "source": "./plugins/demo-claude",
      "description": "Demo bundle"
    },
    {
      "name": "norules",
      "source": "./plugins/norules-claude",
      "description": "Bundle without rules"
    },
    {
      "name": "remote",
      "source": {
        "repo": "example/remote",
        "source": "github"
      },
      "description": "Remote plugin"
    },
    {
      "name": "tool",
      "source": "./vendor/tool",
      "description": "Vendored tool"
    }
  ]
}
```

The `upskill` entry of the input is inside `--out` and names no packed
directory, so `--all` removes it; this tests the rule, not the content of any
real marketplace.

- [ ] **Step 1: Write the failing unit tests in `src/catalog.rs`**

  - `marketplace_paths_of_each_client`: `.claude-plugin/marketplace.json` and
    `.github/plugin/marketplace.json`.
  - `open_refuses_an_out_that_is_not_inside_the_root`: with `cwd = /w` and
    `root = /w/market`, assert that `open` returns an error that downcasts to
    `CatalogUsageError::OutsideRoot` for `out` = `/w/market2/plugins`,
    `/w/other`, `/w/market/../other`; and that it accepts `/w/market`,
    `/w/market/plugins` and `market/./plugins/../plugins` (relative to `cwd`).
    Use a temporary directory in place of `/w`, with header flags so that a
    missing file is no error.
  - `source_is_inside_out_follows_plan_decision_14`: table test of the private
    `fn source_is_inside(source: &str, out_rel: &[String]) -> bool`:
    `("./plugins/a", ["plugins"])` true; `("plugins/a", ["plugins"])` true;
    `("./plugins/a/", ["plugins"])` true; `("./plugins", ["plugins"])` false;
    `("./plugins/", ["plugins"])` false; `("./other/a", ["plugins"])` false;
    `("./plugins/../x", ["plugins"])` false; `("/plugins/a", ["plugins"])`
    false; `("./a", [])` true; `("./pluginsx/a", ["plugins"])` false.
  - `open_refuses_an_invalid_existing_file`: one case for each of the four error
    texts of `open`; for each, the error message contains the path.
  - `open_reads_a_file_without_plugins_as_an_empty_list`: an existing file
    `{"name": "m", "owner": {"name": "o"}}` and one packed plugin render
    `{"name": "m", "owner": {"name": "o"}, "plugins": [<the entry>]}`.

- [ ] **Step 2: Write the failing integration tests in `tests/cli_pack.rs`**

  Add a `Fixture` field `market: PathBuf` (`<tmp>/market`, not created) and a
  method `fn catalog(&self, args: &[&str]) -> assert_cmd::Command` that runs
  `pack <args...> --source driftsys/demo@v1 --out <market>/plugins
  --catalog <market>`
  in `f.registry`. `NEW` below stands for
  `--catalog-name demo-market --catalog-owner driftsys`.

  - `pack_catalog_writes_both_new_marketplace_files`: run
    `f.catalog(&["demo", "norules", NEW...])`; assert success; assert that
    `<market>/.claude-plugin/marketplace.json` equals the bytes of
    `new-claude.json` and `<market>/.github/plugin/marketplace.json` the bytes
    of `new-copilot.json`; assert that no entry holds `version`; assert that the
    last two stdout lines are `Wrote <market>/.claude-plugin/marketplace.json`
    and `Wrote <market>/.github/plugin/marketplace.json`.
  - `pack_catalog_with_a_client_flag_writes_only_that_file`: run
    `f.catalog(&["demo", "--claude", NEW...])`; assert that the Claude Code file
    exists and `<market>/.github` does not exist. Then, in a new fixture, run
    `f.catalog(&["demo", "--copilot", NEW...])` and assert the opposite.
  - `pack_catalog_of_a_new_file_needs_name_and_owner`: for the argument lists
    `[]`, `["--catalog-name", "m"]` and `["--catalog-owner", "o"]` (each after
    `demo`), assert exit 2, stderr contains `does not exist` and
    `--catalog-name`, and `<market>` does not exist.
  - `pack_catalog_name_and_owner_require_catalog`: run
    `f.pack(&["demo", "--catalog-name", "m", "--catalog-owner", "o"])` (no
    `--catalog`); assert exit 2.
  - `pack_catalog_refuses_an_out_outside_the_root`: run the command with
    `--out <tmp>/out --catalog <market>` and the header flags; assert exit 2,
    stderr contains `is not inside --catalog`, and neither `<tmp>/out` nor
    `<market>` exists.
  - `pack_catalog_keeps_top_level_fields_and_foreign_entries`: write
    `existing-claude.json` to `<market>/.claude-plugin/marketplace.json`; run
    `f.catalog(&["demo", "--claude"])`; assert success and that the file equals
    the bytes of `merged-claude.json`; assert that no `Removed` line is printed.
  - `pack_catalog_all_removes_stale_entries_inside_out`: write
    `existing-claude.json` as above; run `f.catalog(&["--all", "--claude"])`;
    assert success, that the file equals `merged-all-claude.json`, that stdout
    contains `Removed upskill from <market>/.claude-plugin/marketplace.json`,
    and that `<market>/plugins` holds exactly `base-claude`, `demo-claude` and
    `norules-claude`.
  - `pack_catalog_deletes_no_plugin_directory`: create
    `<market>/plugins/stale-claude/keep`; write a Claude Code file whose only
    entry is
    `{"name": "stale", "source": "./plugins/stale-claude",
    "description": "d"}`;
    run `f.catalog(&["--all", "--claude"])`; assert that
    `<market>/plugins/stale-claude/keep` still exists and that the file has no
    entry named `stale`.
  - `pack_catalog_uses_the_header_flags_only_for_a_new_file`: write
    `existing-claude.json`; run
    `f.catalog(&["demo", "--catalog-name", "other", "--catalog-owner", "org"])`;
    assert that the Claude Code file has `name` `market` and `owner`
    `{"email": "maintainer@example.com", "name": "driftsys"}`, and that the new
    Copilot CLI file has `name` `other` and `owner` `{"name": "org"}`.
  - `pack_catalog_refuses_an_invalid_marketplace_file_before_any_write`: write
    `not json` to `<market>/.claude-plugin/marketplace.json`; run
    `f.catalog(&["demo", NEW...])`; assert exit 1, stderr contains the file
    path, `<market>/plugins` does not exist, the file still holds `not json`,
    and `<market>/.github` does not exist.
  - `pack_catalog_below_the_registry_twice_writes_equal_bytes`: in `f.registry`,
    run
    `pack --all --source driftsys/demo@v1 --out plugins
    --catalog . NEW...`;
    assert that `.claude-plugin/marketplace.json` has the entry source
    `./plugins/demo-claude`; record `tree(f.registry)`; run
    `pack --all --source driftsys/demo@v1 --out plugins --catalog . --force` (no
    header flags); assert success and an equal tree.
  - `pack_catalog_of_an_unknown_bundle_writes_no_file`: run
    `f.catalog(&["demo", "nope", NEW...])`; assert exit 1 and `<market>` does
    not exist.

  In `tests/cli_mangen.rs`, `pack_man_page_documents_the_catalog_flags`: the
  page of `pack` contains `\-\-catalog\-name` and
  `Write the marketplace files below this root directory`.

- [ ] **Step 3: Run `cargo test --test cli_pack`,
      `cargo test --test
      cli_mangen` and `cargo test --lib catalog` and
      confirm that the new tests fail**

- [ ] **Step 4: Write the five golden fixtures** with the exact bytes above.

- [ ] **Step 5: Implement**

  - `src/pipeline/discovery.rs` and `src/pipeline/mod.rs`: plan decision 20.
    Change nothing else in those files.
  - `src/catalog.rs` and `src/lib.rs`: as specified under Interfaces. For plan
    decision 12, serialize a private struct with
    `#[serde(flatten)] fields: serde_json::Map<String, Value>` (without the key
    `plugins`) followed by `plugins: Vec<Entry>`. `Entry` is one struct for
    packed and kept entries:

    ```rust
    #[derive(Serialize)]
    struct Entry {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        source: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<Value>,
        #[serde(flatten)]
        rest: serde_json::Map<String, Value>,
    }
    ```

    A kept entry moves `name`, `source` and `description` out of its map into
    the fields; the remaining keys go to `rest`. A packed entry has an empty
    `rest`. A new file starts from the fields `name` and `owner: {"name": ...}`.
  - `src/cli.rs`: the three fields of `PackArgs`.
  - `src/main.rs`: `run_pack` as specified under Interfaces. Collect one
    `PackedPlugin` for each built directory, with the `name` and `description`
    of its `SelectedBundle` and the `dir_name` of the returned `PluginDir`.

- [ ] **Step 6: Run `cargo test` and confirm that all tests pass**

- [ ] **Step 7: Update the documentation**

  In `docs/book/commands.md` (section `upskill pack`) and section 2.8 of
  `docs/specification/specification.md`, in the same way:

  - Add a paragraph on `--catalog <root>`: the two files and their paths; one
    file for each selected client; `--out` inside `<root>`, compared without
    following symbolic links, else exit 2; the entry fields and the `./` prefix,
    `/` as separator, no `version`, sorting by name; the merge rules (top-level
    fields and entries of other names kept; an entry of a packed bundle replaced
    as a whole; with `--all`, removal of an entry whose `source` is inside
    `--out` and names no plugin directory of that client packed in the run, with
    a `Removed` line); no plugin directory deleted; `--catalog-name` and
    `--catalog-owner` needed only for a new file, else exit 2, and not applied
    to an existing file; an invalid existing file fails with exit 1 before any
    write; the files are written after every plugin directory is in place, and a
    failure there leaves the directories in place; key order of plan
    decision 12.
  - State that when `--out` equals `--catalog`, `--all` removes every entry with
    a relative-path `source` that is not packed in the run (plan decision 7).
  - Add the example:

    ```bash
    upskill pack --all --source owner/repo@v1 --out market/plugins \
      --catalog market --catalog-name my-market --catalog-owner my-org
    ```
  - Replace "`pack` reads no lockfile and writes only below `--out`" with a
    sentence that adds the two marketplace files below `--catalog`, and remove
    "write a marketplace manifest" from what `pack` does not do.
  - Change any other sentence of these two sections that this task makes false.
  - `docs/book/recipes.md`, "Publish a bundle as client plugins": replace the
    instruction to copy the directories into a marketplace repository by a
    `pack --all ... --catalog` run whose `--out` is inside the marketplace
    repository; state that a hand-written plugin that must stay in the catalog
    has its directory outside `--out`, because `--all` removes an entry inside
    `--out` that names no packed directory. Give the concrete layout: the packed
    output in its own directory and the hand-written plugins in another
    directory, run from the root of the marketplace repository:

    ```bash
    upskill pack --all --force --source owner/repo@v1 --out packed --catalog .
    ```

    The packed plugins are below `packed/`; hand-written plugins stay below
    `plugins/` and keep their entries.
  - `skills/upskill-cli/SKILL.md`, "Bundle plugin output and project rules": one
    sentence on `--catalog` and the two marketplace files; replace "does not
    install or publish the output" only if it becomes false (it stays true).
  - `docs/roadmap.md`, stage 2: in "Catalog generation", add that #635
    implements it with `pack --catalog`; in the first paragraph, add that
    several bundles, `--all` and the marketplace files are implemented in #635.
  - `AGENTS.md`, module layout (a `text` block): add after the `pack.rs` line
    the line `catalog.rs           Marketplace files of pack --catalog`, with
    the description in the column of the other lines.
  - `docs/wip/README.md`: in the paragraph of the plugin MVP design, which
    records the state of each story, add that story
    [#635](https://github.com/driftsys/upskill/issues/635) (several bundles,
    `--all` and the marketplace files in `pack`) is implemented.
  - `docs/adr/0046-plugin-mvp-components.md`, Decision 14, and
    `docs/wip/specs/2026-11-24-plugin-mvp-design.md`, section 7: change the CI
    command to
    `upskill pack --all --force --source <source> --out <dir> --catalog .` and
    add one sentence: `<dir>` holds only packed plugin directories. This
    corrects the command, which cannot run without `--source` and `--out`; it
    does not change the decision. Change nothing else in these two files.

- [ ] **Step 8: Hand over**

  The implementer does not commit; the orchestrator runs `just fmt`,
  `just verify`, `just sanity` and commits with
  `feat(pack): write the marketplace files with --catalog (#635)`.
