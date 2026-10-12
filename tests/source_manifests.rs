use std::fs;
use std::path::Path;

use upskill::source_manifest::{
    CapturedSourceFile, SourceManifest, capture_source, validate_manifest,
};

const EMPTY_DIGEST: &str = "ecbfd2081fcd5a8c1c993491058a1913b092eb5cb7d16034f12807969ab4619b";

const MIXED_MODE_DIGEST: &str = "d349d581c2e5bdf60cc476d62688ee3abc3f12f7b815d961ac80c38a1ad6ed63";
const SWAPPED_MODE_DIGEST: &str =
    "84b31d2e445d49686c1ea82b36b61d9429de950c1a11dd0d003bf8c818236b1b";

fn single_a() -> SourceManifest {
    SourceManifest {
        files: vec![CapturedSourceFile {
            path: "a".into(),
            bytes: b"bc".to_vec(),
            content_sha256: "1e0bbd6c686ba050b8eb03ffeedc64fdc9d80947fce821abbe5d6dc8d252c5ac"
                .into(),
            mode: None,
        }],
        content_sha256: "1cb8488ccbda1fcef669daabf5ceab19c98708642534db354eacc4c3d4202b14".into(),
    }
}

fn single_ab() -> SourceManifest {
    SourceManifest {
        files: vec![CapturedSourceFile {
            path: "ab".into(),
            bytes: b"c".to_vec(),
            content_sha256: "2e7d2c03a9507ae265ecf5b5356885a53393a2029d241394997265a1a25aefc6"
                .into(),
            mode: None,
        }],
        content_sha256: "c8ca3f0b05498df16c899154aec285d419ccdac53d7d10e48db3323aec27187c".into(),
    }
}

fn assert_valid(manifest: &SourceManifest) {
    let before = manifest.clone();
    validate_manifest(manifest).unwrap();
    assert_eq!(*manifest, before);
}

fn assert_rejected(manifest: &SourceManifest, diagnostic: &str) {
    let before = manifest.clone();
    let result = validate_manifest(manifest);
    assert!(
        result.is_err(),
        "expected {diagnostic} rejection, got {result:?}"
    );
    let message = format!("{:#}", result.unwrap_err());
    assert!(
        message.contains(diagnostic),
        "expected {diagnostic:?}, got {message:?}"
    );
    assert_eq!(*manifest, before);
}

fn assert_capture_rejected(root: &Path, diagnostic: &str) {
    let result = capture_source(root);
    assert!(
        result.is_err(),
        "expected {diagnostic} rejection, got {result:?}"
    );
    let message = format!("{:#}", result.unwrap_err());
    assert!(
        message.contains(diagnostic),
        "expected {diagnostic:?}, got {message:?}"
    );
}

fn tree_records() -> Vec<CapturedSourceFile> {
    [
        (
            "RULE.claude.md",
            b"override\n".as_slice(),
            "46e0313ca59003e4def743b25cd1ed4a1276f99782e5cbf8bfa849ff4cf171db",
        ),
        (
            "a/SKILL.md",
            b"# Nested\n".as_slice(),
            "aeab417db4e43d83788b35bde1945cdfc5ac1d38879f02403798410b0e37107f",
        ),
        (
            "a/empty",
            b"".as_slice(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            "z.bin",
            [0, 255, 10].as_slice(),
            "712450d3c4a79eea9509e75dc1dacdeff58034df538536cfae2da882bd8a0c50",
        ),
    ]
    .into_iter()
    .map(|(path, bytes, hash)| CapturedSourceFile {
        path: path.into(),
        bytes: bytes.to_vec(),
        content_sha256: hash.into(),
        mode: if cfg!(unix) { Some(0o640) } else { None },
    })
    .collect()
}

fn write_tree(root: &Path, reversed: bool) {
    fs::create_dir_all(root.join("a")).unwrap();
    fs::create_dir(root.join("empty-directory")).unwrap();
    let mut records = tree_records();
    if reversed {
        records.reverse();
    }
    for record in records {
        let path = root.join(record.path);
        fs::write(&path, record.bytes).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o640)).unwrap();
        }
    }
}

