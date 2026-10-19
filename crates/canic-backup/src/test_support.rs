use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
pub use ic_host_process::testing::crash_barrier::{
    hold_at_acknowledged_barrier, kill_child_at_acknowledged_barrier, wait_for_child_path,
    wait_for_path,
};

// Build a unique temporary directory path for tests that create their own layout.
pub fn temp_dir(prefix: &str) -> PathBuf {
    std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(unique_name(prefix))
}

// Build a unique temporary file path for tests that only need one artifact.
pub fn temp_path(prefix: &str) -> PathBuf {
    temp_dir(prefix)
}

// Include process and timestamp data so parallel test runs do not collide.
fn unique_name(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    format!("{prefix}-{}-{nanos}", std::process::id())
}
