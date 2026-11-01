//! Physical root binding and nonblocking live writer exclusion only.

use crate::filesystem::{QualifiedScope, ScopeBinding, ScopeWriterGuard};
use crate::ownership::InstallationRoot;
use anyhow::{Result, ensure};

/// One logical installation root paired with its actual qualified directory.
#[derive(Debug)]
pub struct NativeRoot<'a> {
    pub root: InstallationRoot,
    pub scope: &'a QualifiedScope,
}

/// Captured logical and physical identity; this record is not a write capability.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRootBinding {
    pub root: InstallationRoot,
    pub binding: ScopeBinding,
}

/// Live advisory exclusion in physical-root order. Drop releases all guards.
/// No journal, pending marker, native effect or post-crash exclusion is supplied.
#[derive(Debug)]
pub struct NativeRootLocks<'a> {
    bindings: Vec<NativeRootBinding>,
    scopes: Vec<&'a QualifiedScope>,
    guards: Vec<ScopeWriterGuard<'a>>,
}
impl NativeRootLocks<'_> {
    /// Return captured bindings in ascending `(device, inode)` order.
    pub fn bindings(&self) -> &[NativeRootBinding] {
        &self.bindings
    }

    /// Revalidate every selected path and pinned directory while holding locks.
    /// This does not freeze the namespace or exclude non-cooperating editors.
    pub fn verify_bindings(&self) -> Result<()> {
        for ((binding, scope), guard) in self.bindings.iter().zip(&self.scopes).zip(&self.guards) {
            guard.verify_binding()?;
            ensure!(
                scope.binding()? == binding.binding,
                "native root binding changed while locked"
            );
        }
        Ok(())
    }
}

/// Bind exact logical paths, reject duplicate physical roots and acquire all
/// writer guards without waiting. On any failure, acquired guards are dropped.
/// No filesystem-profile admission, source authority or native success follows.
pub fn lock_native_roots<'a>(roots: &[NativeRoot<'a>]) -> Result<NativeRootLocks<'a>> {
    ensure!(
        !roots.is_empty(),
        "native delivery requires at least one root"
    );
    let mut ordered = Vec::new();
    for root in roots {
        crate::compile::skill_package::scope_identity(&root.root)?;
        let binding = root.scope.binding()?;
        ensure!(
            root.root.path.as_os_str() == binding.selected.as_os_str(),
            "logical native root does not match its qualified scope"
        );
        ordered.push((
            NativeRootBinding {
                root: root.root.clone(),
                binding,
            },
            root.scope,
        ));
    }
    order_roots(&mut ordered)?;
    let mut bindings = Vec::new();
    let mut scopes = Vec::new();
    let mut guards = Vec::new();
    for (binding, scope) in ordered {
        guards.push(scope.try_lock_writer()?);
        bindings.push(binding);
        scopes.push(scope);
    }
    let locks = NativeRootLocks {
        bindings,
        scopes,
        guards,
    };
    locks.verify_bindings()?;
    Ok(locks)
}

fn order_roots(ordered: &mut [(NativeRootBinding, &QualifiedScope)]) -> Result<()> {
    ordered.sort_by_key(|(root, _)| (root.binding.device, root.binding.inode));
    ensure!(
        ordered
            .windows(2)
            .all(|pair| (pair[0].0.binding.device, pair[0].0.binding.inode)
                != (pair[1].0.binding.device, pair[1].0.binding.inode)),
        "duplicate or aliased native roots"
    );
    Ok(())
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod tests {
    use super::*;
    use crate::ownership::OwnershipScope;

    #[test]
    fn physical_order_and_alias_detection_use_both_device_and_inode() {
        let tmp = tempfile::tempdir().unwrap();
        let root = InstallationRoot {
            scope: OwnershipScope::Project,
            path: tmp.path().into(),
        };
        let scope = QualifiedScope::open(&root).unwrap();
        // Sorting operates on captured claims; fake numeric IDs cannot construct
        // a guard or skip the real binding checks in lock_native_roots.
        let record = |device, inode| NativeRootBinding {
            root: root.clone(),
            binding: ScopeBinding {
                selected: root.path.clone(),
                resolved: root.path.clone(),
                device,
                inode,
            },
        };
        let mut rows = vec![
            (record(2, 1), &scope),
            (record(1, 9), &scope),
            (record(1, 1), &scope),
        ];
        order_roots(&mut rows).unwrap();
        assert_eq!(
            rows.iter()
                .map(|r| (r.0.binding.device, r.0.binding.inode))
                .collect::<Vec<_>>(),
            vec![(1, 1), (1, 9), (2, 1)]
        );
        let mut distinct = vec![(record(1, 1), &scope), (record(2, 1), &scope)];
        order_roots(&mut distinct).unwrap();
        let mut duplicate = vec![(record(1, 1), &scope), (record(1, 1), &scope)];
        assert!(order_roots(&mut duplicate).is_err());
    }
}