#[test]
fn complete_source_matches_literal_vector_across_roots_and_orders() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    write_tree(first.path(), false);
    write_tree(second.path(), true);
    let captured = capture_source(first.path()).unwrap();
    assert_eq!(captured.files.len(), 4);
    assert_eq!(captured.files, tree_records());
    #[cfg(unix)]
    assert!(captured.files.iter().all(|file| file.mode == Some(0o640)));
    #[cfg(not(unix))]
    assert!(captured.files.iter().all(|file| file.mode.is_none()));
    let expected_digest = if cfg!(unix) {
        "d277aa179cdcdfc083aab28ccace1714cdf239315f9bdf6767764f4c9e27ded1"
    } else {
        "24ff8dddb80b667251644a4d68ff5bd8715e1df34fb132b3406b5153ea821cbf"
    };
    assert_eq!(captured.content_sha256, expected_digest);
    assert_valid(&captured);
    assert_eq!(capture_source(second.path()).unwrap(), captured);
}

#[test]
fn empty_source_has_versioned_framed_evidence() {
    let root = tempfile::tempdir().unwrap();
    let captured = capture_source(root.path()).unwrap();
    assert!(captured.files.is_empty());
    assert_eq!(captured.content_sha256, EMPTY_DIGEST);
    assert_valid(&captured);
}

#[test]
fn literal_framing_distinguishes_ambiguous_path_and_content_streams() {
    let a = single_a();
    let ab = single_ab();
    assert_valid(&a);
    assert_valid(&ab);
    assert_ne!(a.content_sha256, ab.content_sha256);
    let mut changed = a.clone();
    changed.content_sha256 = ab.content_sha256.clone();
    assert_rejected(&changed, "manifest digest");
    let mut changed = ab;
    changed.content_sha256 = a.content_sha256;
    assert_rejected(&changed, "manifest digest");
}

#[test]
fn mixed_mode_literal_digests_bind_modes_to_paths() {
    let mut original = single_a();
    original.files.extend(single_ab().files);
    assert_eq!(original.files.len(), 2);
    original.files[0].mode = Some(0o640);
    original.files[1].mode = Some(0o2640);
    original.content_sha256 = MIXED_MODE_DIGEST.into();
    assert_valid(&original);

    let mut swapped = original.clone();
    swapped.files[0].mode = Some(0o2640);
    swapped.files[1].mode = Some(0o640);
    assert_rejected(&swapped, "manifest digest");
    swapped.content_sha256 = SWAPPED_MODE_DIGEST.into();
    assert_valid(&swapped);
    assert_ne!(original.content_sha256, swapped.content_sha256);

    let mut wrong_digest = original;
    wrong_digest.content_sha256 = swapped.content_sha256;
    assert_rejected(&wrong_digest, "manifest digest");
}

#[test]
fn multibyte_utf8_path_matches_literal_byte_length_framing() {
    let pure = SourceManifest {
        files: vec![CapturedSourceFile {
            path: "雪.md".into(),
            bytes: b"bc".to_vec(),
            content_sha256: "1e0bbd6c686ba050b8eb03ffeedc64fdc9d80947fce821abbe5d6dc8d252c5ac"
                .into(),
            mode: None,
        }],
        content_sha256: "2c185ac1577faee63b84378f18a1fff47479369212bb97d0a5357b916c2699f4".into(),
    };
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("雪.md");
    fs::write(&path, b"bc").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o640)).unwrap();
    }
    let captured = capture_source(root.path()).unwrap();
    assert_eq!(captured.files.len(), 1);
    let mut expected_files = pure.files.clone();
    if cfg!(unix) {
        expected_files[0].mode = Some(0o640);
    }
    assert_eq!(captured.files, expected_files);
    let expected_digest = if cfg!(unix) {
        "4e8d9cb3aa95b2fbf1d1d03c77bfd5bbb74a3744c79c8ec19131e2fd0a186252"
    } else {
        "2c185ac1577faee63b84378f18a1fff47479369212bb97d0a5357b916c2699f4"
    };
    assert_eq!(captured.content_sha256, expected_digest);
    assert_valid(&pure);
    assert_valid(&captured);
}

