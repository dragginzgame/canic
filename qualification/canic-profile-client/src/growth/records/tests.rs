use super::*;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};

#[test]
fn immutable_approvals_use_private_parents_and_refuse_aliases() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("target/profile-record-tests");
    let (path, held) = attempt(&root).unwrap();
    write_in(&held, "approval.bin".as_ref(), b"exact frame").unwrap();
    assert!(write_in(&held, "approval.bin".as_ref(), b"replacement").is_err());
    assert_eq!(fs::read(path.join("approval.bin")).unwrap(), b"exact frame");
    assert_eq!(
        fs::metadata(path.join("approval.bin"))
            .unwrap()
            .permissions()
            .mode()
            & 0o077,
        0
    );
    symlink("approval.bin", path.join("alias")).unwrap();
    assert!(write(&path.join("alias"), b"replacement").is_err());
    assert!(write(&path.join("missing/../escape"), b"bad").is_err());
    assert!(!path.join("missing").exists());
    fs::remove_dir_all(path).unwrap();
}
