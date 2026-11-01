//! Descriptor-bound private evidence; separate from authoritative graph state.
use super::execution::backend;
use super::qualification::Mount;
use super::{QualifiedScope, ScopeBinding, ScopeWriterGuard};
use crate::files::plan::{FileLocation, FileValue, PlannedFile};
use crate::native_delivery::state::{
    EvidenceBlob, EvidenceKind, EvidenceReference, NativeDeliveryState, NativeStateContext,
    evidence_references, parse_native_state, serialize_native_state, validate_blob,
    validate_reference,
};
use crate::ownership::{InstallationRoot, OwnershipScope};
use anyhow::{Context, Result, ensure};
use rustix::fs::{Mode, mkdirat};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

const DIRECTORY: &str = "native-evidence";
const PREFIX: &str = ".upskill/native-evidence";
const IGNORE: &[u8] = b"*\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Area {
    Evidence,
    Pending,
}
impl Area {
    fn directory(self) -> &'static str {
        match self {
            Self::Evidence => DIRECTORY,
            Self::Pending => "native-pending",
        }
    }
    fn prefix(self) -> &'static str {
        match self {
            Self::Evidence => PREFIX,
            Self::Pending => ".upskill/native-pending",
        }
    }
    fn purpose(self) -> Option<String> {
        match self {
            Self::Evidence => None,
            Self::Pending => Some("native-pending-v1".into()),
        }
    }
    fn domain(self) -> &'static [u8] {
        match self {
            Self::Evidence => b"upskill:native-evidence-namespace:v1\0",
            Self::Pending => b"upskill:native-marker-namespace:v1\0",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Owner {
    schema: u32,
    root: ScopeBinding,
    directory: (u64, u64),
    nonce: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    purpose: Option<String>,
}

/// Private immutable native evidence beneath an explicitly selected user root.
/// The borrowed writer guard must remain held. This store publishes no graph,
/// operation intent, pending marker or native effect. Checks sample bindings;
/// they do not exclude non-cooperating editors or establish native authority.
pub struct NativeEvidenceStore<'a> {
    scope: &'a QualifiedScope,
    guard: &'a ScopeWriterGuard<'a>,
    root: InstallationRoot,
    directory: File,
    mount: Mount,
    owner: Owner,
    owner_bytes: Vec<u8>,
    namespace: String,
    area: Area,
}
impl<'a> NativeEvidenceStore<'a> {
    /// Create a private store or open the same owned store. Partial/unowned
    /// initialization fails closed and requires reconciliation; it is not adopted.
    pub fn initialize(
        root: &InstallationRoot,
        scope: &'a QualifiedScope,
        guard: &'a ScopeWriterGuard<'a>,
    ) -> Result<Self> {
        Self::open_inner(root, scope, guard, true, Area::Evidence)?
            .context("private evidence directory missing")
    }

    /// Open an existing store without creating directories or files.
    pub fn open(
        root: &InstallationRoot,
        scope: &'a QualifiedScope,
        guard: &'a ScopeWriterGuard<'a>,
    ) -> Result<Option<Self>> {
        Self::open_inner(root, scope, guard, false, Area::Evidence)
    }

    pub(crate) fn open_marker(
        root: &InstallationRoot,
        scope: &'a QualifiedScope,
        guard: &'a ScopeWriterGuard<'a>,
        create: bool,
    ) -> Result<Option<Self>> {
        Self::open_inner(root, scope, guard, create, Area::Pending)
    }

