//! Pure conservative path topology, also available without a qualified OS backend.
fn may_match(a: &str, b: &str) -> bool {
    !a.is_ascii() || !b.is_ascii() || a.eq_ignore_ascii_case(b)
}
pub(crate) fn may_overlap(a: &str, b: &str) -> bool {
    a.split('/').zip(b.split('/')).all(|(a, b)| may_match(a, b))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn potential_overlap_examples() {
        assert!(may_overlap("a/X", "a/x"));
        assert!(may_overlap("a/é", "a/e"));
        assert!(may_overlap("a/x", "a/x/y"));
        assert!(!may_overlap("a/é", "b/e"));
        assert!(!may_overlap("a/b", "ab/c"));
    }
}
