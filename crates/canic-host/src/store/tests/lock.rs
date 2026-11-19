use crate::store::Store;
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt, symlink},
};

#[test]
fn sidecar_preserves_inode_bytes_modes_and_legacy_contention() {
    let root = crate::tests::directory();
    let path = root.join("lock");
    fs::write(&path, b"existing sidecar").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let before = fs::metadata(&path).unwrap();
    fs::hard_link(&path, root.join("alias")).unwrap();
    let external = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    external.try_lock().unwrap();
    assert!(Store::open(&root).is_err());
    drop(external);
    let owner = Store::open(&root).unwrap();
    let external = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    assert!(external.try_lock().is_err());
    drop(owner);
    external.try_lock().unwrap();
    drop(external);
    let after = fs::metadata(&path).unwrap();
    assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
    assert_eq!(after.permissions().mode() & 0o777, 0o644);
    assert_eq!(fs::read(&path).unwrap(), b"existing sidecar");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unsafe_sidecars_refuse_without_replacing_foreign_entries() {
    let root = crate::tests::directory();
    let path = root.join("lock");
    let foreign = root.join("foreign");
    fs::write(&foreign, b"keep").unwrap();
    symlink(&foreign, &path).unwrap();
    assert!(Store::open(&root).is_err());
    assert_eq!(fs::read_link(&path).unwrap(), foreign);
    assert_eq!(fs::read(&foreign).unwrap(), b"keep");
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(Store::open(&root).is_err());
    assert!(path.is_dir());
    fs::remove_dir_all(root).unwrap();
}
