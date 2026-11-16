use crate::store;
use std::fs;
use std::os::unix::fs::PermissionsExt;

#[test]
fn oversize_refuses_before_parent_creation() {
    let root = crate::tests::directory();
    let path = root.join("missing/record");
    assert!(store::put(&path, &vec![0; store::MAX_RECORD + 1]).is_err());
    assert!(!root.join("missing").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn record_creation_preserves_parent_mode_and_never_replaces_evidence() {
    let root = crate::tests::directory();
    fs::create_dir(root.join("mode-control")).unwrap();
    store::put(&root.join("new/deep/record"), b"signed evidence").unwrap();
    let control = fs::metadata(root.join("mode-control"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    for name in ["new", "new/deep"] {
        assert_eq!(
            fs::metadata(root.join(name)).unwrap().permissions().mode() & 0o777,
            control
        );
    }
    assert!(store::put(&root.join("new/deep/record"), b"replacement").is_err());
    assert_eq!(
        fs::read(root.join("new/deep/record")).unwrap(),
        b"signed evidence"
    );
    assert_eq!(fs::read_dir(root.join("new/deep")).unwrap().count(), 1);
    fs::remove_dir_all(root).unwrap();
}