    fn open_inner(
        root: &InstallationRoot,
        scope: &'a QualifiedScope,
        guard: &'a ScopeWriterGuard<'a>,
        create: bool,
        area: Area,
    ) -> Result<Option<Self>> {
        guard.require_scope(scope)?;
        crate::compile::skill_package::scope_identity(root)?;
        ensure!(
            (area == Area::Pending || root.scope == OwnershipScope::User)
                && root.path.as_os_str() == scope.binding()?.selected.as_os_str(),
            "private evidence requires its exact qualified user root"
        );
        require_untracked(root, &scope.binding()?.resolved, area)?;
        let base = scope.reopen_directory()?;
        qualify_directory(&base, &base, false)?;
        if create {
            make_directory(&base, ".upskill")?;
        }
        let parent = match optional_directory(&base, ".upskill")? {
            Some(parent) => parent,
            None => return Ok(None),
        };
        qualify_directory(&base, &parent, false)?;
        let created = create && make_directory(&parent, area.directory())?;
        let Some(directory) = optional_directory(&parent, area.directory())? else {
            return Ok(None);
        };
        if created {
            directory.set_permissions(std::fs::Permissions::from_mode(0o700))?;
            directory.sync_all()?;
        }
        qualify_directory(&base, &directory, true)?;
        let (owner, owner_bytes) = if created {
            let owner = Owner {
                schema: 1,
                root: scope.binding()?,
                directory: backend::identity(&directory)?,
                nonce: backend::nonce()?,
                purpose: area.purpose(),
            };
            let bytes = serde_json::to_vec(&owner)?;
            create_file(scope, root, area, "owner.json", &bytes, owner.directory)?;
            create_file(scope, root, area, ".gitignore", IGNORE, owner.directory)?;
            (owner, bytes)
        } else {
            let bytes = read_file(scope, root, area, "owner.json")?
                .context("unowned private evidence directory requires reconciliation")?
                .bytes;
            let value = crate::parse::json::parse_unique(std::str::from_utf8(&bytes)?)?;
            (serde_json::from_value::<Owner>(value)?, bytes)
        };
        ensure!(
            owner.schema == 1
                && owner.purpose == area.purpose()
                && owner.root == scope.binding()?
                && owner.directory == backend::identity(&directory)?
                && owner.nonce.len() == 32
                && owner
                    .nonce
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "private evidence ownership binding differs"
        );
        let mut hash = Sha256::new();
        hash.update(area.domain());
        hash.update((owner_bytes.len() as u64).to_be_bytes());
        hash.update(&owner_bytes);
        let namespace = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
        let store = Self {
            scope,
            guard,
            root: root.clone(),
            directory,
            mount: Mount::of(&base, Mount::queries())?,
            owner,
            owner_bytes,
            namespace,
            area,
        };
        store.verify()?;
        Ok(Some(store))
    }

    /// Machine-local identity derived from the owned store and qualified root.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Publish and sync an immutable blob. Equal existing content is verified
    /// and synced; drift is preserved and rejects rather than being overwritten.
    pub fn put(&self, blob: &EvidenceBlob) -> Result<()> {
        ensure!(
            self.area == Area::Evidence,
            "marker storage cannot publish native evidence"
        );
        validate_blob(blob)?;
        self.verify()?;
        let name = name(&blob.reference);
        let location = location(&self.root, self.area, &name);
        let before = read_file(self.scope, &self.root, self.area, &name)?;
        let after = FileValue {
            bytes: blob.bytes.clone(),
            mode: Some(0o600),
        };
        ensure!(
            before.is_none() || before.as_ref() == Some(&after),
            "immutable private evidence changed"
        );
        backend::apply_create_only_bound(
            self.scope,
            &PlannedFile {
                location: location.clone(),
                before,
                after: Some(after.clone()),
            },
            0,
            self.owner.directory,
        )?;
        backend::sync_post(self.scope, &location, &Some(after))?;
        self.verify()?;
        ensure!(
            self.read(&blob.reference)?.as_ref() == Some(blob),
            "published private evidence changed"
        );
        Ok(())
    }

    /// Read one exact typed reference, rejecting unsafe metadata or corruption.
    /// Malformed digests reject before filesystem path selection.
    pub fn read(&self, reference: &EvidenceReference) -> Result<Option<EvidenceBlob>> {
        ensure!(
            self.area == Area::Evidence,
            "marker storage cannot read native evidence"
        );
        validate_reference(reference)?;
        self.verify()?;
        let value = read_file(self.scope, &self.root, self.area, &name(reference))?;
        self.verify()?;
        let Some(value) = value else {
            return Ok(None);
        };
        let blob = EvidenceBlob {
            reference: reference.clone(),
            bytes: value.bytes,
        };
        validate_blob(&blob)?;
        Ok(Some(blob))
    }

