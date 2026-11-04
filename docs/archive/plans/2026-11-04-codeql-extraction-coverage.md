# Rust CodeQL extraction coverage plan

> **For agentic workers:** Use superpowers:executing-plans to execute this task.

**Goal:** Resolve issue #344 using exact extraction and SARIF evidence.

**Architecture:** Diagnose the default-setup Rust extractor before changing
code. Keep GitHub default setup, query selection, dependencies and runtime
behavior unchanged. Test whether a conditional block prevents the extractor from
trying to expand macros on inactive directly attributed statements. If the
workaround does not remove the diagnostics, retain the upstream limitation
honestly.

**Tech stack:** Rust 2024, CodeQL 2.27.3, GitHub CLI, Python 3 evidence checks.

**Spec:** [Issue #344](https://github.com/driftsys/upskill/issues/344). The user
approved starting this independent lane on 2026-11-04.

## Global constraints

- Work only in the codeql-extraction-coverage worktree; preserve other lanes.
- No default-setup, ruleset or security-settings changes.
- Do not suppress warnings or exclude source files to improve the metric.
- Preserve operating-system guards, error messages and assertion behavior.
- Keep successful SARIF ingestion separate from extraction quality.

## Review focus

- Inactive code must remain inactive on Linux, macOS and Windows.
- No unsupported-platform error may turn into successful completion.
- Test assertions must retain their original operands and messages.
- CodeQL must scan the same complete tracked Rust file inventory.
- Bind reported metrics and ingestion to the exact analyzed commit.

## Task 1: Reproduce and isolate the extraction warnings

- [x] Capture the current default-setup configuration, Rust job log and SARIF.
- [x] Confirm seven warnings in five files and metrics 5 errored / 188 clean at
      main 1a291a7de62481b14360ddcc09cf19cc93ccbdc2.
- [x] Compare every warning against its surrounding conditional compilation.
- [ ] Compare active, directly attributed inactive and block-wrapped macros with
      the same extractor. A hosted exact-head run may provide the decisive
      before/after check if local tool acquisition is impractical.
- [x] Complete the clean baseline Rust suite.

## Task 2: Remove false extraction errors and retain verification

**Files:** src/filesystem/{destinations,scope,sources}.rs,
tests/filesystem_destinations.rs, tests/source_manifests.rs,
docs/technotes/codeql-rust-extraction.md and its linked evidence directory.

- [x] If the hypothesis holds, wrap only the affected conditional macro
      statements in blocks without changing conditions, messages or operands.
- [x] Record the root cause, exact tool/query versions, seven warning sites,
      before/after metrics, coverage limits and independent controls.
- [x] Run just fmt, just verify and appropriate MSRV checks.
- [x] Obtain independent review, address findings, and create a task PR.
- [x] Inspect the PR CodeQL job and its exact-head SARIF ingestion; require zero
      errored files without reducing the source inventory before claiming the
      coverage metric fixed.
- [ ] Reconcile this plan into durable evidence before final delivery.
