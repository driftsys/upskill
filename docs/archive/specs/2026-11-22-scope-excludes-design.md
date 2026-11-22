# Per-scope excludes — design

Issue: #570 (bullet 4). Replaces the per-source design of
[ADR-0045](../../adr/0045-record-add-excludes-in-lockfile.md) on PR #608.

## Intent

`upskill add <source> --exclude <name>` skips an item. `update` must keep
skipping it, also when the item arrives as a dependency or a bundle member from
another source. The first design stored excludes per exact source label. A code
review of PR #608 found that this key does not match how `--exclude` works,
because `--exclude` matches by name across the whole dependency closure. The
mismatch caused findings 3–6 of that review: removal and reinstall on every
update, cross-source dependencies coming back, excludes lost when the label
spelling changes, and an undo that fails.

## Goals

- An excluded name stays excluded on every later `add` and `update` in the same
  install scope, whatever source would pull it in.
- An exclude never removes an item that the user installed by name.
- A typo in `--exclude` is refused, not recorded.

## Non-goals

- Excluding a name for one source only ("S without `d`, but T may pull `d`").
  The new design cannot express this; the loss is accepted.
- Repairing wrong `required_by` links already present in existing lockfiles.
- The older error when a source's last item is deleted upstream.

## Approaches

- **(a) Keep the per-source records and patch them.** Rejected: the key mismatch
  stays, so new cross-source cases keep appearing.
- **(b) One set of excluded names per lockfile, applied in the resolver.**
  Chosen by the user after an independent review (Fable) agreed.
- **(c) Close #608 and record the limitation.** Rejected: leaves #570 open.

## Design

**Storage.** A top-level, sorted, deduplicated list in `.upskill-lock.json`:
`"excludes": ["d", "e"]`. Omitted when empty. Remove `LockedExclude`,
`prune_excludes` and the `doctor` `orphan_excludes` check.

**Where it applies.** In the resolver (`src/pipeline/install.rs`), when it
visits a dependency or expands a bundle member: an excluded name is not visited.
It gets no `required_by` link and its own dependencies are not pulled. A
whole-source `add S` counts as expansion, so the exclude applies there too. It
never applies to a name passed positionally to `add`.

**Recording.** `add` stores (recorded ∪ `--exclude`) − positional names. Every
`add` and every per-source `update` reads the same set.

**Undo.** `add <any source that has d> d` installs `d` and removes it from the
set.

**Typos.** `add --exclude x` exits 2 when `x` matches no item in the closure,
and records nothing.

**Visibility.** `add` and `update` print each item they skip because of an
exclude (stderr, suppressed by `--quiet`). `list` prints the recorded set.

## Findings of the #608 review

| Finding                                        | Result                                         |
| ---------------------------------------------- | ---------------------------------------------- |
| 1 `update` fails when excludes leave nothing   | Removed: excludes no longer filter named items |
| 2 wrong `required_by` deletes a direct install | Fixed for new installs                         |
| 3 removed and reinstalled each update          | Fixed                                          |
| 4 cross-source dependencies come back          | Fixed                                          |
| 5 label spelling drops excludes                | Fixed: no label key                            |
| 6 undo fails                                   | Fixed                                          |
| 7 Windows path test                            | One-line test fix                              |

## Risks

- A name applies to every source and kind; a later unrelated item `d` is
  skipped. The skip message and the `list` output make this visible.
- The project lockfile is committed, so an exclude applies to the whole team.
  This is the same as every other lockfile entry.

## Left to the builder

The exact skip-message wording, the `list` line format, and how the resolver
receives the set.
