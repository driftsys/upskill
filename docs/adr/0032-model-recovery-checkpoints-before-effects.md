# Model recovery checkpoints before file effects

AD-0032

Status: selected; pure implementation pending.

## Context

Owned lifecycle recovery must distinguish recorded completion from current observations, recognize an interrupted final state write and preserve manual edits. Neither supplied values nor the existing pure permission policy grant file-effect authority.

## Decision

Add a separate pure ordered checkpoint model before durable journals or file effects. Require one final State action and a completed prefix, followed by at most one Pending action and queued work. Validate all supplied observation relations before returning a next step. A pending post-state is acknowledged, queued post-state requires reconciliation, and an unchanged queued action performs no physical write. Report recorded and verified post-state prefixes separately.

## Alternatives and cost

Embedding these rules into effectful code couples logical defects to file mutation. Inferring completion from desired values alone adopts unstarted changes and can repeat the final state write. The selected model adds an interface and requires future qualified producers and durable checkpoint protocols. It does not select journal serialization, credential storage, native metadata preservation or lifecycle activation.

Related: [recovery model](../design/file-recovery-model.md), [permission policy](0031-select-explicit-file-permissions.md).
