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

    assert_eq!(
        updated.content,
        "# Before\n\n<!-- upskill:begin alpha -->\nNew.\n<!-- upskill:end alpha -->\n\n# After\nKeep this.\n"
    );
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
fn added_separator_newline_blocks_update_and_removal() {
    let initial = reconcile("Manual.\n", &[], &[Contribution::new("alpha", "Old.\n")]).unwrap();
    let changed = initial
        .content
        .replacen("Manual.\n\n<!--", "Manual.\n\n\n<!--", 1);

    for desired in [Vec::new(), vec![Contribution::new("alpha", "New.\n")]] {
        let error = reconcile(&changed, &initial.owned, &desired).unwrap_err();
        assert!(error.to_string().contains("separator"));
    }
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

#[test]
fn manual_prefix_before_a_block_with_no_owned_separator_survives() {
    let initial = reconcile("", &[], &[Contribution::new("alpha", "Old.\n")]).unwrap();
    let with_prefix = format!("# Manual\n{}", initial.content);
    let updated = reconcile(
        &with_prefix,
        &initial.owned,
        &[Contribution::new("alpha", "New.\n")],
    )
    .unwrap();
    assert_eq!(
        updated.content,
        "# Manual\n<!-- upskill:begin alpha -->\nNew.\n<!-- upskill:end alpha -->\n"
    );
    let removed = reconcile(&updated.content, &updated.owned, &[]).unwrap();
    assert_eq!(removed.content, "# Manual\n");
}

#[test]
fn opening_marker_must_start_on_its_own_line() {
    let initial = reconcile("", &[], &[Contribution::new("alpha", "Old.\n")]).unwrap();
    let inline = format!("Manual prose{}", initial.content);
    assert!(
        reconcile(&inline, &initial.owned, &[])
            .unwrap_err()
            .to_string()
            .contains("marker")
    );
}

#[test]
fn same_length_body_edit_is_a_conflict_and_record_has_sha256() {
    let initial = reconcile("", &[], &[Contribution::new("alpha", "Guidance.\n")]).unwrap();
    assert_eq!(
        initial.owned[0].content_sha256,
        "7370422ff30a9380aedf0a5d7a9d701953b284b1576e9fb2bfa9a1cac392dfdf"
    );
    let changed = initial.content.replace("Guidance.", "Different");
    assert!(
        reconcile(&changed, &initial.owned, &[])
            .unwrap_err()
            .to_string()
            .contains("edited")
    );
}

#[test]
fn removing_first_owner_recalculates_second_separator_context() {
    for manual in ["Manual.", "Manual.\n\n\n"] {
        let initial = reconcile(
            manual,
            &[],
            &[
                Contribution::new("alpha", "Alpha.\n"),
                Contribution::new("beta", "Beta.\n"),
            ],
        )
        .unwrap();
        let without_alpha = reconcile(
            &initial.content,
            &initial.owned,
            &[Contribution::new("beta", "Beta.\n")],
        )
        .unwrap();
        assert_eq!(without_alpha.owned.len(), 1);
        let restored = reconcile(&without_alpha.content, &without_alpha.owned, &[]).unwrap();
        assert_eq!(restored.content, manual);
    }
}

#[test]
fn independently_added_owners_reconcile_in_file_order() {
    let manual = "Manual.";
    let beta = reconcile(manual, &[], &[Contribution::new("beta", "Beta.\n")]).unwrap();
    let both = reconcile(
        &beta.content,
        &beta.owned,
        &[
            Contribution::new("beta", "Beta.\n"),
            Contribution::new("alpha", "Alpha.\n"),
        ],
    )
    .unwrap();
    assert!(both.content.find("begin beta").unwrap() < both.content.find("begin alpha").unwrap());
    assert_eq!(both.owned[0].id, "alpha");
    let without_beta = reconcile(
        &both.content,
        &both.owned,
        &[Contribution::new("alpha", "Alpha new.\n")],
    )
    .unwrap();
    assert!(!without_beta.content.contains("Beta."));
    assert!(without_beta.content.contains("Alpha new."));
    let restored = reconcile(&without_beta.content, &without_beta.owned, &[]).unwrap();
    assert_eq!(restored.content, manual);
}

#[test]
fn verified_reader_returns_bodies_in_document_order() {
    let beta = reconcile("Manual", &[], &[Contribution::new("beta", "B\r\n")]).unwrap();
    let both = reconcile(
        &beta.content,
        &beta.owned,
        &[
            Contribution::new("alpha", "A\n"),
            Contribution::new("beta", "B\r\n"),
        ],
    )
    .unwrap();
    assert_eq!(
        upskill::instructions::aggregate::inspect_owned(&both.content, &both.owned).unwrap(),
        vec![
            Contribution::new("beta", "B\r\n"),
            Contribution::new("alpha", "A\n")
        ]
    );
}

#[test]
fn reader_and_editor_reject_the_same_unverified_spans() {
    use upskill::instructions::aggregate::inspect_owned;
    let edit = reconcile(
        "Manual.\n",
        &[],
        &[
            Contribution::new("alpha", "A\n"),
            Contribution::new("beta", "B\n"),
        ],
    )
    .unwrap();
    let mut duplicate = edit.owned.clone();
    duplicate.push(duplicate[0].clone());
    let mut overlap = edit.owned.clone();
    overlap[1].leading_newlines = 2;
    overlap[1].preceding_newlines = 0;
    let cases = vec![
        (edit.content.clone(), duplicate),
        (edit.content.clone(), overlap),
        (edit.content.replace("A\n", "Z\n"), edit.owned.clone()),
        (
            edit.content.replace("<!-- upskill:begin alpha -->\n", ""),
            edit.owned.clone(),
        ),
        (
            edit.content.replace("<!-- upskill:end alpha -->\n", ""),
            edit.owned.clone(),
        ),
        (
            edit.content
                .replace("<!-- upskill:begin alpha", "inline<!-- upskill:begin alpha"),
            edit.owned.clone(),
        ),
        (
            edit.content.replace(
                "<!-- upskill:begin alpha -->",
                "<!-- upskill:begin alpha-- >",
            ),
            edit.owned.clone(),
        ),
        (
            edit.content.replace("Manual.\n\n", "Manual.\n"),
            edit.owned.clone(),
        ),
        (
            edit.content.replace("Manual.\n\n", "Manual.\n\n\n"),
            edit.owned.clone(),
        ),
        (
            format!("{}<!-- upskill:unknown -->\n", edit.content),
            edit.owned.clone(),
        ),
        (
            format!("{}{}", edit.content, edit.content),
            edit.owned.clone(),
        ),
    ];
    for (index, (content, records)) in cases.into_iter().enumerate() {
        assert!(
            inspect_owned(&content, &records).is_err(),
            "reader case {index}"
        );
        assert!(
            reconcile(&content, &records, &[]).is_err(),
            "editor case {index}"
        );
    }
}
