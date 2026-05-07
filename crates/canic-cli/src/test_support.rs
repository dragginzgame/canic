use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

// Build a unique temporary directory path for tests that manage their own cleanup.
pub fn temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}
