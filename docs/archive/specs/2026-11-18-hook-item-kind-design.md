# Hook item kind: `HOOK.md` source convention

Date: 2026-11-18. Status: **replaced on 2026-11-24** by
[AD-0046](../../adr/0046-plugin-mvp-components.md) and the
[plugin MVP design](../../wip/specs/2026-11-24-plugin-mvp-design.md), which keep
the convention of sections 1 to 4 and the ten event names. They decide which
events accept context text, the form of `matcher` and the open points. This
document was a draft; it does not describe released behaviour.

Related records:
[bundle plugin output design](2026-11-18-bundle-plugin-output-design.md),
[format specification](../../topics/0001-content-format.md),
[AD-0040](../../adr/0040-separate-customization-delivery-from-discovery.md),
[AD-0041](../../adr/0041-separate-client-representations-as-design-baseline.md),
[AD-0042](../../adr/0042-resteer-customization-delivery.md).

## Why

A plugin cannot hold an always-loaded instruction file in Claude Code or Copilot
CLI. A session-start hook can add text to the model's context in both clients
(prototype of 2026-11-18, recorded in the bundle plugin output design). A bundle
author has no source format for such a hook today.

## Scope

- This document fixes the source convention of one new item kind, `hook`.
- AD-0040 and AD-0041 keep general hook authoring for a later scope. The slice
  that implements this convention needs its own decision record.
- The first delivery target is the plugin output of `upskill pack`, where the
  plugin owns its hook file. Hooks in a project install write into settings
  files that other tools share, which the ownership record of AD-0042 does not
  cover. That target needs a separate design.

## Convention

### 1. One file for each hook

- The entry file is `HOOK.md`. The file name selects the kind, as for `RULE.md`,
  `SKILL.md` and `AGENT.md` (format specification §2.1).
- One item directory holds at most one `HOOK.md`. The directory may hold other
  entry files under the existing co-location rule.
- A bundle lists hooks under `items.hooks`.

### 2. Frontmatter holds the events and their settings

```yaml
---
name: markspec-write-loop
description: MarkSpec write loop rules and reminder
events:
  session-start:
  post-tool: { matcher: edit, run: check.sh }
---
```

- `events` is a map. Each key is a portable event name. The map is the only list
  of the events of the hook.
- A value is empty or a map of settings: `matcher`, `run`, `timeout`.
- An event without `run` is a **context hook**: the generated hook adds the text
  of the event to the model's context.
- An event with `run` is a **command hook**: the generated hook runs the named
  script from the item directory. The text of the event is documentation.

### 3. The body holds the text of each event

```markdown
## session-start

Before editing a MarkSpec file, load the `markspec-write-loop` skill.

### Details

More text.

## post-tool

Runs `markspec check` on the edited file.
```

- A level-2 heading whose text equals a key of `events` starts the section of
  that event. The section ends at the next such heading or at the end of the
  file.
- A body without such a heading is one text for every event of the hook.
- Text before the first event heading belongs to every event.
- Headings of level 3 and below inside a section are content.
- `@client` directive blocks keep their existing meaning. A hook has no
  per-client override file: #580 removed override files from the content format.

### 4. Lint rules

`upskill lint` reports an error for each of these:

1. A key of `events` that is not a known portable event name.
2. A context-hook event with no text.
3. A level-2 heading in a body with event sections whose text is not a key of
   `events`. The message tells the author to use level 3 for content.
4. A `run` value that names a file that does not exist in the item directory.

### 5. Portable event names

The initial set holds the events that the documentation of Claude Code, Copilot
CLI and Codex all list, as read on 2026-11-18: `session-start`, `session-end`,
`prompt-submit`, `pre-tool`, `post-tool`, `permission-request`, `stop`,
`subagent-start`, `subagent-stop`, `pre-compact`. Generation translates each
name to the name of the client.

## Evidence

- Prototype, 2026-11-18, Claude Code 2.1.308 and Copilot CLI 1.0.112: text from
  a session-start hook reached the model in both clients.
- `prim` 0.9.1: `prim fmt --check` requires no change for a `HOOK.md` with
  level-2 event headings, and `prim lint` reports nothing.
- `markdownlint` with the repository settings reports MD041 (first heading is
  not level 1) for level-2 event headings. Every existing item body that starts
  at level 2 gets the same finding, and `.markdownlintignore` lists `skills/`.

## Not verified

- Which events other than `session-start` accept text for the model's context in
  each client.
- The input and output formats of command hooks in each client, and a portable
  form of `matcher`.
- Whether `session-start` runs again after compaction in each client.

## Alternatives considered

- **Event in the file name** (`ON_<EVENT>.md`, `HOOK_<EVENT>.md`). Rejected: the
  entry file name selects the kind, the event would be stated twice, and a hook
  for two events has no file name.
- **`<!-- @event:... -->` blocks with a closing tag.** Rejected: half of the
  lines are markers, and a preview does not show which text belongs to which
  event.
- **`@event` section markers without a closing tag.** Rejected for the same
  preview reason, and because the marker would behave differently from
  `@client`.
- **Several frontmatter blocks in one file.** Rejected: `markdownlint` reads the
  later blocks as headings and reports three findings, and no item format has
  more than one frontmatter block.
- **Level-1 event headings.** Rejected: `markdownlint` reports MD025 for the
  second heading, and every existing item body starts at level 2.
- **One text for each hook, and one hook for each text.** Kept as the simple
  case: a body without event headings. Rejected as the only form, because the
  user wants several events of one behaviour in one file.
- **Hooks defined inside the bundle file, or in a YAML file.** Rejected: a hook
  would not be an item that several bundles can list, and long text inside YAML
  strings is hard to review.

## Open before implementation

1. Write two or three real hooks in this form, including the update notice, and
   record what is hard to express.
2. Run the trials listed under "Not verified".
3. Write the decision record that brings hook authoring into scope.