    /// Validate supplied state, durably store its private evidence and return
    /// reference-only JSON. This never publishes the authoritative lockfile or
    /// establishes observed completion of a native transaction.
    pub fn store_state_evidence(&self, state: &NativeDeliveryState) -> Result<String> {
        self.require_context(&state.context)?;
        let encoded = serialize_native_state(state)?;
        for blob in &encoded.private_evidence {
            self.put(blob)?;
        }
        self.load_state(&encoded.json, &state.context)?;
        Ok(encoded.json)
    }

    /// Load supplied JSON using only its exact private evidence references.
    /// Schema/context/reference declarations validate before evidence lookup;
    /// the complete codec validates the reconstructed graph and configurations.
    /// Caller-supplied principal/additional roots are claims, not capabilities.
    pub fn load_state(
        &self,
        json: &str,
        expected: &NativeStateContext,
    ) -> Result<NativeDeliveryState> {
        self.require_context(expected)?;
        let references = evidence_references(json, expected)?;
        let blobs = references
            .iter()
            .map(|reference| {
                self.read(reference)?
                    .context("missing referenced private native evidence")
            })
            .collect::<Result<Vec<_>>>()?;
        let state = parse_native_state(json, expected, &blobs)?;
        self.verify()?;
        Ok(state)
    }

    pub(crate) fn new_operation_id(&self) -> Result<String> {
        self.verify()?;
        backend::nonce()
    }

    pub(crate) fn read_record(&self, name: &str) -> Result<Option<Vec<u8>>> {
        self.validate_record_name(name)?;
        self.verify()?;
        let bytes = read_file(self.scope, &self.root, self.area, name)?.map(|value| value.bytes);
        self.verify()?;
        Ok(bytes)
    }

    pub(crate) fn write_record(
        &self,
        name: &str,
        before: Option<&[u8]>,
        after: Option<&[u8]>,
    ) -> Result<()> {
        self.validate_record_name(name)?;
        self.verify()?;
        ensure!(
            self.read_record(name)?.as_deref() == before,
            "native operation record changed"
        );
        let value = |bytes: &[u8]| FileValue {
            bytes: bytes.to_vec(),
            mode: Some(0o600),
        };
        let action = PlannedFile {
            location: location(&self.root, self.area, name),
            before: before.map(value),
            after: after.map(value),
        };
        if before.is_none() && after.is_some() {
            backend::apply_create_only_bound(self.scope, &action, 0, self.owner.directory)?;
        } else {
            backend::apply_bound(self.scope, &action, 0, self.owner.directory)?;
        }
        backend::sync_post(self.scope, &action.location, &action.after)?;
        self.verify()?;
        ensure!(
            self.read_record(name)?.as_deref() == after,
            "native operation record publication changed"
        );
        Ok(())
    }

    fn validate_record_name(&self, name: &str) -> Result<()> {
        let valid = if self.area == Area::Pending {
            name == "pending.json"
        } else {
            [("intent-", ".yaml"), ("journal-", ".json")]
                .iter()
                .any(|(prefix, suffix)| {
                    name.strip_prefix(prefix)
                        .and_then(|value| value.strip_suffix(suffix))
                        .is_some_and(|id| {
                            id.len() == 32
                                && id
                                    .bytes()
                                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                        })
                })
        };
        ensure!(valid, "invalid native operation record name");
        Ok(())
    }

    fn require_context(&self, expected: &NativeStateContext) -> Result<()> {
        ensure!(
            self.area == Area::Evidence,
            "marker storage cannot load installation state"
        );
        ensure!(
            expected.private_root == self.root && expected.namespace == self.namespace,
            "native state context does not match the qualified private store"
        );
        self.verify()
    }

