use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn durable_write_creates_parent_and_replaces_complete_contents() {
    let root = temp_root("replace");
    let path = root.join("nested/state.json");

    write_bytes(&path, b"old").expect("write initial contents");
    write_bytes(&path, b"new complete contents").expect("replace contents");

    assert_eq!(
        fs::read(&path).expect("read target"),
        b"new complete contents"
    );
    assert_eq!(
        fs::read_dir(path.parent().expect("target parent"))
            .expect("read target parent")
            .count(),
        1,
        "successful durable writes must not leave sibling temporary files"
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn durable_write_rejects_a_target_without_a_file_name() {
    let error = write_bytes(Path::new("/"), b"value").expect_err("directory target must fail");

    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
}

fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "canic-host-durable-io-{label}-{}-{nanos}",
        std::process::id()
    ))
}
