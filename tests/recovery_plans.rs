use upskill::operations::recovery::{
    ActionKind::{self, File, State},
    Checkpoint::{self, Done, Pending, Queued},
    ObservedRelation::{self, After, Before, Both, Neither},
    RecoveryAction,
    RecoveryDecision::{self, Complete, Ready, Reconcile},
    RecoveryStep::{Acknowledge, Apply, CheckpointOnly},
    plan_recovery,
};

fn expect(actions: &[RecoveryAction], expected: RecoveryDecision, label: &str) {
    let actual = plan_recovery(actions);
    assert!(
        actual.as_ref().is_ok_and(|value| *value == expected),
        "{label}: expected {expected:?}; actual {actual:?}"
    );
}
fn action(kind: ActionKind, checkpoint: Checkpoint, observed: ObservedRelation) -> RecoveryAction {
    RecoveryAction {
        kind,
        checkpoint,
        observed,
    }
}

#[test]
fn state_only_literal_outcomes() {
    let cases = [
        (
            Queued,
            Before,
            Ready {
                recorded_prefix: 0,
                verified_post_prefix: 0,
                step: Apply { index: 0 },
            },
            "State Queued Before applies",
        ),
        (
            Queued,
            Both,
            Ready {
                recorded_prefix: 0,
                verified_post_prefix: 0,
                step: CheckpointOnly { index: 0 },
            },
            "State Queued Both checkpoints only",
        ),
        (
            Queued,
            After,
            Reconcile {
                index: 0,
                recorded_prefix: 0,
                verified_post_prefix: 0,
            },
            "State Queued After reconciles",
        ),
        (
            Queued,
            Neither,
            Reconcile {
                index: 0,
                recorded_prefix: 0,
                verified_post_prefix: 0,
            },
            "State Queued Neither reconciles",
        ),
        (
            Pending,
            Before,
            Ready {
                recorded_prefix: 0,
                verified_post_prefix: 0,
                step: Apply { index: 0 },
            },
            "State Pending Before applies",
        ),
        (
            Pending,
            After,
            Ready {
                recorded_prefix: 0,
                verified_post_prefix: 1,
                step: Acknowledge { index: 0 },
            },
            "State Pending After acknowledges",
        ),
        (
            Pending,
            Both,
            Ready {
                recorded_prefix: 0,
                verified_post_prefix: 1,
                step: Acknowledge { index: 0 },
            },
            "State Pending Both acknowledges",
        ),
        (
            Pending,
            Neither,
            Reconcile {
                index: 0,
                recorded_prefix: 0,
                verified_post_prefix: 0,
            },
            "State Pending Neither reconciles",
        ),
        (
            Done,
            After,
            Complete { completed: 1 },
            "State Done After completes",
        ),
        (
            Done,
            Both,
            Complete { completed: 1 },
            "State Done Both completes",
        ),
        (
            Done,
            Before,
            Reconcile {
                index: 0,
                recorded_prefix: 1,
                verified_post_prefix: 0,
            },
            "State Done Before preserves recorded prefix",
        ),
        (
            Done,
            Neither,
            Reconcile {
                index: 0,
                recorded_prefix: 1,
                verified_post_prefix: 0,
            },
            "State Done Neither preserves recorded prefix",
        ),
    ];
    for (checkpoint, observed, expected, label) in cases {
        expect(&[action(State, checkpoint, observed)], expected, label);
    }
}

#[test]
fn file_literal_outcomes() {
    let cases = [
        (
            Queued,
            Before,
            Ready {
                recorded_prefix: 0,
                verified_post_prefix: 0,
                step: Apply { index: 0 },
            },
            "File Queued Before applies",
        ),
        (
            Queued,
            Both,
            Ready {
                recorded_prefix: 0,
                verified_post_prefix: 0,
                step: CheckpointOnly { index: 0 },
            },
            "File Queued Both checkpoints only",
        ),
        (
            Queued,
            After,
            Reconcile {
                index: 0,
                recorded_prefix: 0,
                verified_post_prefix: 0,
            },
            "File Queued After reconciles",
        ),
        (
            Queued,
            Neither,
            Reconcile {
                index: 0,
                recorded_prefix: 0,
                verified_post_prefix: 0,
            },
            "File Queued Neither reconciles",
        ),
        (
            Pending,
            Before,
            Ready {
                recorded_prefix: 0,
                verified_post_prefix: 0,
                step: Apply { index: 0 },
            },
            "File Pending Before applies",
        ),
        (
            Pending,
            After,
            Ready {
                recorded_prefix: 0,
                verified_post_prefix: 1,
                step: Acknowledge { index: 0 },
            },
            "File Pending After acknowledges",
        ),
        (
            Pending,
            Both,
            Ready {
                recorded_prefix: 0,
                verified_post_prefix: 1,
                step: Acknowledge { index: 0 },
            },
            "File Pending Both acknowledges",
        ),
        (
            Pending,
            Neither,
            Reconcile {
                index: 0,
                recorded_prefix: 0,
                verified_post_prefix: 0,
            },
            "File Pending Neither reconciles",
        ),
        (
            Done,
            After,
            Ready {
                recorded_prefix: 1,
                verified_post_prefix: 1,
                step: Apply { index: 1 },
            },
            "File Done After advances",
        ),
        (
            Done,
            Both,
            Ready {
                recorded_prefix: 1,
                verified_post_prefix: 1,
                step: Apply { index: 1 },
            },
            "File Done Both advances",
        ),
        (
            Done,
            Before,
            Reconcile {
                index: 0,
                recorded_prefix: 1,
                verified_post_prefix: 0,
            },
            "File Done Before preserves recorded prefix",
        ),
        (
            Done,
            Neither,
            Reconcile {
                index: 0,
                recorded_prefix: 1,
                verified_post_prefix: 0,
            },
            "File Done Neither preserves recorded prefix",
        ),
    ];
    for (checkpoint, observed, expected, label) in cases {
        expect(
            &[
                action(File, checkpoint, observed),
                action(State, Queued, Before),
            ],
            expected,
            label,
        );
    }
}

