//! Bundle resolution: walk the `requires` graph, union items, detect
//! conflicts and cycles. Per format-spec §3.7.
//!
//! Inputs come from [`crate::parse::bundle`] (`load`/`discover`); the
//! install layer in [`crate::pipeline`] consumes the [`Resolved`] output
//! to decide which item directories to render.

use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

use crate::model::Bundle;
use crate::pipeline::ItemKind;

/// What a bundle (and everything it transitively requires) installs.
///
/// `bundles` is the post-order topological walk — every required bundle
/// appears before the bundle that requires it. The entry bundle is the
/// last element. Resolvers preserve this order so callers that record
/// per-bundle state see dependencies first.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub bundles: Vec<Bundle>,
    pub items: ResolvedItems,
}

/// Union of every item across the reached bundles, deduplicated by
/// `(kind, name)` and sorted for deterministic install order.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ResolvedItems {
    pub rules: Vec<String>,
    pub skills: Vec<String>,
    pub agents: Vec<String>,
}

impl ResolvedItems {
    /// Total count across all kinds. Useful for install reports.
    pub fn total(&self) -> usize {
        self.rules.len() + self.skills.len() + self.agents.len()
    }

    /// Lookup by `(kind, name)`. Used by the filtered-install path so
    /// the SSOT walk can skip directories that aren't in the resolution.
    pub fn contains(&self, kind: ItemKind, name: &str) -> bool {
        let bucket = match kind {
            ItemKind::Rule => &self.rules,
            ItemKind::Skill => &self.skills,
            ItemKind::Agent => &self.agents,
        };
        bucket.iter().any(|n| n == name)
    }
}

/// Resolve `entry` transitively against `available` (every bundle the
/// caller discovered in the same source registry).
///
/// Errors:
/// - **missing requirement** — a `requires` name has no matching bundle
///   in `available`.
/// - **cycle** — `entry` (transitively) requires a bundle that
///   (transitively) requires `entry`.
/// - **item conflict** — two reached bundles list the same `(kind,
///   name)` tuple. Same name in different kinds is allowed.
///
/// Version constraints in `Requires.version` are accepted but **not
/// enforced** at this layer (kept as opaque strings). Constraint
/// matching is a follow-up; today the resolver picks the first matching
/// bundle by `name` regardless of version.
pub fn resolve(entry: &Bundle, available: &[Bundle]) -> Result<Resolved> {
    let by_name: BTreeMap<&str, &Bundle> = available.iter().map(|b| (b.name.as_str(), b)).collect();

    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut on_stack: BTreeSet<String> = BTreeSet::new();
    let mut order: Vec<Bundle> = Vec::new();

    visit(entry, &by_name, &mut visited, &mut on_stack, &mut order)?;

    let items = union_items(&order)?;
    Ok(Resolved {
        bundles: order,
        items,
    })
}

fn visit(
    bundle: &Bundle,
    by_name: &BTreeMap<&str, &Bundle>,
    visited: &mut BTreeSet<String>,
    on_stack: &mut BTreeSet<String>,
    order: &mut Vec<Bundle>,
) -> Result<()> {
    if visited.contains(&bundle.name) {
        return Ok(());
    }
    if on_stack.contains(&bundle.name) {
        // Should be unreachable because the caller checks visited first;
        // included for completeness.
        anyhow::bail!("bundle dependency cycle including `{}`", bundle.name);
    }
    on_stack.insert(bundle.name.clone());

    for req in &bundle.requires {
        if on_stack.contains(&req.name) {
            anyhow::bail!(
                "bundle dependency cycle: `{}` requires `{}` which is already on the resolution stack",
                bundle.name,
                req.name
            );
        }
        let dep = by_name.get(req.name.as_str()).ok_or_else(|| {
            anyhow::anyhow!(
                "bundle `{}` requires `{}` which was not found in the source registry",
                bundle.name,
                req.name
            )
        })?;
        visit(dep, by_name, visited, on_stack, order)?;
    }

    on_stack.remove(&bundle.name);
    visited.insert(bundle.name.clone());
    order.push(bundle.clone());
    Ok(())
}

fn union_items(bundles: &[Bundle]) -> Result<ResolvedItems> {
    let mut seen: BTreeMap<(ItemKind, String), String> = BTreeMap::new();
    for bundle in bundles {
        for name in &bundle.items.rules {
            insert_or_conflict(&mut seen, ItemKind::Rule, name, &bundle.name)?;
        }
        for name in &bundle.items.skills {
            insert_or_conflict(&mut seen, ItemKind::Skill, name, &bundle.name)?;
        }
        for name in &bundle.items.agents {
            insert_or_conflict(&mut seen, ItemKind::Agent, name, &bundle.name)?;
        }
    }

    let mut rules = Vec::new();
    let mut skills = Vec::new();
    let mut agents = Vec::new();
    for ((kind, name), _origin) in seen {
        match kind {
            ItemKind::Rule => rules.push(name),
            ItemKind::Skill => skills.push(name),
            ItemKind::Agent => agents.push(name),
        }
    }
    Ok(ResolvedItems {
        rules,
        skills,
        agents,
    })
}