    fn verify(&self) -> Result<()> {
        self.guard.require_scope(self.scope)?;
        ensure!(
            self.scope.binding()? == self.owner.root,
            "private evidence root binding changed"
        );
        let base = self.scope.reopen_directory()?;
        self.mount
            .require_same(&Mount::of(&base, Mount::queries())?)?;
        self.mount
            .require_same(&Mount::of(&self.directory, Mount::queries())?)?;
        let parent = backend::directory(&base, ".upskill")?;
        let directory = backend::directory(&parent, self.area.directory())?;
        backend::spelling(&base, ".upskill")?;
        backend::spelling(&parent, self.area.directory())?;
        qualify_directory(&base, &base, false)?;
        qualify_directory(&base, &parent, false)?;
        qualify_directory(&base, &directory, true)?;
        ensure!(
            backend::identity(&directory)? == self.owner.directory
                && backend::identity(&self.directory)? == self.owner.directory,
            "private evidence directory binding changed"
        );
        ensure!(
            read_file(self.scope, &self.root, self.area, "owner.json")?
                .is_some_and(|v| v.bytes == self.owner_bytes),
            "private evidence owner changed"
        );
        ensure!(
            read_file(self.scope, &self.root, self.area, ".gitignore")?
                .is_some_and(|v| v.bytes == IGNORE),
            "private evidence ignore file changed"
        );
        require_untracked(&self.root, &self.owner.root.resolved, self.area)?;
        self.guard.verify_binding()
    }
}

