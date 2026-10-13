# File recovery checkpoint model

Status: selected pure model; implementation pending.

The next recovery prerequisite separates ordered checkpoint decisions from journal persistence and file effects. It consumes supplied File/State actions, Queued/Pending/Done checkpoints and Before/After/Both/Neither observation relations. Exactly one State action is final. Completed actions form a prefix; at most one pending action follows it.

The model checks the complete supplied sequence before returning one next step. A pending post-state is acknowledged instead of repeated, an unstarted coincidental post-state requires reconciliation, and an unchanged queued action needs only a checkpoint. Recorded completion and currently verified post-state prefixes remain distinct.

This API has no I/O, serialization or execution authority. Future qualified observers and journal producers must establish actual identities, values, source/state bindings and permissions. Credential-free reconstruction and the durable receipt format remain unresolved here. The implemented [permission-selection policy](file-permission-policy.md) is a separate prerequisite, and physical permission application remains pending.
