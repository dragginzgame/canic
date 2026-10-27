//! Required source inventory for architecture guards; failures cannot mean clean.

use std::{fs, path::Path};

/// Inspect every admitted Rust source beneath one required, nonempty root.
pub fn visit(
    root: &Path,
    excluded: &impl Fn(&Path) -> bool,
    visitor: &mut impl FnMut(&Path, &str),
) {
    let inspected = walk(root, excluded, visitor);
    assert!(
        inspected > 0,
        "no Rust sources inspected under {}",
        root.display()
    );
}

fn walk(
    root: &Path,
    excluded: &impl Fn(&Path) -> bool,
    visitor: &mut impl FnMut(&Path, &str),
) -> usize {
    let entries = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("inspect source directory {}: {error}", root.display()));
    let mut inspected = 0;
    for entry in entries {
        let entry = entry
            .unwrap_or_else(|error| panic!("read source entry under {}: {error}", root.display()));
        let path = entry.path();
        if excluded(&path) {
            continue;
        }
        let metadata = fs::metadata(&path)
            .unwrap_or_else(|error| panic!("inspect source metadata {}: {error}", path.display()));
        if metadata.is_dir() {
            inspected += walk(&path, excluded, visitor);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read Rust source {}: {error}", path.display()));
            visitor(&path, &source);
            inspected += 1;
        }
    }
    inspected
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "canic-required-source-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).expect("create private source fixture");
            Self(path)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("remove owned source fixture");
        }
    }

    #[test]
    fn visits_nested_sources_and_preserves_explicit_exclusions() {
        let fixture = Fixture::new();
        fs::create_dir(fixture.0.join("nested")).unwrap();
        fs::write(fixture.0.join("nested/source.rs"), "fn source() {}").unwrap();
        fs::write(fixture.0.join("excluded.rs"), [255]).unwrap();
        let mut paths = Vec::new();
        visit(
            &fixture.0,
            &|path| path.file_name().is_some_and(|name| name == "excluded.rs"),
            &mut |path, _| paths.push(path.to_owned()),
        );
        assert_eq!(paths, vec![fixture.0.join("nested/source.rs")]);
    }

    #[test]
    #[should_panic(expected = "inspect source directory")]
    fn missing_required_root_refuses() {
        let fixture = Fixture::new();
        visit(&fixture.0.join("missing"), &|_| false, &mut |_, _| {});
    }

    #[test]
    #[should_panic(expected = "no Rust sources inspected")]
    fn empty_required_root_refuses() {
        let fixture = Fixture::new();
        visit(&fixture.0, &|_| false, &mut |_, _| {});
    }

    #[test]
    #[should_panic(expected = "read Rust source")]
    fn undecodable_required_source_refuses() {
        let fixture = Fixture::new();
        fs::write(fixture.0.join("source.rs"), [255]).unwrap();
        visit(&fixture.0, &|_| false, &mut |_, _| {});
    }

    #[cfg(unix)]
    #[test]
    #[should_panic(expected = "inspect source metadata")]
    fn broken_entry_metadata_refuses() {
        let fixture = Fixture::new();
        std::os::unix::fs::symlink("absent", fixture.0.join("source.rs")).unwrap();
        visit(&fixture.0, &|_| false, &mut |_, _| {});
    }
}
