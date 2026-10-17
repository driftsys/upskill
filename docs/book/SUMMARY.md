<!-- markdownlint-disable MD025 -->

# Summary

[Introduction](intro.md)

# User guide

- [Getting started](getting-started.md)
- [Commands](commands.md)
- [Recipes](recipes.md)
- [Conventions](conventions.md)

# Reference

- [Tool specification](specification.md)
- [Portable format](format-spec.md)

# Contracts and architecture

- [Generation and compilation](topics/0002-generation.md)
- [Sources and captured evidence](topics/0003-sources.md)
- [State and ownership](topics/0004-state-and-ownership.md)
- [Lifecycle planning](topics/0005-lifecycle-planning.md)
- [Artifact planning](topics/0006-artifact-planning.md)
- [Filesystem access](topics/0007-filesystem-access.md)
- [Execution and recovery](topics/0008-execution-and-recovery.md)

# Compatibility

- [Previous reference entrypoints](previous-reference.md)
  - [Scope locking](design/qualified-scope-locking.md)
  - [Scope locking decision](adr/0028-lock-qualified-scope-directories.md)
  - [Destination observation](design/qualified-destination-observation.md)
  - [Destination observation decision](adr/0029-capture-qualified-destination-evidence.md)
  - [Source observation](design/qualified-source-observation.md)
  - [Source observation decision](adr/0030-capture-qualified-source-evidence.md)
  - [File permissions](design/file-permission-policy.md)
  - [File permission decision](adr/0031-select-explicit-file-permissions.md)
  - [Recovery checkpoints](design/file-recovery-model.md)
  - [Recovery checkpoint decision](adr/0032-model-recovery-checkpoints-before-effects.md)
  - [File execution](design/qualified-file-execution.md)
  - [File execution decision](adr/0033-qualified-file-execution-and-journals.md)
  - [Backup journal decision](adr/0037-require-create-only-backup-journals.md)
