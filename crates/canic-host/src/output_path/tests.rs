use std::{
    fs,
    os::unix::{ffi::OsStrExt, fs::symlink},
    path::{Path, PathBuf},
};

use super::resolve;

#[test]
fn missing_outputs_and_dangling_aliases_share_physical_identity() {
    let root = crate::tests::directory().canonicalize().unwrap();
    fs::create_dir(root.join("real")).unwrap();
    symlink("real/missing", root.join("relative")).unwrap();
    symlink(root.join("real/missing"), root.join("absolute")).unwrap();
    symlink("relative", root.join("chain")).unwrap();
    symlink("absent/../chain", root.join("rewind")).unwrap();
    for name in [
        "relative",
        "absolute",
        "chain",
        "rewind",
        "chain/child/..",
        "absent/../relative",
    ] {
        assert_eq!(
            resolve(Path::new(name), &root).unwrap(),
            root.join("real/missing")
        );
    }
    let native = std::ffi::OsStr::from_bytes(b"nonutf8-\xff");
    assert_eq!(
        resolve(Path::new(native), &root).unwrap(),
        root.join(native)
    );
    assert_eq!(fs::read_dir(&root).unwrap().count(), 5);
    assert!(!root.join("real/missing").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_ancestors_and_cycles_refuse_without_effects() {
    let root = crate::tests::directory().canonicalize().unwrap();
    fs::write(root.join("file"), b"keep").unwrap();
    for (index, name) in [
        "file/child",
        "file/.",
        "file/",
        "file/..",
        "missing/../file/child",
        "missing/../file/.",
        "missing/../file/..",
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(
            resolve(Path::new(name), &root).unwrap_err().kind(),
            std::io::ErrorKind::NotADirectory,
            "{name}"
        );
        let alias = root.join(format!("invalid-{index}"));
        symlink(name, &alias).unwrap();
        assert_eq!(
            resolve(&alias, &root).unwrap_err().kind(),
            std::io::ErrorKind::NotADirectory,
            "alias to {name}"
        );
    }
    symlink("b", root.join("a")).unwrap();
    symlink("a", root.join("b")).unwrap();
    assert!(resolve(Path::new("a"), &root).is_err());
    symlink("missing/../rewind-loop", root.join("rewind-loop")).unwrap();
    assert!(resolve(Path::new("rewind-loop"), &root).is_err());
    assert!(resolve(Path::new("x"), Path::new("relative")).is_err());
    assert!(!root.join("missing").exists());
    assert_eq!(fs::read(root.join("file")).unwrap(), b"keep");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn dangling_depth_budget_counts_only_active_expansions() {
    let root = crate::tests::directory().canonicalize().unwrap();
    for index in 0..41 {
        let target = if index == 40 {
            "missing/output".to_owned()
        } else {
            format!("missing/../link-{}", index + 1)
        };
        symlink(target, root.join(format!("link-{index}"))).unwrap();
    }
    assert!(resolve(Path::new("link-0"), &root).is_err());
    assert_eq!(
        resolve(Path::new("link-1"), &root).unwrap(),
        root.join("missing/output")
    );
    symlink("missing/..", root.join("back")).unwrap();
    let independent = std::iter::repeat_n("back", 50)
        .collect::<PathBuf>()
        .join("output");
    assert_eq!(resolve(&independent, &root).unwrap(), root.join("output"));
    assert!(!root.join("missing").exists());
    fs::remove_dir_all(root).unwrap();
}