#[test]
fn valid_permission_mode_boundaries_match_literal_evidence() {
    for (mode, digest) in [
        (
            0,
            "12ead865136b67ce0a5ae43391ba23e8315268825fec4f2344aa342e72429b3f",
        ),
        (
            0o7777,
            "b31224184e34373b695f52b22660b8030e6650eb0577bfac7faa5441fdfcbfe8",
        ),
    ] {
        let mut manifest = single_a();
        manifest.files[0].mode = Some(mode);
        manifest.content_sha256 = digest.into();
        assert_valid(&manifest);
        let mut absent = manifest;
        absent.files[0].mode = None;
        assert_rejected(&absent, "manifest digest");
    }
}

#[test]
fn lexical_paths_fail_before_digest_validation_without_mutation() {
    for path in [
        "", ".", "..", "/a", "a/", "a//b", "a/./b", "a/../b", "a\\b", "a:b", "a\n", "a\u{85}b",
    ] {
        let mut manifest = single_a();
        manifest.files[0].path = path.into();
        assert_rejected(&manifest, "source path");
    }
}

#[test]
fn duplicate_and_reversed_paths_fail_before_stale_manifest_digest() {
    let mut duplicate = single_a();
    duplicate.files.push(duplicate.files[0].clone());
    assert_rejected(&duplicate, "strict ascending");
    let mut reversed = single_ab();
    reversed.files.push(single_a().files.remove(0));
    assert_rejected(&reversed, "strict ascending");
}

#[test]
fn incorrect_file_digest_is_rejected_without_mutation() {
    let mut manifest = single_a();
    manifest.files[0].content_sha256 = "0".repeat(64);
    assert_rejected(&manifest, "file digest");
}

#[test]
fn out_of_range_mode_fails_before_manifest_digest() {
    let mut manifest = single_a();
    manifest.files[0].mode = Some(0o10000);
    assert_rejected(&manifest, "permission bits");
}

#[test]
fn incorrect_manifest_digest_is_rejected_without_mutation() {
    let mut manifest = single_a();
    manifest.content_sha256 = "0".repeat(64);
    assert_rejected(&manifest, "manifest digest");
}

#[test]
fn renamed_and_edited_files_change_their_evidence() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("a"), b"bc").unwrap();
    let original = capture_source(root.path()).unwrap();
    assert_eq!(original.files.len(), 1);
    fs::rename(root.path().join("a"), root.path().join("ab")).unwrap();
    let renamed = capture_source(root.path()).unwrap();
    assert_eq!(renamed.files.len(), 1);
    assert_eq!(renamed.files[0].path, "ab");
    assert_eq!(renamed.files[0].bytes, b"bc");
    assert_eq!(
        renamed.files[0].content_sha256,
        original.files[0].content_sha256
    );
    assert_ne!(renamed.content_sha256, original.content_sha256);
    fs::write(root.path().join("ab"), b"c").unwrap();
    let edited = capture_source(root.path()).unwrap();
    assert_eq!(edited.files.len(), 1);
    assert_eq!(edited.files[0].bytes, b"c");
    assert_ne!(
        edited.files[0].content_sha256,
        renamed.files[0].content_sha256
    );
    assert_ne!(edited.content_sha256, renamed.content_sha256);
    for manifest in [&original, &renamed, &edited] {
        assert_valid(manifest);
    }
}

#[test]
fn missing_and_regular_file_roots_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    assert_capture_rejected(&root.path().join("missing"), "source root");
    fs::write(root.path().join("file"), b"x").unwrap();
    assert_capture_rejected(&root.path().join("file"), "source root must be a directory");
}

