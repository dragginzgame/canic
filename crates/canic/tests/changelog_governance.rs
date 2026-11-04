//! Each explicit release group owns one unambiguous ledger.

use std::{fs, path::Path, process::Command};

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate directory should have a parent")
        .parent()
        .expect("workspace root should exist")
}

#[derive(Debug, Eq, PartialEq)]
enum ReleaseSelectionError {
    MissingRelease,
    MultiplePending,
    PendingNotLatest,
}

fn latest_release(source: &str) -> Result<&str, ReleaseSelectionError> {
    let candidates = source
        .lines()
        .filter_map(|line| {
            let (version, suffix) = line.strip_prefix("## [")?.split_once(']')?;
            let components = version.split('.').collect::<Vec<_>>();
            let numbered = components.len() == 3
                && components
                    .iter()
                    .all(|component| component.parse::<u64>().is_ok());
            let pending = suffix.is_empty() || suffix == " - Unreleased";
            (numbered && (pending || suffix.starts_with(" - "))).then_some((version, pending))
        })
        .collect::<Vec<_>>();
    let Some((version, is_pending)) = candidates.first() else {
        return Err(ReleaseSelectionError::MissingRelease);
    };
    let pending_count = candidates.iter().filter(|(_, pending)| *pending).count();
    if pending_count > 1 {
        return Err(ReleaseSelectionError::MultiplePending);
    }
    if pending_count == 1 && !is_pending {
        return Err(ReleaseSelectionError::PendingNotLatest);
    }
    Ok(version)
}

#[test]
fn each_group_has_one_canonical_unambiguous_ledger() {
    let root = workspace_root();
    let output = Command::new(root.join(".tools/host/bin/jq"))
        .args(["-er", ".groups[].changelog"])
        .arg(root.join("ci/cargo-release-groups.json"))
        .output()
        .expect("read the admitted group policy");
    assert!(output.status.success());
    let ledgers = String::from_utf8(output.stdout).expect("group ledger paths");
    assert!(!ledgers.trim().is_empty());
    let index = fs::read_to_string(root.join("CHANGELOG.md")).expect("changelog index");
    for path in ledgers.lines() {
        let source = fs::read_to_string(root.join(path)).expect("group ledger");
        latest_release(&source).expect("one latest numbered release per group");
        assert!(
            index.contains(path),
            "group ledger missing from index: {path}"
        );
    }
}

#[test]
fn release_selection_accepts_finalization_and_refuses_ambiguous_drafts() {
    for (source, expected) in [
        ("## [1.2.3]\n", Ok("1.2.3")),
        ("## [1.2.3] - Unreleased\n", Ok("1.2.3")),
        ("## [1.2.3] - 2026-10-31\n", Ok("1.2.3")),
        ("## [1.2.3]\n## [1.2.2] - 2026-10-30\n", Ok("1.2.3")),
        (
            "## [1.2.3]\n## [1.2.2] - Unreleased\n",
            Err(ReleaseSelectionError::MultiplePending),
        ),
        (
            "## [1.2.3] - 2026-10-31\n## [1.2.2]\n",
            Err(ReleaseSelectionError::PendingNotLatest),
        ),
        ("# Changelog\n", Err(ReleaseSelectionError::MissingRelease)),
    ] {
        assert_eq!(latest_release(source), expected);
    }
}
