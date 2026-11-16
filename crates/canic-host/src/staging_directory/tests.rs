use super::*;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};

#[test]
fn collisions_preserve_foreign_entries_and_stop_at_budget() {
    let root = crate::tests::directory();
    let parent = Directory::open(&root, 0).unwrap().unwrap();
    let candidate = |n| root.join(format!(".probe-{}-{n}", std::process::id()));
    fs::write(candidate(0), b"foreign").unwrap();
    fs::create_dir(candidate(1)).unwrap();
    symlink("missing", candidate(2)).unwrap();
    let sequence = AtomicU64::new(0);
    let (name, held) = allocate_with(&parent, ".probe", &sequence).unwrap();
    assert_eq!(root.join(&name), candidate(3));
    assert_eq!(
        fs::metadata(candidate(3)).unwrap().permissions().mode() & 0o077,
        0
    );
    assert_eq!(
        parent
            .open_directory(&name, 0)
            .unwrap()
            .unwrap()
            .identity()
            .unwrap(),
        held.identity().unwrap()
    );
    for n in 4..32 {
        fs::write(candidate(n), b"foreign").unwrap();
    }
    assert!(allocate_with(&parent, ".probe", &AtomicU64::new(0)).is_err());
    assert!(!candidate(32).exists());
    assert_eq!(fs::read(candidate(0)).unwrap(), b"foreign");
    assert!(candidate(1).is_dir());
    assert_eq!(
        fs::read_link(candidate(2)).unwrap(),
        std::path::Path::new("missing")
    );
    assert_eq!(fs::read_dir(&root).unwrap().count(), 32);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn held_allocation_survives_parent_rename_and_rejects_substitution() {
    let root = crate::tests::directory();
    let path = root.join("parent");
    fs::create_dir(&path).unwrap();
    let parent = Directory::open(&path, 0).unwrap().unwrap();
    fs::rename(&path, root.join("moved")).unwrap();
    fs::create_dir(&path).unwrap();
    let (name, held) = allocate_with(&parent, ".probe", &AtomicU64::new(0)).unwrap();
    assert!(!path.join(&name).exists());
    let moved = root.join("moved");
    fs::rename(moved.join(&name), moved.join("retained")).unwrap();
    fs::create_dir(moved.join(&name)).unwrap();
    fs::write(moved.join(&name).join("foreign"), b"untouched").unwrap();
    assert!(
        parent
            .publish_directory(&name, &held, "published".as_ref())
            .is_err()
    );
    assert!(!moved.join("published").exists());
    assert!(moved.join("retained").is_dir());
    assert_eq!(
        fs::read(moved.join(&name).join("foreign")).unwrap(),
        b"untouched"
    );
    fs::remove_dir_all(root).unwrap();
}
