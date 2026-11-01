//! Keep the latest declared release paired with its detailed notes, before and after dating.

use std::{fs, path::Path};

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

fn detailed_notes_match(source: &str, version: &str) -> bool {
    let heading = format!("## [{version}]");
    let dated_heading = format!("{heading} - ");
    source
        .lines()
        .any(|line| line == heading || line.starts_with(&dated_heading))
}

#[test]
fn latest_changelog_release_has_matching_detailed_notes() {
    let root = workspace_root();
    let root_source =
        fs::read_to_string(root.join("CHANGELOG.md")).expect("root changelog should be readable");
    let version = latest_release(&root_source).expect("one latest numbered release");
    let minor = version.rsplit_once('.').expect("release minor line").0;
    let detailed = root.join(format!("docs/changelog/{minor}.md"));
    let source = fs::read_to_string(detailed).expect("selected detailed notes must exist");
    assert!(detailed_notes_match(&source, version));
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

#[test]
fn matching_notes_cover_pending_and_released_headings() {
    for source in [
        "## [1.2.3]\n",
        "## [1.2.3] - Unreleased\n",
        "## [1.2.3] - 2026-10-31\n",
    ] {
        assert!(detailed_notes_match(source, "1.2.3"));
        assert!(!detailed_notes_match(source, "1.2.2"));
    }
}
