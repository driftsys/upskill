use super::{ExecutionRequest, backend, request};
use crate::files::plan::{FileLocation, FileValue, PlannedFile};
use crate::filesystem::{QualifiedScope, qualification::Mount};
use crate::operations::recovery::{
    ActionKind, Checkpoint, ObservedRelation, RecoveryAction, plan_recovery,
};
use crate::ownership::InstallationRoot;
use anyhow::{Context, Result, ensure};
use rustix::fs::{AtFlags, Mode, OFlags, mkdirat, openat, renameat, unlinkat};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};

type Identity = (u64, u64);
fn validate_backup_request(request: &ExecutionRequest) -> Result<()> {
    let backup = request
        .files
        .first()
        .context("missing backup prerequisite")?;
    let after = backup
        .after
        .as_ref()
        .context("backup must remain present")?;
    let original = request
        .state
        .before
        .as_ref()
        .context("backup requires original state")?;
    ensure!(
        after.mode == Some(0o600)
            && (backup.before.is_none() || backup.before.as_ref() == Some(after))
            && after.bytes == original.bytes,
        "backup must preserve exact original state with private create-only policy"
    );
    Ok(())
}
#[cfg(test)]
type PublicationSyncHook = Box<dyn FnMut(&File, usize) -> Result<()>>;
#[cfg(test)]
thread_local! {
    pub(super) static TEST_PUBLICATION_SYNC: std::cell::RefCell<Option<PublicationSyncHook>> = std::cell::RefCell::new(None);
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Evidence {
    pub digest: String,
    pub length: u64,
    pub mode: u32,
}
impl Evidence {
    pub fn of(v: &FileValue) -> Result<Self> {
        Ok(Self {
            digest: Sha256::digest(&v.bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
            length: v.bytes.len() as u64,
            mode: v.mode.context("journal requires selected mode")?,
        })
    }
    pub fn matches(&self, v: &FileValue) -> bool {
        Self::of(v).is_ok_and(|actual| actual == *self)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum Receipt {
    Queued,
    Pending,
    Done,
}
impl Receipt {
    pub fn checkpoint(self) -> Checkpoint {
        match self {
            Self::Queued => Checkpoint::Queued,
            Self::Pending => Checkpoint::Pending,
            Self::Done => Checkpoint::Done,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Disk {
    schema: u32,
    pub id: String,
    pub generation: u64,
    pub cleanup_started: bool,
    pub root: InstallationRoot,
    root_identity: Identity,
    pub actions: Vec<Action>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Action {
    pub path: String,
    pub state: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub create_only: bool,
    pub before: Option<Evidence>,
    pub after: Option<Evidence>,
    pub payload: Option<Payload>,
    pub receipt: Receipt,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Payload {
    pub name: String,
    identity: Identity,
}
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Owner {
    schema: u32,
    root: Identity,
    directory: Identity,
}

struct Store<'a> {
    scope: &'a QualifiedScope,
    dir: File,
    owner: Owner,
}
impl<'a> Store<'a> {
    fn open(scope: &'a QualifiedScope, create: bool) -> Result<Option<Self>> {
        let root = scope.reopen_directory()?;
        backend::require_plain_metadata(&root)?;
        ensure!(
            root.metadata()?.mode() & 0o022 == 0,
            "execution storage requires a scope without group/other write access"
        );
        if create {
            match mkdirat(&root, ".upskill", Mode::from_raw_mode(0o700)) {
                Ok(()) => root.sync_all()?,
                Err(rustix::io::Errno::EXIST) => {}
                Err(e) => return Err(e.into()),
            }
        }
        let parent = match backend::directory(&root, ".upskill") {
            Ok(dir) => dir,
            Err(e)
                if !create
                    && e.downcast_ref::<rustix::io::Errno>() == Some(&rustix::io::Errno::NOENT) =>
            {
                return Ok(None);
            }
            Err(e) => return Err(e),
        };
        backend::spelling(&root, ".upskill")?;
        let created = if create {
            match mkdirat(&parent, "state", Mode::from_raw_mode(0o700)) {
                Ok(()) => {
                    parent.sync_all()?;
                    true
                }
                Err(rustix::io::Errno::EXIST) => false,
                Err(e) => return Err(e.into()),
            }
        } else {
            false
        };
        let dir = match backend::directory(&parent, "state") {
            Ok(dir) => dir,
            Err(e)
                if !create
                    && e.downcast_ref::<rustix::io::Errno>() == Some(&rustix::io::Errno::NOENT) =>
            {
                return Ok(None);
            }
            Err(e) => return Err(e),
        };
        backend::spelling(&parent, "state")?;
        if created {
            dir.set_permissions(std::fs::Permissions::from_mode(0o700))?;
            dir.sync_all()?;
        }
        ensure!(
            dir.metadata()?.mode() & 0o7777 == 0o700,
            "execution state directory permissions must be private"
        );
        let owner = Owner {
            schema: 1,
            root: backend::identity(&root)?,
            directory: backend::identity(&dir)?,
        };
        let mount = Mount::of(&root, Mount::queries())?;
        mount.require_same(&Mount::of(&parent, Mount::queries())?)?;
        mount.require_same(&Mount::of(&dir, Mount::queries())?)?;
        backend::require_plain_metadata(&parent)?;
        backend::require_plain_metadata(&dir)?;
        ensure!(
            parent.metadata()?.mode() & 0o022 == 0
                && dir.metadata()?.uid() == root.metadata()?.uid(),
            "execution storage ownership or parent permissions are unqualified"
        );
        let store = Self { scope, dir, owner };
        if created {
            store.new_file("owner.json", &serde_json::to_vec(&store.owner)?)?;
            store.new_file(".gitignore", b"*\n!.gitignore\n")?;
        } else {
            let (bytes, _) = store
                .read("owner.json")?
                .context("unowned execution state directory requires reconciliation")?;
            ensure!(
                serde_json::from_slice::<Owner>(&bytes)? == store.owner,
                "execution state directory ownership binding differs"
            );
            ensure!(
                store.read(".gitignore")?.map(|v| v.0) == Some(b"*\n!.gitignore\n".to_vec()),
                "execution ignore file changed"
            );
        }
        store.verify()?;
        Ok(Some(store))
    }
    fn verify(&self) -> Result<()> {
        self.scope.verify_binding()?;
        let root = self.scope.reopen_directory()?;
        let parent = backend::directory(&root, ".upskill")?;
        let dir = backend::directory(&parent, "state")?;
        backend::spelling(&root, ".upskill")?;
        backend::spelling(&parent, "state")?;
        let mount = Mount::of(&root, Mount::queries())?;
        mount.require_same(&Mount::of(&parent, Mount::queries())?)?;
        mount.require_same(&Mount::of(&dir, Mount::queries())?)?;
        backend::require_plain_metadata(&root)?;
        backend::require_plain_metadata(&parent)?;
        backend::require_plain_metadata(&dir)?;
        ensure!(
            root.metadata()?.mode() & 0o022 == 0
                && parent.metadata()?.mode() & 0o022 == 0
                && dir.metadata()?.uid() == root.metadata()?.uid(),
            "execution storage permissions changed"
        );
        ensure!(
            backend::identity(&root)? == self.owner.root
                && backend::identity(&dir)? == self.owner.directory
                && dir.metadata()?.mode() & 0o7777 == 0o700,
            "execution state directory binding changed"
        );
        Ok(())
    }
    fn read(&self, name: &str) -> Result<Option<(Vec<u8>, Identity)>> {
        self.verify()?;
        let file = match openat(
            &self.dir,
            name,
            OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(fd) => File::from(fd),
            Err(rustix::io::Errno::NOENT) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        backend::require_plain_metadata(&file)?;
        let before = file.metadata()?;
        ensure!(
            before.is_file()
                && before.nlink() == 1
                && before.mode() & 0o7777 == 0o600
                && before.uid() == self.dir.metadata()?.uid(),
            "execution storage entry is not a private singly-linked regular file"
        );
        let mut bytes = Vec::new();
        (&file).read_to_end(&mut bytes)?;
        let after = file.metadata()?;
        ensure!(
            before.len() == bytes.len() as u64
                && before.len() == after.len()
                && before.mtime() == after.mtime()
                && before.mtime_nsec() == after.mtime_nsec()
                && before.ctime() == after.ctime()
                && before.ctime_nsec() == after.ctime_nsec(),
            "execution storage entry changed during read"
        );
        self.verify()?;
        Ok(Some((bytes, (before.dev(), before.ino()))))
    }
    fn new_file(&self, name: &str, bytes: &[u8]) -> Result<Identity> {
        self.verify()?;
        let mut file = File::from(openat(
            &self.dir,
            name,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o600),
        )?);
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        backend::require_plain_metadata(&file)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        self.dir.sync_all()?;
        let id = backend::identity(&file)?;
        ensure!(
            self.read(name)?.is_some_and(|v| v.0 == bytes && v.1 == id),
            "new execution storage entry changed"
        );
        Ok(id)
    }
    fn publish(&self, bytes: &[u8], old: Option<&[u8]>) -> Result<()> {
        let temp = format!("journal-{}", backend::nonce()?);
        let id = self.new_file(&temp, bytes)?;
        ensure!(
            self.read("operation.json")?
                .as_ref()
                .map(|v| v.0.as_slice())
                == old,
            "execution journal changed before publication"
        );
        ensure!(
            self.read(&temp)?.is_some_and(|v| v.1 == id && v.0 == bytes),
            "staged journal changed"
        );
        self.verify()?;
        let generation = usize::try_from(serde_json::from_slice::<Disk>(bytes)?.generation)?;
        super::checkpoint("before_journal_rename", generation)?;
        renameat(&self.dir, temp.as_str(), &self.dir, "operation.json")?;
        super::checkpoint("after_journal_rename", generation)?;
        self.sync_publication(generation)?;
        super::checkpoint("after_journal_sync", generation)?;
        self.verify()?;
        Ok(())
    }
    fn sync_publication(&self, generation: usize) -> Result<()> {
        #[cfg(test)]
        if let Some(result) = TEST_PUBLICATION_SYNC.with(|hook| {
            hook.borrow_mut()
                .as_mut()
                .map(|sync| sync(&self.dir, generation))
        }) {
            return result;
        }
        let _ = generation;
        self.dir.sync_all()?;
        Ok(())
    }
}

pub(super) struct Journal<'a> {
    store: Store<'a>,
    pub disk: Disk,
    bytes: Vec<u8>,
}
impl<'a> Journal<'a> {
    pub fn create(scope: &'a QualifiedScope, request: &ExecutionRequest) -> Result<Self> {
        Self::create_inner(scope, request, false)
    }
    pub fn create_with_backup(
        scope: &'a QualifiedScope,
        request: &ExecutionRequest,
    ) -> Result<Self> {
        Self::create_inner(scope, request, true)
    }
    fn create_inner(
        scope: &'a QualifiedScope,
        request: &ExecutionRequest,
        backup: bool,
    ) -> Result<Self> {
        request::validate(request)?;
        if backup {
            validate_backup_request(request)?;
        }
        let store = Store::open(scope, true)?.context("missing execution state directory")?;
        ensure!(
            store.read("operation.json")?.is_none(),
            "an existing operation must be recovered before application"
        );
        super::checkpoint("after_setup", 0)?;
        let id = backend::nonce()?;
        let mut actions = Vec::new();
        for (index, action) in request
            .files
            .iter()
            .chain(std::iter::once(&request.state))
            .enumerate()
        {
            let payload = if let Some(v) = &action.after {
                let name = format!("payload-{id}-{index}");
                Some(Payload {
                    identity: store.new_file(&name, &v.bytes)?,
                    name,
                })
            } else {
                None
            };
            super::checkpoint("after_payload", index)?;
            actions.push(Action {
                path: action.location.path.clone(),
                state: index == request.files.len(),
                create_only: backup && index == 0,
                before: action.before.as_ref().map(Evidence::of).transpose()?,
                after: action.after.as_ref().map(Evidence::of).transpose()?,
                payload,
                receipt: Receipt::Queued,
            });
        }
        let disk = Disk {
            schema: if backup { 2 } else { 1 },
            id,
            generation: 0,
            cleanup_started: false,
            root: request.state.location.root.clone(),
            root_identity: store.owner.root,
            actions,
        };
        let bytes = serde_json::to_vec(&disk)?;
        super::checkpoint("before_initial_journal", 0)?;
        store.publish(&bytes, None)?;
        super::checkpoint("after_initial_journal", 0)?;
        Ok(Self { store, disk, bytes })
    }
    pub fn load(scope: &'a QualifiedScope) -> Result<Option<Self>> {
        let Some(store) = Store::open(scope, false)? else {
            return Ok(None);
        };
        let Some((bytes, _)) = store.read("operation.json")? else {
            return Ok(None);
        };
        let disk: Disk =
            serde_json::from_slice(&bytes).context("parse strict execution journal")?;
        ensure!(
            matches!(disk.schema, 1 | 2)
                && disk.root_identity == store.owner.root
                && disk.id.len() == 32
                && disk.id.bytes().all(|b| b.is_ascii_hexdigit()),
            "unsupported or mismatched execution journal"
        );
        let journal = Self { store, disk, bytes };
        journal.validate()?;
        Ok(Some(journal))
    }
    fn validate(&self) -> Result<()> {
        let backup = self.disk.schema == 2;
        ensure!(
            self.disk
                .actions
                .iter()
                .enumerate()
                .all(|(i, a)| a.create_only == (backup && i == 0)),
            "invalid create-only journal ordering"
        );
        if backup {
            let first = self.disk.actions.first().context("missing backup action")?;
            let state = self.disk.actions.last().context("missing final state")?;
            let after = first.after.as_ref().context("backup must remain present")?;
            let original = state
                .before
                .as_ref()
                .context("backup requires original state evidence")?;
            ensure!(
                !first.state
                    && state.state
                    && after.mode == 0o600
                    && (first.before.is_none() || first.before.as_ref() == Some(after))
                    && after.digest == original.digest
                    && after.length == original.length,
                "backup evidence differs from original state or private create-only policy"
            );
        }
        let mut files = Vec::new();
        let mut states = Vec::new();
        let mut steps = Vec::new();
        for (i, a) in self.disk.actions.iter().enumerate() {
            let after = if self.disk.cleanup_started {
                ensure!(
                    self.disk.actions.iter().all(|a| a.receipt == Receipt::Done),
                    "cleanup marker requires completed operation"
                );
                match (&a.after, &a.payload) {
                    (Some(value), Some(payload)) => {
                        ensure!(
                            payload.name == format!("payload-{}-{i}", self.disk.id),
                            "invalid completed payload path"
                        );
                        Some(FileValue {
                            bytes: vec![],
                            mode: Some(value.mode),
                        })
                    }
                    (None, None) => None,
                    _ => anyhow::bail!("invalid completed payload binding"),
                }
            } else {
                self.payload(i)?
            };
            let before = a.before.as_ref().map(|v| FileValue {
                bytes: vec![],
                mode: Some(v.mode),
            });
            for value in [&a.before, &a.after].into_iter().flatten() {
                ensure!(
                    value.digest.len() == 64 && value.digest.bytes().all(|b| b.is_ascii_hexdigit()),
                    "invalid journal digest"
                );
            }
            let planned = PlannedFile {
                location: FileLocation {
                    root: self.disk.root.clone(),
                    path: a.path.clone(),
                },
                before,
                after,
            };
            if a.state {
                states.push(planned);
            } else {
                files.push(planned);
            }
            steps.push(RecoveryAction {
                kind: if a.state {
                    ActionKind::State
                } else {
                    ActionKind::File
                },
                checkpoint: a.receipt.checkpoint(),
                observed: if a.receipt == Receipt::Done {
                    ObservedRelation::After
                } else {
                    ObservedRelation::Before
                },
            });
        }
        plan_recovery(&steps)?;
        ensure!(states.len() == 1, "journal requires one final state action");
        request::validate(&ExecutionRequest {
            files,
            state: states.remove(0),
        })?;
        let locations: Vec<_> = self
            .disk
            .actions
            .iter()
            .map(|a| FileLocation {
                root: self.disk.root.clone(),
                path: a.path.clone(),
            })
            .collect();
        self.store.scope.observe_destinations(&locations)?;
        Ok(())
    }
    pub fn payload(&self, index: usize) -> Result<Option<FileValue>> {
        let a = &self.disk.actions[index];
        match (&a.after, &a.payload) {
            (None, None) => Ok(None),
            (Some(expected), Some(payload)) => {
                ensure!(
                    payload.name == format!("payload-{}-{index}", self.disk.id),
                    "invalid journal payload path"
                );
                let (bytes, id) = self
                    .store
                    .read(&payload.name)?
                    .context("missing operation payload")?;
                let value = FileValue {
                    bytes,
                    mode: Some(expected.mode),
                };
                ensure!(
                    id == payload.identity && expected.matches(&value),
                    "operation payload evidence changed"
                );
                Ok(Some(value))
            }
            _ => anyhow::bail!("journal payload does not match action"),
        }
    }
    pub fn record(&mut self, index: usize, receipt: Receipt) -> Result<()> {
        ensure!(
            !self.disk.cleanup_started,
            "cannot advance completed cleanup journal"
        );
        self.disk.actions[index].receipt = receipt;
        self.disk.generation = self
            .disk
            .generation
            .checked_add(1)
            .context("journal generation overflow")?;
        self.validate()?;
        let bytes = serde_json::to_vec(&self.disk)?;
        self.store.publish(&bytes, Some(&self.bytes))?;
        self.bytes = bytes;
        Ok(())
    }
    pub fn cleanup(mut self) -> Result<()> {
        ensure!(
            self.disk.actions.iter().all(|a| a.receipt == Receipt::Done),
            "cannot clean incomplete operation"
        );
        if !self.disk.cleanup_started {
            self.disk.cleanup_started = true;
            self.disk.generation = self
                .disk
                .generation
                .checked_add(1)
                .context("journal generation overflow")?;
            let bytes = serde_json::to_vec(&self.disk)?;
            self.store.publish(&bytes, Some(&self.bytes))?;
            self.bytes = bytes;
        }
        super::checkpoint("after_cleanup_marker", 0)?;
        for (index, action) in self.disk.actions.iter().enumerate() {
            if let Some(payload) = &action.payload {
                if let Some((bytes, id)) = self.store.read(&payload.name)? {
                    let after = action
                        .after
                        .as_ref()
                        .context("missing cleanup payload evidence")?;
                    let value = FileValue {
                        bytes,
                        mode: Some(after.mode),
                    };
                    ensure!(
                        id == payload.identity && after.matches(&value),
                        "cleanup payload changed; retained completed receipt"
                    );
                    self.store.verify()?;
                    unlinkat(&self.store.dir, payload.name.as_str(), AtFlags::empty())?;
                    self.store.dir.sync_all()?;
                    super::checkpoint("after_cleanup_payload", index)?;
                }
            }
        }
        ensure!(
            self.store
                .read("operation.json")?
                .as_ref()
                .map(|v| v.0.as_slice())
                == Some(self.bytes.as_slice()),
            "completed journal changed"
        );
        self.store.verify()?;
        super::checkpoint("before_cleanup_receipt_removal", 0)?;
        unlinkat(&self.store.dir, "operation.json", AtFlags::empty())?;
        self.store.dir.sync_all()?;
        Ok(())
    }
}
#[cfg(test)]
#[path = "journal_tests.rs"]
mod tests;
