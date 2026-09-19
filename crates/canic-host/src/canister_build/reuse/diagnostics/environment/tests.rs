use super::*;
use crate::test_support::temp_dir;
use sha2::Digest as _;
use std::fs;

fn inputs(value: &str) -> Vec<(OsString, OsString)> {
    vec![("CANIC_TEST_BUILD_INPUT".into(), value.into())]
}

#[test]
fn keyed_comparison_names_changes_without_retaining_values_or_unkeyed_hashes() {
    let mut key = [0; 32];
    getrandom::fill(&mut key).unwrap();
    let value = "synthetic-comparison-value";
    let baseline = EnvironmentComparison::with_key(&key, &inputs(value)).unwrap();
    let repeat = EnvironmentComparison::with_key(&key, &inputs(value)).unwrap();
    assert_eq!(repeat.changed_keys(&baseline), Some(vec![]));
    let changed = EnvironmentComparison::with_key(&key, &inputs("another-setting")).unwrap();
    assert_eq!(
        changed.changed_keys(&baseline),
        Some(vec!["CANIC_TEST_BUILD_INPUT".into()])
    );
    let bytes = serde_json::to_string(&baseline).unwrap();
    assert!(!bytes.contains(value));
    let raw_hash: [u8; 32] = Sha256::digest(value.as_bytes()).into();
    assert_ne!(baseline.tags["CANIC_TEST_BUILD_INPUT"], raw_hash);
    assert!(!bytes.contains(&serde_json::to_string(&raw_hash).unwrap()));
    assert!(!bytes.contains(&serde_json::to_string(&key).unwrap()));
    key[0] ^= 1;
    let rotated = EnvironmentComparison::with_key(&key, &inputs(value)).unwrap();
    assert!(rotated.changed_keys(&baseline).is_none());
}

#[test]
fn comparison_is_order_independent_bounded_and_names_only_safe_keys() {
    let mut key = [0; 32];
    getrandom::fill(&mut key).unwrap();
    let mut values = vec![
        ("SAFE".into(), "one".into()),
        ("BAD\nKEY".into(), "two".into()),
    ];
    let first = EnvironmentComparison::with_key(&key, &values).unwrap();
    values.reverse();
    let repeat = EnvironmentComparison::with_key(&key, &values).unwrap();
    assert_eq!(first.tags, repeat.tags);
    assert_eq!(first.tags.len(), 1);
    let baseline: Vec<(OsString, OsString)> = (0..20)
        .map(|i| (format!("SAFE_{i}").into(), "old".into()))
        .collect::<Vec<_>>();
    let changed = baseline
        .iter()
        .map(|(name, _)| (name.clone(), "new".into()))
        .collect::<Vec<_>>();
    assert_eq!(
        EnvironmentComparison::with_key(&key, &changed)
            .unwrap()
            .changed_keys(&EnvironmentComparison::with_key(&key, &baseline).unwrap())
            .unwrap()
            .len(),
        8
    );
    assert!(
        EnvironmentComparison::with_key(&key, &vec![("SAFE".into(), "value".into()); MAX_KEYS + 1])
            .is_none()
    );
}

#[test]
#[cfg(unix)]
fn missing_corrupt_shared_and_linked_keys_disable_comparison_without_replacement() {
    use std::os::unix::fs::{PermissionsExt as _, symlink};
    let root = temp_dir("environment-comparison-key");
    let path = root.join(KEY_PATH);
    let values = inputs("synthetic-value");
    assert!(EnvironmentComparison::capture(&root, &values).is_none());
    prepare_key(&root);
    let original = fs::read(&path).unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let baseline = EnvironmentComparison::capture(&root, &values).unwrap();
    prepare_key(&root);
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(
        EnvironmentComparison::capture(&root, &values)
            .unwrap()
            .changed_keys(&baseline),
        Some(vec![])
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    prepare_key(&root);
    assert!(EnvironmentComparison::capture(&root, &values).is_none());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&path, b"truncated").unwrap();
    prepare_key(&root);
    assert!(EnvironmentComparison::capture(&root, &values).is_none());
    assert_eq!(fs::read(&path).unwrap(), b"truncated");
    fs::remove_file(&path).unwrap();
    let target = root.join("private-target");
    create_private_bytes_with_parents(&target, &original).unwrap();
    symlink(&target, &path).unwrap();
    prepare_key(&root);
    assert!(EnvironmentComparison::capture(&root, &values).is_none());
    assert!(
        fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    fs::remove_file(&path).unwrap();
    fs::hard_link(&target, &path).unwrap();
    assert!(EnvironmentComparison::capture(&root, &values).is_none());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn keyed_environment_tag_matches_independent_sha256_reference() {
    // Python standard-library HMAC-SHA256 over the domain and length-prefixed fields.
    let expected: [u8; 32] = [
        47, 77, 213, 184, 240, 88, 48, 90, 187, 112, 113, 56, 64, 24, 7, 244, 9, 245, 144, 241,
        182, 0, 204, 124, 17, 170, 78, 205, 26, 240, 20, 132,
    ];
    assert_eq!(
        tag(
            &[0x0b; 32],
            b"environment-value",
            &[b"CANIC_TEST_BUILD_INPUT", b"synthetic-setting"]
        ),
        expected
    );
    assert_ne!(tag(&[0x0b; 32], b"environment-key-id", &[]), expected);
}
