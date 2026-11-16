use super::read;
use crate::{Error, tests::directory};
use std::{error::Error as _, os::unix::fs::symlink};

#[test]
fn exact_bytes_limits_and_original_missing_cause() {
    let root = directory();
    let path = root.join("signed.json");
    std::fs::write(&path, b"retained signed bytes").unwrap();
    assert_eq!(read(&path, 21).unwrap(), b"retained signed bytes");
    assert_eq!(read(&path, 100).unwrap(), b"retained signed bytes");
    assert!(read(&path, 20).is_err());
    assert!(read(&path, 0).is_err());
    std::fs::write(&path, b"").unwrap();
    assert_eq!(read(&path, 0).unwrap(), b"");
    let Error::Supplier(cause) = read(&root.join("missing"), 100).unwrap_err() else {
        panic!("missing file must preserve supplier cause");
    };
    let mut current: &(dyn std::error::Error + 'static) = cause.as_ref();
    let original = loop {
        if let Some(original) = current.downcast_ref::<std::io::Error>() {
            break original;
        }
        current = current
            .source()
            .expect("original I/O cause must be retained");
    };
    assert_eq!(original.kind(), std::io::ErrorKind::NotFound);
    assert!(read(&root, 100).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn final_links_refuse_and_ancestor_links_remain_supported() {
    let root = directory();
    let actual = root.join("actual");
    std::fs::create_dir(&actual).unwrap();
    std::fs::write(actual.join("record"), b"exact").unwrap();
    symlink(&actual, root.join("ancestor")).unwrap();
    assert_eq!(read(&root.join("ancestor/record"), 5).unwrap(), b"exact");
    symlink(actual.join("record"), root.join("link")).unwrap();
    let error = read(&root.join("link"), 100).unwrap_err();
    assert!(error.source().is_some());
    assert_eq!(std::fs::read(actual.join("record")).unwrap(), b"exact");
    assert!(read(&actual.join("record/"), 100).is_err());
    std::fs::remove_dir_all(root).unwrap();
}
