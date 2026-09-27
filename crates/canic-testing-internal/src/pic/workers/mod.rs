//! Module: pic::workers
//!
//! Responsibility: select exact registered cases for two isolated test processes.
//! Boundary: the shell launcher owns process groups, private scratch and server cleanup.

use super::GovernedTestCase;
use std::{collections::BTreeSet, path::PathBuf, process::Command};

pub(super) const CASE_FILE_ENV: &str = "CANIC_GOVERNED_CASE_FILE";

#[derive(Debug, Eq, PartialEq)]
pub(super) enum SelectionError {
    Empty,
    Duplicate,
    Unknown,
}

pub(super) fn select(
    cases: &[GovernedTestCase],
    names: &str,
) -> Result<Vec<GovernedTestCase>, SelectionError> {
    let mut seen = BTreeSet::new();
    let mut selected = Vec::new();
    for name in names.lines() {
        if !seen.insert(name) {
            return Err(SelectionError::Duplicate);
        }
        let case = cases
            .iter()
            .find(|(registered, _)| *registered == name)
            .ok_or(SelectionError::Unknown)?;
        selected.push(*case);
    }
    if selected.is_empty() {
        return Err(SelectionError::Empty);
    }
    Ok(selected)
}

pub(super) fn run(groups: [Vec<GovernedTestCase>; 2]) {
    let scratch = PathBuf::from(std::env::var_os("CANIC_TEST_SCRATCH").expect("governed scratch"));
    let mut paths = Vec::new();
    let mut names = BTreeSet::new();
    for (index, cases) in groups.into_iter().enumerate() {
        assert!(!cases.is_empty());
        let mut selection = String::new();
        for (name, _) in cases {
            assert!(names.insert(name), "case cannot belong to both workers");
            assert!(!name.contains('\n'));
            selection.push_str(name);
            selection.push('\n');
        }
        let path = scratch.join(format!("governed-worker-{index}.cases"));
        std::fs::write(&path, selection).expect("write exact worker selection");
        paths.push(path);
    }
    let script =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/ci/run-pocketic-workers.sh");
    let status = Command::new("bash")
        .arg(script)
        .arg(std::env::current_exe().expect("compiled test executable"))
        .args(paths)
        .status()
        .expect("start isolated PocketIC workers");
    assert!(
        status.success(),
        "isolated PocketIC worker failed: {status}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_preserves_exact_order_and_rejects_invalid_membership() {
        fn noop() {}
        let cases: Vec<GovernedTestCase> = vec![("first", noop), ("second", noop)];
        let selected = select(&cases, "second\nfirst\n").unwrap();
        assert_eq!(
            selected.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
            ["second", "first"]
        );
        for (input, error) in [
            ("", SelectionError::Empty),
            ("first\nfirst", SelectionError::Duplicate),
            ("missing", SelectionError::Unknown),
        ] {
            assert_eq!(select(&cases, input).unwrap_err(), error);
        }
    }
}
