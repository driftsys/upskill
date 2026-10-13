//! Pure recovery decisions over supplied checkpoint and observation classifications.
//!
//! These values are not qualified filesystem evidence. Decisions carry no authority
//! to write files, persist checkpoints or activate a lifecycle operation.

/// The logical role of an action in a supplied complete operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    File,
    State,
}

/// Supplied recorded progress for one logical action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Checkpoint {
    Queued,
    Pending,
    Done,
}

/// Supplied comparison with the action's exact expected before and after values.
/// `Both` means the values are unchanged; these labels do not qualify evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedRelation {
    Before,
    After,
    Both,
    Neither,
}

/// One supplied action; the final action must have the `State` role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryAction {
    pub kind: ActionKind,
    pub checkpoint: Checkpoint,
    pub observed: ObservedRelation,
}

/// One proposed logical step, without physical execution or persistence authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStep {
    Apply { index: usize },
    Acknowledge { index: usize },
    CheckpointOnly { index: usize },
}

/// Classification of the whole supplied sequence.
/// Recorded progress and currently verified post-state remain distinct counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryDecision {
    Ready {
        recorded_prefix: usize,
        verified_post_prefix: usize,
        step: RecoveryStep,
    },
    Complete {
        completed: usize,
    },
    Reconcile {
        index: usize,
        recorded_prefix: usize,
        verified_post_prefix: usize,
    },
}

/// Validate the full structure and all supplied observations before selecting a step.
///
/// Exactly one final `State` and `Done* Pending? Queued*` checkpoint ordering are
/// required. Structural errors return `Err`; an observation conflict returns the
/// first conflicting index. The caller must qualify actual identities, observations
/// and checkpoint persistence separately before performing any effects.
pub fn plan_recovery(actions: &[RecoveryAction]) -> anyhow::Result<RecoveryDecision> {
    anyhow::ensure!(
        actions.last().is_some_and(|a| a.kind == ActionKind::State),
        "recovery requires a final State action"
    );
    anyhow::ensure!(
        actions
            .iter()
            .filter(|a| a.kind == ActionKind::State)
            .count()
            == 1,
        "recovery requires exactly one State action"
    );
    let recorded_prefix = actions
        .iter()
        .take_while(|a| a.checkpoint == Checkpoint::Done)
        .count();
    for (index, action) in actions.iter().enumerate().skip(recorded_prefix) {
        anyhow::ensure!(
            action.checkpoint != Checkpoint::Done,
            "recovery action {index}: Done checkpoint is outside completed prefix"
        );
        anyhow::ensure!(
            action.checkpoint != Checkpoint::Pending || index == recorded_prefix,
            "recovery action {index}: Pending checkpoint must immediately follow completed prefix"
        );
    }
    let mut verified_post_prefix = 0;
    let mut conflict = None;
    for (index, action) in actions.iter().enumerate() {
        let post = matches!(
            action.observed,
            ObservedRelation::After | ObservedRelation::Both
        );
        if index == verified_post_prefix && action.checkpoint != Checkpoint::Queued && post {
            verified_post_prefix += 1;
        }
        let valid = match action.checkpoint {
            Checkpoint::Done => post,
            Checkpoint::Pending => action.observed != ObservedRelation::Neither,
            Checkpoint::Queued => matches!(
                action.observed,
                ObservedRelation::Before | ObservedRelation::Both
            ),
        };
        if !valid && conflict.is_none() {
            conflict = Some(index);
        }
    }
    if let Some(index) = conflict {
        return Ok(RecoveryDecision::Reconcile {
            index,
            recorded_prefix,
            verified_post_prefix,
        });
    }
    if recorded_prefix == actions.len() {
        return Ok(RecoveryDecision::Complete {
            completed: actions.len(),
        });
    }
    let index = recorded_prefix;
    let step = match (actions[index].checkpoint, actions[index].observed) {
        (Checkpoint::Pending, ObservedRelation::After | ObservedRelation::Both) => {
            RecoveryStep::Acknowledge { index }
        }
        (Checkpoint::Pending, ObservedRelation::Before) => RecoveryStep::Apply { index },
        (Checkpoint::Queued, ObservedRelation::Both) => RecoveryStep::CheckpointOnly { index },
        (Checkpoint::Queued, ObservedRelation::Before) => RecoveryStep::Apply { index },
        _ => anyhow::bail!("recovery action {index}: inconsistent validated checkpoint"),
    };
    Ok(RecoveryDecision::Ready {
        recorded_prefix,
        verified_post_prefix,
        step,
    })
}
