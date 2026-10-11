//! Acceptance tests for editing independently owned Markdown instructions.

use upskill::instructions::aggregate::{Contribution, reconcile};

#[test]
fn adding_two_contributions_preserves_manual_bytes() {
    let manual = "# Manual\nKeep this.\n";
    let edited = reconcile(
        manual,
        &[],
        &[
            Contribution::new("beta", "Beta guidance.\n"),
            Contribution::new("alpha", "Alpha guidance.\n"),
        ],
    )
    .unwrap();

    assert_eq!(
        edited.content,
        "# Manual\nKeep this.\n\n\
         <!-- upskill:begin alpha -->\nAlpha guidance.\n<!-- upskill:end alpha -->\n\n\
         <!-- upskill:begin beta -->\nBeta guidance.\n<!-- upskill:end beta -->\n"
    );
    assert_eq!(edited.owned.len(), 2);
}

#[test]
fn updating_one_contribution_preserves_other_owner_and_manual_text() {
    let manual = "# Manual\nKeep this.\n";
    let initial = reconcile(
        manual,
        &[],
        &[
            Contribution::new("alpha", "Alpha old.\n"),
            Contribution::new("beta", "Beta guidance.\n"),
        ],
    )
    .unwrap();

    let updated = reconcile(
        &initial.content,
        &initial.owned,
        &[
            Contribution::new("alpha", "Alpha new.\n"),
            Contribution::new("beta", "Beta guidance.\n"),
        ],
    )
    .unwrap();

    assert_eq!(
        updated.content,
        "# Manual\nKeep this.\n\n\
         <!-- upskill:begin alpha -->\nAlpha new.\n<!-- upskill:end alpha -->\n\n\
         <!-- upskill:begin beta -->\nBeta guidance.\n<!-- upskill:end beta -->\n"
    );
    assert_eq!(updated.owned.len(), 2);
}

#[test]
fn unowned_managed_marker_blocks_a_new_contribution() {
    let existing =
        "# Manual\n<!-- upskill:begin rogue -->\nUser text.\n<!-- upskill:end rogue -->\n";
    let error = reconcile(existing, &[], &[Contribution::new("alpha", "Guidance.\n")]).unwrap_err();
    assert!(error.to_string().contains("unowned"));
}

#[test]
fn contribution_body_cannot_inject_managed_markers() {
    let error = reconcile(
        "",
        &[],
        &[Contribution::new(
            "alpha",
            "Guidance.\n<!-- upskill:begin beta -->\n",
        )],
    )
    .unwrap_err();
    assert!(error.to_string().contains("marker"));
}

#[test]
fn removing_all_contributions_restores_manual_bytes() {
    let manual = "# Manual\nKeep this.";
    let initial = reconcile(manual, &[], &[Contribution::new("alpha", "Guidance.\n")]).unwrap();

    let removed = reconcile(&initial.content, &initial.owned, &[]).unwrap();
    assert_eq!(removed.content, manual);
    assert!(removed.owned.is_empty());
}

#[test]
fn removing_one_owner_leaves_the_other_then_restores_manual_text() {
    let manual = "# Manual\nKeep this.\n";
    let initial = reconcile(
        manual,
        &[],
        &[
            Contribution::new("alpha", "Alpha guidance.\n"),
            Contribution::new("beta", "Beta guidance.\n"),
        ],
    )
    .unwrap();

    let without_alpha = reconcile(
        &initial.content,
        &initial.owned,
        &[Contribution::new("beta", "Beta guidance.\n")],
    )
    .unwrap();
    assert!(without_alpha.content.starts_with(manual));
    assert!(!without_alpha.content.contains("Alpha guidance."));
    assert!(without_alpha.content.contains("Beta guidance."));
    assert_eq!(without_alpha.owned.len(), 1);

    let empty = reconcile(&without_alpha.content, &without_alpha.owned, &[]).unwrap();
    assert_eq!(empty.content, manual);
}

#[test]
fn edited_owned_contribution_blocks_update_and_removal() {
    let initial = reconcile(
        "Manual.\n",
        &[],
        &[Contribution::new("alpha", "Guidance.\n")],
    )
    .unwrap();
    let changed = initial.content.replace("Guidance.", "User edit.");

    let update_error = reconcile(
        &changed,
        &initial.owned,
        &[Contribution::new("alpha", "New guidance.\n")],
    )
    .unwrap_err();
    assert!(update_error.to_string().contains("edited"));
    let remove_error = reconcile(&changed, &initial.owned, &[]).unwrap_err();
    assert!(remove_error.to_string().contains("edited"));
}
