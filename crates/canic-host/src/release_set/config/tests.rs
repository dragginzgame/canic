use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn failed_package_manifest_write_restores_original_config() {
    let root = temp_root("rename-rollback");
    fs::create_dir_all(&root).expect("create temp root");
    let config_path = root.join("canic.toml");
    let invalid_package_target = root.join("Cargo.toml");
    fs::write(&config_path, "original config").expect("write original config");
    fs::create_dir(&invalid_package_target).expect("create invalid package target directory");

    commit_role_rename_sources(
        &config_path,
        "original config",
        "updated config",
        Some((&invalid_package_target, "updated package")),
    )
    .expect_err("package write must fail");

    assert_eq!(
        fs::read_to_string(&config_path).expect("read rolled back config"),
        "original config"
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "canic-host-release-config-{label}-{}-{nanos}",
        std::process::id()
    ))
}
