use upskill::files::permissions::{
    DestinationMode::{Absent, Present},
    PermissionSource::{Generated, Resource},
    ResourceExecutionPolicy::{Disabled, OwnerExecutable},
    select_file_mode,
};

#[test]
fn generated_output_is_private() {
    for policy in [Disabled, OwnerExecutable] {
        assert_eq!(select_file_mode(Absent, Generated, policy).unwrap(), 0o600);
    }
}

#[test]
fn existing_ordinary_modes_are_exact() {
    for mode in [0o000, 0o400, 0o640, 0o755, 0o777] {
        for source in [
            Generated,
            Resource {
                captured_mode: Some(0o7777),
            },
        ] {
            for policy in [Disabled, OwnerExecutable] {
                assert_eq!(
                    select_file_mode(Present(Some(mode)), source, policy).unwrap(),
                    mode,
                    "existing ordinary mode was not preserved exactly"
                );
            }
        }
    }
}

#[test]
fn unknown_present_rejects_while_absent_selects_private() {
    assert!(
        select_file_mode(Present(None), Generated, Disabled).is_err(),
        "unknown existing mode was treated as absence"
    );
    assert_eq!(
        select_file_mode(Absent, Generated, Disabled).unwrap(),
        0o600
    );
}

#[test]
fn existing_special_modes_reject_without_a_result() {
    for mode in [0o1000, 0o2000, 0o4000, 0o2640] {
        assert!(
            select_file_mode(Present(Some(mode)), Generated, Disabled).is_err(),
            "unsupported existing special bits produced an application mode"
        );
    }
}

#[test]
fn disabled_resource_execution_never_copies_source_mode() {
    for captured_mode in [None, Some(0o000), Some(0o755), Some(0o7777)] {
        assert_eq!(
            select_file_mode(Absent, Resource { captured_mode }, Disabled).unwrap(),
            0o600,
            "disabled resource execution copied captured permissions"
        );
    }
}

#[test]
fn resource_execution_requires_owner_bit_and_explicit_policy() {
    for (captured_mode, expected) in [
        (0o100, 0o700),
        (0o755, 0o700),
        (0o4777, 0o700),
        (0o011, 0o600),
        (0o644, 0o600),
        (0o1000, 0o600),
        (0o2000, 0o600),
        (0o4000, 0o600),
    ] {
        assert_eq!(
            select_file_mode(
                Absent,
                Resource {
                    captured_mode: Some(captured_mode)
                },
                OwnerExecutable
            )
            .unwrap(),
            expected,
            "resource execution did not follow only the captured owner-execute bit"
        );
    }
}

#[test]
fn unknown_resource_mode_cannot_grant_new_execution() {
    assert!(
        select_file_mode(
            Absent,
            Resource {
                captured_mode: None
            },
            OwnerExecutable
        )
        .is_err(),
        "unknown resource mode granted new executable output"
    );
    assert_eq!(
        select_file_mode(
            Present(Some(0o640)),
            Resource {
                captured_mode: None
            },
            OwnerExecutable
        )
        .unwrap(),
        0o640
    );
}

#[test]
fn invalid_evidence_rejects_even_when_existing_mode_would_win() {
    for mode in [0o10000, u32::MAX] {
        assert!(
            select_file_mode(
                Absent,
                Resource {
                    captured_mode: Some(mode)
                },
                Disabled
            )
            .is_err(),
            "invalid captured resource mode was accepted"
        );
        assert!(
            select_file_mode(
                Present(Some(0o640)),
                Resource {
                    captured_mode: Some(mode)
                },
                Disabled
            )
            .is_err(),
            "unused invalid captured resource mode bypassed validation"
        );
        assert!(
            select_file_mode(Present(Some(mode)), Generated, Disabled).is_err(),
            "invalid destination mode was accepted"
        );
    }
}

#[test]
fn errors_identify_evidence_and_policy_context() {
    let cases = [
        (
            Present(Some(0o10000)),
            Generated,
            Disabled,
            "destination mode must contain permission bits only",
        ),
        (
            Absent,
            Resource {
                captured_mode: Some(0o10000),
            },
            Disabled,
            "captured resource mode must contain permission bits only",
        ),
        (
            Present(None),
            Generated,
            Disabled,
            "existing destination mode is unknown",
        ),
        (
            Present(Some(0o2640)),
            Generated,
            Disabled,
            "destination special permission bits are unsupported",
        ),
        (
            Absent,
            Resource {
                captured_mode: None,
            },
            OwnerExecutable,
            "resource owner-execute policy requires captured mode",
        ),
    ];
    for (destination, source, policy, expected) in cases {
        let diagnostic = select_file_mode(destination, source, policy)
            .unwrap_err()
            .to_string();
        assert!(
            diagnostic.contains(expected),
            "expected {expected:?}, got {diagnostic:?}"
        );
    }
}