#[test]
fn multi_action_literal_outcomes() {
    let cases = [
        (
            vec![
                action(File, Done, After),
                action(File, Pending, After),
                action(State, Queued, Before),
            ],
            Ready {
                recorded_prefix: 1,
                verified_post_prefix: 2,
                step: Acknowledge { index: 1 },
            },
            "pending post extends verified prefix",
        ),
        (
            vec![
                action(File, Done, After),
                action(File, Pending, After),
                action(State, Queued, Neither),
            ],
            Reconcile {
                index: 2,
                recorded_prefix: 1,
                verified_post_prefix: 2,
            },
            "later Neither blocks acknowledgement",
        ),
        (
            vec![
                action(File, Done, After),
                action(File, Pending, Before),
                action(State, Queued, After),
            ],
            Reconcile {
                index: 2,
                recorded_prefix: 1,
                verified_post_prefix: 1,
            },
            "later After blocks application",
        ),
        (
            vec![
                action(File, Done, Before),
                action(File, Done, After),
                action(State, Queued, Before),
            ],
            Reconcile {
                index: 0,
                recorded_prefix: 2,
                verified_post_prefix: 0,
            },
            "first completed conflict stops verified prefix",
        ),
        (
            vec![action(File, Queued, Both), action(State, Queued, Before)],
            Ready {
                recorded_prefix: 0,
                verified_post_prefix: 0,
                step: CheckpointOnly { index: 0 },
            },
            "unchanged queued File needs checkpoint only",
        ),
        (
            vec![action(File, Done, Both), action(State, Queued, Both)],
            Ready {
                recorded_prefix: 1,
                verified_post_prefix: 1,
                step: CheckpointOnly { index: 1 },
            },
            "queued Both never extends verified prefix",
        ),
        (
            vec![action(File, Done, After), action(State, Pending, After)],
            Ready {
                recorded_prefix: 1,
                verified_post_prefix: 2,
                step: Acknowledge { index: 1 },
            },
            "pending final State is acknowledged",
        ),
        (
            vec![action(File, Done, After), action(State, Done, After)],
            Complete { completed: 2 },
            "complete includes final State",
        ),
        (
            vec![
                action(File, Queued, Before),
                action(File, Queued, Before),
                action(State, Queued, Before),
            ],
            Ready {
                recorded_prefix: 0,
                verified_post_prefix: 0,
                step: Apply { index: 0 },
            },
            "all queued starts at first File",
        ),
        (
            vec![
                action(File, Queued, Before),
                action(File, Queued, Before),
                action(State, Queued, Neither),
            ],
            Reconcile {
                index: 2,
                recorded_prefix: 0,
                verified_post_prefix: 0,
            },
            "full suffix Neither blocks Ready",
        ),
        (
            vec![
                action(File, Queued, Before),
                action(File, Queued, Before),
                action(State, Queued, After),
            ],
            Reconcile {
                index: 2,
                recorded_prefix: 0,
                verified_post_prefix: 0,
            },
            "full suffix After blocks Ready",
        ),
        (
            vec![action(File, Done, After), action(State, Done, Before)],
            Reconcile {
                index: 1,
                recorded_prefix: 2,
                verified_post_prefix: 1,
            },
            "contradicted completed State retains both recorded checkpoints",
        ),
    ];
    for (actions, expected, label) in cases {
        expect(&actions, expected, label);
    }
    expect(
        &[action(File, Queued, After), action(State, Queued, Neither)],
        Reconcile {
            index: 0,
            recorded_prefix: 0,
            verified_post_prefix: 0,
        },
        "first conflict is retained across later conflict",
    );
    expect(
        &[
            action(File, Done, Before),
            action(File, Pending, After),
            action(State, Queued, Before),
        ],
        Reconcile {
            index: 0,
            recorded_prefix: 1,
            verified_post_prefix: 0,
        },
        "pending post cannot extend broken verified prefix",
    );
}

#[test]
fn invalid_structure_is_rejected_before_observations() {
    let cases = [
        (vec![], "State", "empty requires final State"),
        (
            vec![action(File, Queued, Before)],
            "State",
            "Files only requires final State",
        ),
        (
            vec![action(State, Queued, Before), action(File, Queued, Before)],
            "final State",
            "State must be final",
        ),
        (
            vec![action(State, Queued, Before), action(State, Queued, Before)],
            "exactly one State",
            "multiple State actions reject",
        ),
        (
            vec![action(File, Queued, Before), action(State, Done, After)],
            "action 1",
            "Done after Queued rejects",
        ),
        (
            vec![action(File, Pending, Before), action(State, Done, After)],
            "action 1",
            "Done after Pending rejects",
        ),
        (
            vec![action(File, Queued, Before), action(State, Pending, Before)],
            "action 1",
            "Pending after Queued rejects",
        ),
        (
            vec![
                action(File, Pending, Before),
                action(State, Pending, Before),
            ],
            "action 1",
            "multiple Pending actions reject",
        ),
        (
            vec![action(File, Queued, Neither), action(State, Done, After)],
            "action 1",
            "late invalid checkpoint precedes early observation conflict",
        ),
    ];
    for (actions, context, label) in cases {
        let actual = plan_recovery(&actions);
        assert!(
            actual
                .as_ref()
                .is_err_and(|error| error.to_string().contains(context)),
            "{label}: expected Err containing {context:?}; actual {actual:?}"
        );
    }
}
