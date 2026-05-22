use std::{fs, path::Path};

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate directory should have a parent")
        .parent()
        .expect("workspace root should exist")
}

#[test]
fn unreleased_section_exists_only_in_root_changelog() {
    let root = workspace_root();
    let root_changelog = root.join("CHANGELOG.md");
    let root_source =
        fs::read_to_string(&root_changelog).expect("root changelog should be readable");

    assert!(
        root_source.contains("\n## Unreleased\n"),
        "root CHANGELOG.md must contain the only Unreleased section"
    );

    let changelog_dir = root.join("docs/changelog");
    let mut failures = Vec::new();
    for entry in fs::read_dir(&changelog_dir).expect("docs/changelog should be readable") {
        let path = entry.expect("changelog entry should be readable").path();
        if path.extension().is_none_or(|extension| extension != "md") {
            continue;
        }
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        if source.contains("\n## Unreleased\n") {
            failures.push(
                path.strip_prefix(root)
                    .unwrap_or(&path)
                    .display()
                    .to_string(),
            );
        }
    }

    assert!(
        failures.is_empty(),
        "detailed changelogs must not contain Unreleased sections:\n{}",
        failures.join("\n")
    );
}