#[test]
fn hidden_and_gitignore_matched_entries_are_captured() {
    let root = tempfile::tempdir().unwrap();
    let mode = if cfg!(unix) { Some(0o640) } else { None };
    let expected = vec![
        CapturedSourceFile {
            path: ".gitignore".into(),
            bytes: b"ignored\n".to_vec(),
            content_sha256: "a6c7e3c5ddd823efc8b7a30853eabc152dac866843bd7a39dc4a29e0a0eb68b3"
                .into(),
            mode,
        },
        CapturedSourceFile {
            path: ".hidden".into(),
            bytes: b"bc".to_vec(),
            content_sha256: "1e0bbd6c686ba050b8eb03ffeedc64fdc9d80947fce821abbe5d6dc8d252c5ac"
                .into(),
            mode,
        },
        CapturedSourceFile {
            path: "ignored".into(),
            bytes: b"bc".to_vec(),
            content_sha256: "1e0bbd6c686ba050b8eb03ffeedc64fdc9d80947fce821abbe5d6dc8d252c5ac"
                .into(),
            mode,
        },
    ];
    for file in &expected {
        let path = root.path().join(&file.path);
        fs::write(&path, &file.bytes).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o640)).unwrap();
        }
    }
    let captured = capture_source(root.path()).unwrap();
    assert_eq!(captured.files.len(), 3);
    assert_eq!(captured.files, expected);
    assert_valid(&captured);
}

#[cfg(unix)]
mod unix {
    use super::*;
    #[cfg(not(target_os = "macos"))]
    use std::ffi::OsString;
    #[cfg(not(target_os = "macos"))]
    use std::os::unix::ffi::OsStringExt;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::os::unix::net::UnixListener;
    use std::process::Command;

