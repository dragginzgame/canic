use std::{fs, path::Path};

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate directory should have a parent")
        .parent()
        .expect("workspace root should exist")
}

#[test]
fn pending_changelog_version_has_matching_detailed_notes() {
    let root = workspace_root();
    let root_changelog = root.join("CHANGELOG.md");
    let root_source =
        fs::read_to_string(&root_changelog).expect("root changelog should be readable");

    let candidates = root_source
        .lines()
        .filter_map(|line| line.strip_prefix("## [")?.strip_suffix(']'))
        .filter(|version| {
            let components = version.split('.').collect::<Vec<_>>();
            components.len() == 3
                && components
                    .iter()
                    .all(|component| component.parse::<u64>().is_ok())
        })
        .collect::<Vec<_>>();
    let [version] = candidates.as_slice() else {
        panic!("root changelog must select one numbered pending release: {candidates:?}");
    };
    let minor = version.rsplit_once('.').expect("release minor line").0;
    let detailed = root.join(format!("docs/changelog/{minor}.md"));
    let source = fs::read_to_string(detailed).expect("selected detailed notes must exist");
    assert!(source.lines().any(|line| line == format!("## [{version}]")));
}
