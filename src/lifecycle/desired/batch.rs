//! Complete batch preflight over supplied captures and lexical destinations.
use super::{DesiredInstallationSet, RuleDesiredSetRequest, compile_rule_desired_set};
use crate::operations::plan::DesiredArtifact;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredSetDiagnostic {
    pub request_indices: Vec<usize>,
    pub installation_ids: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesiredSetsOutcome {
    Ready(Vec<DesiredInstallationSet>),
    Blocked(Vec<DesiredSetDiagnostic>),
}

/// Compile every independent request; expose complete sets only if all preflight
/// checks pass. Root identity is raw spelling and scope, not physical identity.
pub fn compile_rule_desired_sets(requests: &[RuleDesiredSetRequest<'_>]) -> DesiredSetsOutcome {
    let mut diagnostics = Vec::new();
    let mut successful = Vec::new();
    for (index, request) in requests.iter().enumerate() {
        match compile_rule_desired_set(*request) {
            Ok(set) => successful.push((index, set)),
            Err(error) => diagnostics.push(diagnostic(requests, vec![index], format!("{error:#}"))),
        }
        for previous in 0..index {
            if requests[previous].installation.owner.installation_id
                == request.installation.owner.installation_id
            {
                diagnostics.push(diagnostic(
                    requests,
                    vec![previous, index],
                    "duplicate installation ID".into(),
                ));
            }
        }
    }
    for (position, (left_index, left)) in successful.iter().enumerate() {
        for (right_index, right) in &successful[position + 1..] {
            if left.installation.root.scope != right.installation.root.scope
                || left.installation.root.path.as_os_str()
                    != right.installation.root.path.as_os_str()
            {
                continue;
            }
            for left_artifact in &left.artifacts {
                for right_artifact in &right.artifacts {
                    let (DesiredArtifact::File(left_file), DesiredArtifact::File(right_file)) =
                        (left_artifact, right_artifact)
                    else {
                        unreachable!("rule producer returns only files")
                    };
                    let left_path = &left_file.location.path;
                    let right_path = &right_file.location.path;
                    if left_path == right_path
                        || descendant(left_path, right_path)
                        || descendant(right_path, left_path)
                    {
                        diagnostics.push(diagnostic(
                            requests,
                            vec![*left_index, *right_index],
                            format!("selected output paths conflict: {left_path} and {right_path}"),
                        ));
                    }
                }
            }
        }
    }
    diagnostics.sort_by(|left, right| {
        left.request_indices
            .cmp(&right.request_indices)
            .then_with(|| left.message.cmp(&right.message))
    });
    if diagnostics.is_empty() {
        DesiredSetsOutcome::Ready(successful.into_iter().map(|(_, set)| set).collect())
    } else {
        DesiredSetsOutcome::Blocked(diagnostics)
    }
}

fn descendant(ancestor: &str, candidate: &str) -> bool {
    candidate
        .strip_prefix(ancestor)
        .is_some_and(|suffix| suffix.starts_with('/'))
}

fn diagnostic(
    requests: &[RuleDesiredSetRequest<'_>],
    request_indices: Vec<usize>,
    message: String,
) -> DesiredSetDiagnostic {
    let installation_ids = request_indices
        .iter()
        .map(|index| requests[*index].installation.owner.installation_id.clone())
        .collect();
    DesiredSetDiagnostic {
        request_indices,
        installation_ids,
        message,
    }
}