fn name(reference: &EvidenceReference) -> String {
    let kind = match reference.kind {
        EvidenceKind::NativePrestate => "native-prestate",
        EvidenceKind::ConfigurationSnapshot => "configuration-snapshot",
    };
    format!("{kind}-{}", reference.digest)
}
fn location(root: &InstallationRoot, area: Area, name: &str) -> FileLocation {
    FileLocation {
        root: root.clone(),
        path: format!("{}/{name}", area.prefix()),
    }
}
fn read_file(
    scope: &QualifiedScope,
    root: &InstallationRoot,
    area: Area,
    name: &str,
) -> Result<Option<FileValue>> {
    let location = location(root, area, name);
    backend::qualify_metadata(scope, &location)?;
    let snapshot = scope.observe_destinations(&[location])?;
    let value = snapshot.observations()[0].value.clone();
    ensure!(
        value.as_ref().is_none_or(|v| v.mode == Some(0o600)),
        "private evidence entry must have mode 0600"
    );
    snapshot.verify_unchanged()?;
    Ok(value)
}
fn create_file(
    scope: &QualifiedScope,
    root: &InstallationRoot,
    area: Area,
    name: &str,
    bytes: &[u8],
    parent_identity: (u64, u64),
) -> Result<()> {
    backend::apply_create_only_bound(
        scope,
        &PlannedFile {
            location: location(root, area, name),
            before: None,
            after: Some(FileValue {
                bytes: bytes.to_vec(),
                mode: Some(0o600),
            }),
        },
        0,
        parent_identity,
    )
}
fn make_directory(parent: &File, name: &str) -> Result<bool> {
    match mkdirat(parent, name, Mode::from_raw_mode(0o700)) {
        Ok(()) => {
            parent.sync_all()?;
            Ok(true)
        }
        Err(rustix::io::Errno::EXIST) => Ok(false),
        Err(error) => Err(error.into()),
    }
}
fn optional_directory(parent: &File, name: &str) -> Result<Option<File>> {
    match backend::directory(parent, name) {
        Ok(directory) => {
            backend::spelling(parent, name)?;
            Ok(Some(directory))
        }
        Err(error)
            if error.downcast_ref::<rustix::io::Errno>() == Some(&rustix::io::Errno::NOENT) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}
fn qualify_directory(root: &File, directory: &File, private: bool) -> Result<()> {
    Mount::of(root, Mount::queries())?.require_same(&Mount::of(directory, Mount::queries())?)?;
    backend::require_plain_metadata(directory)?;
    let metadata = directory.metadata()?;
    ensure!(
        metadata.uid() == root.metadata()?.uid()
            && metadata.mode() & 0o022 == 0
            && (!private || metadata.mode() & 0o7777 == 0o700),
        "private evidence directory ownership or permissions are unqualified"
    );
    Ok(())
}
fn require_untracked(
    root: &InstallationRoot,
    resolved: &std::path::Path,
    area: Area,
) -> Result<()> {
    // A user root may itself belong to a dotfiles repository. The private ignore
    // file prevents normal staging; reject force-tracked evidence as well.
    let mut repository = false;
    for ancestor in resolved.ancestors() {
        match std::fs::symlink_metadata(ancestor.join(".git")) {
            Ok(_) => {
                repository = true;
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    if !repository {
        return Ok(());
    }
    let mut command = std::process::Command::new("git");
    command.current_dir(&root.path).args([
        "-c",
        "core.fsmonitor=false",
        "ls-files",
        "-z",
        "--",
        area.prefix(),
    ]);
    for (key, _) in
        std::env::vars_os().filter(|(key, _)| key.as_encoded_bytes().starts_with(b"GIT_"))
    {
        command.env_remove(key);
    }
    command.env("GIT_OPTIONAL_LOCKS", "0");
    let output = command
        .output()
        .context("check private evidence tracking")?;
    ensure!(
        output.status.success() && output.stdout.is_empty(),
        "private native evidence must remain outside tracked project content"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn marker_storage_binds_purpose_rejects_evidence_and_applies_exact_record_preconditions() {
        let tmp = tempfile::tempdir().unwrap();
        let root = InstallationRoot {
            scope: OwnershipScope::Project,
            path: tmp.path().into(),
        };
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        let marker = NativeEvidenceStore::open_marker(&root, &scope, &guard, true)
            .unwrap()
            .unwrap();
        let blob = super::process_tests::blob();
        assert!(marker.put(&blob).is_err());
        assert!(marker.read(&blob.reference).is_err());
        assert!(marker.read_record("../native-evidence/owner.json").is_err());
        marker
            .write_record("pending.json", None, Some(b"first"))
            .unwrap();
        assert!(
            marker
                .write_record("pending.json", Some(b"wrong"), Some(b"changed"))
                .is_err()
        );
        assert_eq!(
            marker.read_record("pending.json").unwrap(),
            Some(b"first".to_vec())
        );
        marker
            .write_record("pending.json", Some(b"first"), Some(b"second"))
            .unwrap();
        assert_eq!(
            marker.read_record("pending.json").unwrap(),
            Some(b"second".to_vec())
        );
        marker
            .write_record("pending.json", Some(b"second"), None)
            .unwrap();
        assert_eq!(marker.read_record("pending.json").unwrap(), None);
        let user = InstallationRoot {
            scope: OwnershipScope::User,
            path: root.path.clone(),
        };
        let evidence = NativeEvidenceStore::initialize(&user, &scope, &guard).unwrap();
        std::fs::rename(
            tmp.path().join(".upskill/native-pending"),
            tmp.path().join("retained-marker"),
        )
        .unwrap();
        std::fs::rename(
            tmp.path().join(".upskill/native-evidence"),
            tmp.path().join(".upskill/native-pending"),
        )
        .unwrap();
        assert!(NativeEvidenceStore::open_marker(&root, &scope, &guard, false).is_err());
        assert!(evidence.read(&blob.reference).is_err());
    }

    #[test]
    fn store_rejects_changed_mount_binding_before_read_or_publication() {
        let tmp = tempfile::tempdir().unwrap();
        let root = InstallationRoot {
            scope: OwnershipScope::User,
            path: tmp.path().into(),
        };
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        let mut store = NativeEvidenceStore::initialize(&root, &scope, &guard).unwrap();
        store.mount = crate::filesystem::qualification::tests::different_binding(&store.mount);
        let blob = super::process_tests::blob();
        assert!(store.read(&blob.reference).is_err());
        assert!(store.put(&blob).is_err());
        assert_eq!(
            std::fs::read_dir(tmp.path().join(PREFIX)).unwrap().count(),
            2
        );
    }

    #[test]
    fn bound_private_publication_rejects_wrong_directory_even_for_equal_content() {
        let tmp = tempfile::tempdir().unwrap();
        let root = InstallationRoot {
            scope: OwnershipScope::User,
            path: tmp.path().into(),
        };
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        let store = NativeEvidenceStore::initialize(&root, &scope, &guard).unwrap();
        let wrong = backend::identity(&scope.reopen_directory().unwrap()).unwrap();
        let action = PlannedFile {
            location: location(&root, Area::Evidence, "payload"),
            before: None,
            after: Some(FileValue {
                bytes: b"private".to_vec(),
                mode: Some(0o600),
            }),
        };
        assert!(backend::apply_create_only_bound(&scope, &action, 0, wrong).is_err());
        assert!(!tmp.path().join(PREFIX).join("payload").exists());
        assert_eq!(
            std::fs::read_dir(tmp.path().join(PREFIX)).unwrap().count(),
            2
        );
        backend::apply_create_only_bound(&scope, &action, 0, store.owner.directory).unwrap();
        let noop = PlannedFile {
            before: action.after.clone(),
            ..action
        };
        assert!(backend::apply_create_only_bound(&scope, &noop, 0, wrong).is_err());
        assert_eq!(
            std::fs::read(tmp.path().join(PREFIX).join("payload")).unwrap(),
            b"private"
        );
    }
}

#[cfg(test)]
mod process_tests {
    use super::*;
    pub(super) fn blob() -> EvidenceBlob {
        EvidenceBlob {
            reference: EvidenceReference {
                kind: EvidenceKind::NativePrestate,
                digest: "5eadfd350ace3da982d31480c7aa2f9f5d3b9fda1838b35196e9068c83f969bc".into(),
                length: 3,
            },
            bytes: vec![0, 255, 1],
        }
    }
    #[test]
    #[ignore]
    fn probe() {
        let Some(path) = std::env::var_os("UPSKILL_EVIDENCE_PROBE_ROOT") else {
            return;
        };
        let checkpoint = std::env::var("UPSKILL_EVIDENCE_PROBE_CHECKPOINT").unwrap();
        let root = InstallationRoot {
            scope: OwnershipScope::User,
            path: path.into(),
        };
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        let store = NativeEvidenceStore::open(&root, &scope, &guard)
            .unwrap()
            .unwrap();
        crate::filesystem::execution::TEST_HOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |label, _| {
                if label == checkpoint {
                    std::process::exit(73);
                }
                Ok(())
            }));
        });
        store.put(&blob()).unwrap();
        panic!("requested checkpoint was never reached");
    }

    #[test]
    fn fresh_process_store_reopens_after_interrupted_publication_boundaries() {
        for (checkpoint, present) in [
            ("before_effect", false),
            ("after_effect", true),
            ("after_effect_sync", true),
        ] {
            let tmp = tempfile::tempdir().unwrap();
            let root = InstallationRoot {
                scope: OwnershipScope::User,
                path: tmp.path().into(),
            };
            let namespace = {
                let scope = QualifiedScope::open(&root).unwrap();
                let guard = scope.try_lock_writer().unwrap();
                NativeEvidenceStore::initialize(&root, &scope, &guard)
                    .unwrap()
                    .namespace()
                    .to_owned()
            };
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "filesystem::native_evidence::process_tests::probe",
                    "--nocapture",
                ])
                .env("UPSKILL_EVIDENCE_PROBE_ROOT", tmp.path())
                .env("UPSKILL_EVIDENCE_PROBE_CHECKPOINT", checkpoint)
                .env("HOME", tmp.path())
                .env("USERPROFILE", tmp.path())
                .output()
                .unwrap();
            assert_eq!(
                output.status.code(),
                Some(73),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let scope = QualifiedScope::open(&root).unwrap();
            let guard = scope.try_lock_writer().unwrap();
            let store = NativeEvidenceStore::open(&root, &scope, &guard)
                .unwrap()
                .unwrap();
            assert_eq!(store.namespace(), namespace);
            assert_eq!(store.read(&blob().reference).unwrap(), present.then(blob));
            // Unreferenced staging is retained; no graph or native completion is inferred.
            assert!(!tmp.path().join(".upskill-lock.json").exists());
        }
    }
}
