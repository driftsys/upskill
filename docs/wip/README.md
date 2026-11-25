# Working documents

This directory holds the designs and plans of work that is planned or in
progress. It replaces `docs/superpowers/`. New task designs belong in `specs/`,
and their implementation plans in `plans/`.

## Active documents

- [Plugin MVP design](specs/2026-11-24-plugin-mvp-design.md). Approved on
  2026-11-24. Stories [#632](https://github.com/driftsys/upskill/issues/632)
  (command items and user-only init),
  [#633](https://github.com/driftsys/upskill/issues/633) (MCP servers in `pack`
  output), [#634](https://github.com/driftsys/upskill/issues/634) (LSP servers
  in `pack` output), [#631](https://github.com/driftsys/upskill/issues/631)
  (hook items), and [#635](https://github.com/driftsys/upskill/issues/635)
  (several bundles, `--all` and the marketplace files in `pack`) are
  implemented. `add` and `update` report skipped command items, hook items and
  LSP servers in one line. Plans for #631 to #635 are archived:
  [#631](../archive/README.md#hook-item-kind-in-lint-fmt-new-and-pack-output),
  [#632](../archive/README.md#command-item-kind-in-lint-fmt-new-and-pack-output),
  [#633](../archive/README.md#mcp-servers-in-pack-output),
  [#634](../archive/README.md#lsp-servers-in-pack-output) and
  [#635](../archive/README.md#several-bundles-and-catalog-generation-in-pack).
  The design stays here because two stories remain planned work: #636 and #637,
  in stage 2 of the
  [roadmap](../roadmap.md#stage-2--finish-plugin-distribution). Its decisions
  are in [AD-0046](../adr/0046-plugin-mvp-components.md). It replaced the hook
  item kind draft, which is in the
  [archive](../archive/README.md#hook-item-kind-draft).

The triage of 2026-11-23
([#536](https://github.com/driftsys/upskill/issues/536)) moved the 36 other
documents to the [archive](../archive/README.md#triage-of-2026-11-23), each with
its delivery evidence or the record that replaced it.

## Promotion and archival

After validating a completed feature, update its specifications, accepted ADRs,
as-built design and user guide as applicable. Record remaining work explicitly,
then move only the completed feature's raw working documents into
[archive](../archive/README.md), with links to the delivery evidence and durable
successors. Unrelated or uncertain work stays here, with the reason stated in
this file.