fn insert_or_conflict(
    seen: &mut BTreeMap<(ItemKind, String), String>,
    kind: ItemKind,
    name: &str,
    origin: &str,
) -> Result<()> {
    let key = (kind, name.to_string());
    if let Some(prior) = seen.get(&key) {
        anyhow::bail!(
            "item conflict: {:?} `{}` is named by both bundles `{}` and `{}`",
            kind,
            name,
            prior,
            origin
        );
    }
    seen.insert(key, origin.to_string());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BundleItems, Requires, SchemaVersion};

    fn bundle(name: &str, items: BundleItems, requires: Vec<&str>) -> Bundle {
        Bundle {
            schema: SchemaVersion::new(1).unwrap(),
            name: name.to_string(),
            description: format!("test bundle {name}"),
            license: None,
            items,
            requires: requires
                .into_iter()
                .map(|n| Requires {
                    name: n.to_string(),
                    version: None,
                })
                .collect(),
            plugins: Default::default(),
            metadata: None,
            extra: Default::default(),
        }
    }

    fn items(rules: &[&str], skills: &[&str], agents: &[&str]) -> BundleItems {
        BundleItems {
            rules: rules.iter().map(|s| s.to_string()).collect(),
            skills: skills.iter().map(|s| s.to_string()).collect(),
            agents: agents.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn resolve_single_bundle_with_no_requires() {
        let entry = bundle("solo", items(&["r1"], &["s1"], &["a1"]), vec![]);
        let resolved = resolve(&entry, &[entry.clone()]).unwrap();

        assert_eq!(resolved.bundles.len(), 1);
        assert_eq!(resolved.bundles[0].name, "solo");
        assert_eq!(resolved.items.total(), 3);
        assert!(resolved.items.contains(ItemKind::Rule, "r1"));
        assert!(resolved.items.contains(ItemKind::Skill, "s1"));
        assert!(resolved.items.contains(ItemKind::Agent, "a1"));
    }

    #[test]
    fn resolve_walks_requires_in_dependency_order() {
        // entry → mid → leaf (mid requires leaf, entry requires mid)
        let leaf = bundle("leaf", items(&["leaf-rule"], &[], &[]), vec![]);
        let mid = bundle("mid", items(&[], &["mid-skill"], &[]), vec!["leaf"]);
        let entry = bundle("entry", items(&[], &[], &["entry-agent"]), vec!["mid"]);
        let available = vec![leaf.clone(), mid.clone(), entry.clone()];

        let resolved = resolve(&entry, &available).unwrap();

        let order: Vec<&str> = resolved.bundles.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(
            order,
            vec!["leaf", "mid", "entry"],
            "post-order topological"
        );
        assert_eq!(resolved.items.total(), 3);
    }

    #[test]
    fn resolve_unions_items_across_bundles() {
        let a = bundle("a", items(&["a-rule"], &["common-skill"], &[]), vec![]);
        let b = bundle("b", items(&["b-rule"], &[], &["b-agent"]), vec!["a"]);
        let resolved = resolve(&b, &[a, b.clone()]).unwrap();
        assert_eq!(resolved.items.rules.len(), 2);
        assert_eq!(resolved.items.skills, vec!["common-skill"]);
        assert_eq!(resolved.items.agents, vec!["b-agent"]);
    }

    #[test]
    fn resolve_dedupe_path_diamond_does_not_error() {
        // diamond: entry → A, entry → B, A → leaf, B → leaf.
        // `leaf` is reached twice; the resolver should visit it once.
        let leaf = bundle("leaf", items(&["leaf-rule"], &[], &[]), vec![]);
        let a = bundle("a", items(&["a-rule"], &[], &[]), vec!["leaf"]);
        let b = bundle("b", items(&["b-rule"], &[], &[]), vec!["leaf"]);
        let entry = bundle("entry", items(&[], &[], &[]), vec!["a", "b"]);
        let available = vec![leaf, a, b, entry.clone()];

        let resolved = resolve(&entry, &available).unwrap();
        assert_eq!(
            resolved.bundles.len(),
            4,
            "leaf reached twice, visited once"
        );
        // Items unioned without duplicates because each bundle's items
        // are disjoint.
        assert_eq!(resolved.items.rules.len(), 3);
    }

    #[test]
    fn resolve_errors_on_item_conflict() {
        // Both bundles list rule "x" — even though they're related by
        // requires, the spec calls this a conflict.
        let a = bundle("a", items(&["x"], &[], &[]), vec![]);
        let b = bundle("b", items(&["x"], &[], &[]), vec!["a"]);
        let err = resolve(&b, &[a, b.clone()]).expect_err("conflict");
        let msg = format!("{:#}", err);
        assert!(msg.contains("item conflict"), "{msg}");
        assert!(msg.contains("`x`"), "{msg}");
    }

    #[test]
    fn resolve_allows_same_name_in_different_kinds() {
        let a = bundle("a", items(&["shared"], &[], &[]), vec![]);
        let b = bundle("b", items(&[], &["shared"], &[]), vec!["a"]);
        let resolved = resolve(&b, &[a, b.clone()]).unwrap();
        assert_eq!(resolved.items.rules, vec!["shared"]);
        assert_eq!(resolved.items.skills, vec!["shared"]);
    }

    #[test]
    fn resolve_errors_on_cycle() {
        let a = bundle("a", items(&[], &[], &[]), vec!["b"]);
        let b = bundle("b", items(&[], &[], &[]), vec!["a"]);
        let err = resolve(&a, &[a.clone(), b]).expect_err("cycle");
        let msg = format!("{:#}", err);
        assert!(msg.contains("cycle"), "{msg}");
    }

    #[test]
    fn resolve_errors_on_missing_requirement() {
        let a = bundle("a", items(&[], &[], &[]), vec!["does-not-exist"]);
        let err = resolve(&a, &[a.clone()]).expect_err("missing");
        let msg = format!("{:#}", err);
        assert!(msg.contains("does-not-exist"), "{msg}");
        assert!(msg.contains("not found"), "{msg}");
    }
}
