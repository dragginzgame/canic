//! Parent-only corrections, interrupted seeding and unchanged resolved graphs.

use super::*;
use std::fs;

#[test]
fn cargo_resolves_a_refreshed_parent_seed_into_the_generated_workspace() {
    use crate::cargo_metadata::{CargoFeatureSelection, cargo_metadata_catalog_for_manifest};
    let root = crate::test_support::temp_dir("generated-lock-cargo");
    let generated = root.join("generated");
    let shared = root.join("shared");
    for directory in [&root, &generated, &shared] {
        fs::create_dir_all(directory.join("src")).unwrap();
        fs::write(directory.join("src/lib.rs"), "").unwrap();
    }
    fs::write(root.join("Cargo.toml"), "[workspace]\n[package]\nname='parent'\nversion='0.1.0'\n[dependencies]\nshared={path='shared'}\n").unwrap();
    let manifest = b"[workspace]\n[package]\nname='wrapper'\nversion='0.1.0'\n[dependencies]\nshared={path='../shared'}\n";
    fs::write(generated.join("Cargo.toml"), manifest).unwrap();
    for version in ["0.1.0", "0.2.0"] {
        fs::write(
            shared.join("Cargo.toml"),
            format!("[package]\nname='shared'\nversion='{version}'\n"),
        )
        .unwrap();
        cargo_metadata_catalog_for_manifest(
            &root.join("Cargo.toml"),
            false,
            true,
            &CargoFeatureSelection::default(),
        )
        .unwrap();
        refresh_seed(&generated, &root.join("Cargo.lock"), manifest).unwrap();
        let resolved = cargo_metadata_catalog_for_manifest(
            &generated.join("Cargo.toml"),
            false,
            true,
            &CargoFeatureSelection::default(),
        )
        .unwrap();
        assert_eq!(
            resolved
                .packages
                .iter()
                .find(|package| package.name == "shared")
                .unwrap()
                .version,
            version
        );
        let bytes = fs::read(generated.join("Cargo.lock")).unwrap();
        refresh_seed(&generated, &root.join("Cargo.lock"), manifest).unwrap();
        assert_eq!(fs::read(generated.join("Cargo.lock")).unwrap(), bytes);
        cargo_metadata_catalog_for_manifest(
            &generated.join("Cargo.toml"),
            true,
            true,
            &CargoFeatureSelection::default(),
        )
        .unwrap();
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn parent_resolution_changes_refresh_once_without_overwriting_unchanged_cargo_resolution() {
    let root = crate::test_support::temp_dir("generated-lock-parent");
    fs::create_dir_all(root.join("generated")).unwrap();
    let directory = root.join("generated");
    let parent = root.join("Cargo.lock");
    let lock = directory.join("Cargo.lock");
    let original = b"version = 4\n[[package]]\nname = 'dependency'\nversion = '0.1.0'\n";
    let updated = b"version = 4\n[[package]]\nname = 'dependency'\nversion = '0.2.0'\n";
    fs::write(&parent, original).unwrap();
    refresh_seed(&directory, &parent, b"manifest").unwrap();
    assert_eq!(fs::read(&lock).unwrap(), original);
    let expected_record = format!(
        "{{\n  \"schema_version\": 1,\n  \"parent_lock_sha256\": \"{}\",\n  \"manifest_sha256\": \"{}\"\n}}",
        sha256_hex(original),
        sha256_hex(b"manifest"),
    );
    assert_eq!(
        fs::read(directory.join("lock-seed.json")).unwrap(),
        expected_record.as_bytes()
    );
    let resolved = b"Cargo's resolved wrapper graph";
    fs::write(&lock, resolved).unwrap();
    let modified = fs::metadata(&lock).unwrap().modified().unwrap();
    refresh_seed(&directory, &parent, b"manifest").unwrap();
    assert_eq!(fs::read(&lock).unwrap(), resolved);
    assert_eq!(fs::metadata(&lock).unwrap().modified().unwrap(), modified);
    fs::write(&parent, updated).unwrap();
    refresh_seed(&directory, &parent, b"manifest").unwrap();
    assert_eq!(fs::read(&lock).unwrap(), updated);
    fs::write(&lock, resolved).unwrap();
    refresh_seed(&directory, &parent, b"new feature selection").unwrap();
    assert_eq!(fs::read(&lock).unwrap(), updated);
    // Reproduce interruption after atomic lock publication but before its derivation commits.
    fs::remove_file(directory.join("lock-seed.json")).unwrap();
    fs::write(&lock, original).unwrap();
    refresh_seed(&directory, &parent, b"new feature selection").unwrap();
    assert_eq!(fs::read(&lock).unwrap(), updated);
    fs::remove_file(&lock).unwrap();
    refresh_seed(&directory, &parent, b"new feature selection").unwrap();
    assert_eq!(fs::read(&lock).unwrap(), updated);
    fs::remove_dir_all(root).unwrap();
}
