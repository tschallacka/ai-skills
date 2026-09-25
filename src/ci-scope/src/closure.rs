// MODE: DEV
// PACKAGE: PROD
//! The transitive reverse-dependency closure: start with the changed crates,
//! repeatedly add any crate that depends on something already selected,
//! until a full pass adds nothing new. Order-independent (a least fixed
//! point), so batching every pass's additions together produces the
//! identical result.

use std::collections::BTreeSet;

/// `edges` are `(dependent, dependency)` pairs: `dependent` depends on
/// `dependency`, so a change to `dependency` must also rebuild `dependent`.
pub fn reverse_closure(changed: &BTreeSet<String>, edges: &[(String, String)]) -> BTreeSet<String> {
    let mut selected: BTreeSet<String> = changed.clone();
    loop {
        let mut added = Vec::new();
        for (dependent, dependency) in edges {
            if selected.contains(dependency) && !selected.contains(dependent) {
                added.push(dependent.clone());
            }
        }
        if added.is_empty() {
            break;
        }
        selected.extend(added);
    }
    selected
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn edges(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    #[test]
    fn a_crate_with_no_dependents_closes_on_itself() {
        let changed = set(&["leaf"]);
        let out = reverse_closure(&changed, &edges(&[]));
        assert_eq!(out, set(&["leaf"]));
    }

    #[test]
    fn a_direct_dependent_is_added() {
        let changed = set(&["lib"]);
        let out = reverse_closure(&changed, &edges(&[("app", "lib")]));
        assert_eq!(out, set(&["app", "lib"]));
    }

    #[test]
    fn a_multi_level_transitive_dependent_is_reached() {
        // c depends on b, b depends on a; changing a must pull in b and c.
        let changed = set(&["a"]);
        let out = reverse_closure(&changed, &edges(&[("b", "a"), ("c", "b")]));
        assert_eq!(out, set(&["a", "b", "c"]));
    }

    #[test]
    fn an_unrelated_edge_does_not_pollute_the_closure() {
        let changed = set(&["a"]);
        let out = reverse_closure(&changed, &edges(&[("x", "y")]));
        assert_eq!(out, set(&["a"]));
    }

    #[test]
    fn multiple_changed_crates_each_pull_their_own_dependents() {
        let changed = set(&["a", "z"]);
        let out = reverse_closure(&changed, &edges(&[("b", "a"), ("y", "z")]));
        assert_eq!(out, set(&["a", "b", "y", "z"]));
    }

    #[test]
    fn a_cycle_does_not_loop_forever() {
        let changed = set(&["a"]);
        let out = reverse_closure(&changed, &edges(&[("b", "a"), ("a", "b")]));
        assert_eq!(out, set(&["a", "b"]));
    }
}
