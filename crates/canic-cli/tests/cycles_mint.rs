//! Exercise mint amount dispatch through the CLI with an inert ICP executable.

#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn mint_amount_and_global_executable_reach_the_selected_icp() {
    let root = std::env::temp_dir().join(format!(
        "canic-mint-amount-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let app = root.join("apps/fixture");
    let bin = root.join("bin");
    fs::create_dir_all(&app).unwrap();
    fs::create_dir(&bin).unwrap();
    fs::write(app.join("canic.toml"), "[app]\nname = \"fixture\"\n").unwrap();
    fs::write(root.join("icp.yaml"), "canisters: []\n").unwrap();
    let root = fs::canonicalize(root).unwrap();
    let icp = root.join("bin/icp");
    fs::write(
        &icp,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
    printf '%s\n' 'icp 1.7.0'
    exit 0
fi
printf '%s\n' "$@" > arguments
printf '%s\n' 'stub mint accepted'
"#,
    )
    .unwrap();
    fs::set_permissions(&icp, fs::Permissions::from_mode(0o700)).unwrap();
    let command = || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_canic"));
        command
            .current_dir(&root)
            .env("PATH", root.join("bin"))
            .env_remove("CANIC_ICP_IDENTITY_PASSWORD_FILE");
        command
    };
    for (binary_flag, amount) in [
        (false, vec!["--icp", "0.01"]),
        (true, vec!["--icp", "0.01"]),
        (true, vec!["--icp=0.01"]),
        (true, vec!["--cycles", "100000000000"]),
    ] {
        let mut command = command();
        command.args(["--environment", "sample"]);
        if binary_flag {
            command.arg("--icp").arg(&icp).env("PATH", &root);
        }
        let output = command
            .args(["cycles", "mint"])
            .args(&amount)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{amount:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"stub mint accepted\n");
        let amount = if amount[0] == "--cycles" {
            ["--cycles", "100000000000"]
        } else {
            ["--icp", "0.01"]
        };
        assert_eq!(
            fs::read_to_string(root.join("arguments")).unwrap(),
            format!(
                "--project-root-override\n{}\ncycles\nmint\n{}\n{}\n-e\nsample\n",
                root.display(),
                amount[0],
                amount[1]
            )
        );
    }
    for args in [
        vec!["cycles", "mint", "--environment", "sample"],
        vec!["cycles", "balance", "--icp", "0.01"],
        vec!["cycles", "mint", "--icp"],
    ] {
        fs::write(root.join("arguments"), "").unwrap();
        let output = command().args(args).output().unwrap();
        assert!(!output.status.success());
        assert_eq!(fs::read(root.join("arguments")).unwrap(), b"");
    }
    fs::remove_dir_all(root).unwrap();
}
