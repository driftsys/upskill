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
    let repeated = reconcile(
        &updated.content,
        &updated.owned,
        &[
            Contribution::new("alpha", "Alpha new.\n"),
            Contribution::new("beta", "Beta guidance.\n"),
        ],
    )
    .unwrap();
    assert_eq!(repeated.content, updated.content);
}

#[test]
fn updating_a_block_preserves_manual_text_after_it() {
    let initial = reconcile("# Before\n", &[], &[Contribution::new("alpha", "Old.\n")]).unwrap();
    let with_tail = format!("{}\n# After\nKeep this.\n", initial.content);

    let updated = reconcile(
        &with_tail,
        &initial.owned,
        &[Contribution::new("alpha", "New.\n")],
    )
    .unwrap();

    assert!(updated.content.starts_with("# Before\n"));
    assert!(updated.content.ends_with("\n# After\nKeep this.\n"));
    assert!(updated.content.contains("New.\n"));
    assert!(!updated.content.contains("Old.\n"));
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

#[test]
fn removed_owned_separator_cannot_consume_a_manual_newline() {
    let manual = "Manual.\n";
    let initial = reconcile(manual, &[], &[Contribution::new("alpha", "Guidance.\n")]).unwrap();
    let changed = initial
        .content
        .replacen("Manual.\n\n<!--", "Manual.\n<!--", 1);

    let error = reconcile(&changed, &initial.owned, &[]).unwrap_err();
    assert!(error.to_string().contains("separator"));
}

#[test]
fn removed_owned_separator_cannot_consume_multiple_manual_newlines() {
    let manual = "Manual.\n\n\n";
    let initial = reconcile(manual, &[], &[Contribution::new("alpha", "Guidance.\n")]).unwrap();
    let changed = initial
        .content
        .replacen("Manual.\n\n\n\n<!--", "Manual.\n\n\n<!--", 1);

    let error = reconcile(&changed, &initial.owned, &[]).unwrap_err();
    assert!(error.to_string().contains("separator"));
    let removed = reconcile(&initial.content, &initial.owned, &[]).unwrap();
    assert_eq!(removed.content, manual);
}

#[test]
fn missing_or_unclosed_owned_block_blocks_removal() {
    let initial = reconcile(
        "Manual.\n",
        &[],
        &[Contribution::new("alpha", "Guidance.\n")],
    )
    .unwrap();
    let without_begin = initial
        .content
        .replace("<!-- upskill:begin alpha -->\n", "");
    assert!(
        reconcile(&without_begin, &initial.owned, &[])
            .unwrap_err()
            .to_string()
            .contains("missing")
    );

    let without_end = initial.content.replace("<!-- upskill:end alpha -->\n", "");
    assert!(
        reconcile(&without_end, &initial.owned, &[])
            .unwrap_err()
            .to_string()
            .contains("no end marker")
    );
}