    #[test]
    fn mode_only_edit_changes_manifest_evidence() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("a");
        fs::write(&path, b"bc").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        let original = capture_source(root.path()).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
        let changed = capture_source(root.path()).unwrap();
        assert_eq!(original.files.len(), 1);
        assert_eq!(changed.files.len(), 1);
        assert_eq!(original.files[0].mode, Some(0o640));
        assert_eq!(changed.files[0].mode, Some(0o600));
        assert_eq!(original.files[0].bytes, changed.files[0].bytes);
        assert_eq!(
            original.files[0].content_sha256,
            changed.files[0].content_sha256
        );
        assert_ne!(original.content_sha256, changed.content_sha256);
        assert_valid(&original);
        assert_valid(&changed);
    }

    #[test]
    fn mixed_modes_preserve_each_files_permission_identity() {
        let root = tempfile::tempdir().unwrap();
        let expected = vec![
            CapturedSourceFile {
                path: "a".into(),
                bytes: b"bc".to_vec(),
                content_sha256: "1e0bbd6c686ba050b8eb03ffeedc64fdc9d80947fce821abbe5d6dc8d252c5ac"
                    .into(),
                mode: Some(0o640),
            },
            CapturedSourceFile {
                path: "ab".into(),
                bytes: b"c".to_vec(),
                content_sha256: "2e7d2c03a9507ae265ecf5b5356885a53393a2029d241394997265a1a25aefc6"
                    .into(),
                mode: Some(0o2640),
            },
        ];
        for file in &expected {
            let path = root.path().join(&file.path);
            fs::write(&path, &file.bytes).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(file.mode.unwrap())).unwrap();
        }
        let captured = capture_source(root.path()).unwrap();
        assert_eq!(captured.files.len(), 2);
        assert_eq!(captured.files, expected);
        assert_eq!(captured.content_sha256, MIXED_MODE_DIGEST);
        assert_valid(&captured);

        fs::set_permissions(root.path().join("a"), fs::Permissions::from_mode(0o2640)).unwrap();
        fs::set_permissions(root.path().join("ab"), fs::Permissions::from_mode(0o640)).unwrap();
        let swapped = capture_source(root.path()).unwrap();
        assert_eq!(swapped.files.len(), 2);
        let mut swapped_expected = expected;
        swapped_expected[0].mode = Some(0o2640);
        swapped_expected[1].mode = Some(0o640);
        assert_eq!(swapped.files, swapped_expected);
        assert_ne!(swapped.content_sha256, captured.content_sha256);
        assert_eq!(swapped.content_sha256, SWAPPED_MODE_DIGEST);
        assert_valid(&swapped);
    }

    #[test]
    fn maximum_permission_mode_is_captured_without_masking() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("a");
        fs::write(&path, b"bc").unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o7777)).unwrap();
        let captured = capture_source(root.path()).unwrap();
        assert_eq!(captured.files.len(), 1);
        let mut expected = single_a();
        expected.files[0].mode = Some(0o7777);
        expected.content_sha256 =
            "b31224184e34373b695f52b22660b8030e6650eb0577bfac7faa5441fdfcbfe8".into();
        assert_eq!(captured, expected);
        assert_valid(&captured);
    }

    #[test]
    fn invalid_lexical_directory_names_are_rejected_even_when_empty() {
        for name in ["back\\slash", "with:colon", "control\n", "unicode\u{85}"] {
            let root = tempfile::tempdir().unwrap();
            fs::create_dir(root.path().join(name)).unwrap();
            assert_capture_rejected(root.path(), "source path");
        }
    }

    #[test]
    fn hardlinked_regular_files_are_captured_by_each_path() {
        let root = tempfile::tempdir().unwrap();
        let first = root.path().join("a");
        fs::write(&first, b"bc").unwrap();
        fs::set_permissions(&first, fs::Permissions::from_mode(0o640)).unwrap();
        fs::hard_link(first, root.path().join("b")).unwrap();
        let captured = capture_source(root.path()).unwrap();
        assert_eq!(captured.files.len(), 2);
        assert_eq!(
            captured.files,
            vec![
                CapturedSourceFile {
                    path: "a".into(),
                    bytes: b"bc".to_vec(),
                    content_sha256:
                        "1e0bbd6c686ba050b8eb03ffeedc64fdc9d80947fce821abbe5d6dc8d252c5ac".into(),
                    mode: Some(0o640),
                },
                CapturedSourceFile {
                    path: "b".into(),
                    bytes: b"bc".to_vec(),
                    content_sha256:
                        "1e0bbd6c686ba050b8eb03ffeedc64fdc9d80947fce821abbe5d6dc8d252c5ac".into(),
                    mode: Some(0o640),
                },
            ]
        );
        assert_valid(&captured);
    }

    #[test]
    fn root_symlinks_with_trailing_separator_or_dot_are_rejected() {
        let parent = tempfile::tempdir().unwrap();
        fs::create_dir(parent.path().join("real")).unwrap();
        let link = parent.path().join("link");
        symlink(parent.path().join("real"), &link).unwrap();
        for suffix in ["", "/", "/."] {
            assert_capture_rejected(
                Path::new(&format!("{}{suffix}", link.display())),
                "root symlink",
            );
        }
    }

    #[test]
    fn entry_symlinks_to_files_directories_missing_targets_and_cycles_are_rejected() {
        for target in ["file", "directory", "missing", "."] {
            let root = tempfile::tempdir().unwrap();
            fs::write(root.path().join("file"), b"x").unwrap();
            fs::create_dir(root.path().join("directory")).unwrap();
            symlink(target, root.path().join("link")).unwrap();
            assert_capture_rejected(root.path(), "entry symlink");
        }
    }

    #[test]
    fn fifo_is_rejected_before_opening() {
        let root = tempfile::tempdir().unwrap();
        assert!(
            Command::new("mkfifo")
                .arg(root.path().join("pipe"))
                .status()
                .unwrap()
                .success()
        );
        assert_capture_rejected(root.path(), "special entry");
    }

    #[test]
    fn socket_is_rejected_before_opening() {
        let root = tempfile::tempdir().unwrap();
        let _listener = UnixListener::bind(root.path().join("socket")).unwrap();
        assert_capture_rejected(root.path(), "special entry");
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn non_utf8_name_is_rejected() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(OsString::from_vec(vec![0xff])), b"x").unwrap();
        assert_capture_rejected(root.path(), "non-UTF-8");
    }

    #[test]
    fn invalid_lexical_names_are_rejected() {
        for name in ["back\\slash", "with:colon", "control\n", "unicode\u{85}"] {
            let root = tempfile::tempdir().unwrap();
            fs::write(root.path().join(name), b"x").unwrap();
            assert_capture_rejected(root.path(), "source path");
        }
    }
}
