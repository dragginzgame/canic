use super::Store;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};

#[test]
fn private_parent_creation_preserves_existing_modes() {
    let root = crate::tests::directory();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
    let nested = root.join("new/deep");
    let held = Store::open(&nested).unwrap();
    assert_eq!(
        fs::metadata(&root).unwrap().permissions().mode() & 0o777,
        0o755
    );
    for name in ["new", "new/deep"] {
        assert_eq!(
            fs::metadata(root.join(name)).unwrap().permissions().mode() & 0o077,
            0
        );
    }
    assert!(nested.join("lock").is_file());
    drop(held);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn private_parent_refusals_precede_creation() {
    let root = crate::tests::directory();
    assert!(Store::open(&root.join("not-created/../escape")).is_err());
    assert!(!root.join("not-created").exists());
    assert!(!root.join("escape").exists());
    let foreign = root.join("foreign");
    fs::create_dir(&foreign).unwrap();
    symlink(&foreign, root.join("alias")).unwrap();
    assert!(Store::open(&root.join("alias/new")).is_err());
    assert!(!foreign.join("new").exists());
    assert_eq!(fs::read_link(root.join("alias")).unwrap(), foreign);
    fs::remove_dir_all(root).unwrap();
}
