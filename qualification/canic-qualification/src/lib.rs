//! Shared preparation paths for installed Canic qualification; no product API.

/// Prepared PocketIC bundle used by every installed fixture and explicit setup.
/// Tests check this directory; only the setup example provisions it.
///
/// # Panics
/// Requires this package to remain below the repository's qualification directory.
#[must_use]
pub fn server_directory() -> std::path::PathBuf {
    std::env::var_os("IC_TESTKIT_SERVER_DIRECTORY")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(2)
                .expect("repository root")
                .join("target/qualification/server")
        })
}

/// Create a signer identity inside an existing fixture directory, never the operator wallet.
/// Uses the prepared repository ICP executable; seed and configuration stay in the fixture.
/// Returns the isolated signer wrapper and its public principal.
///
/// # Panics
/// Panics on unavailable prepared tools, invalid paths or failed identity creation.
#[cfg(unix)]
#[must_use]
pub fn isolated_signer(
    directory: &std::path::Path,
    identity: &str,
) -> (std::path::PathBuf, String) {
    use std::{os::unix::fs::PermissionsExt, process::Command};
    fn quoted(path: &std::path::Path) -> String {
        format!(
            "'{}'",
            path.to_str()
                .expect("UTF-8 fixture path")
                .replace('\'', "'\\''")
        )
    }
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repository root");
    let signer = directory.join("signer");
    std::fs::write(&signer, format!("#!/bin/sh\nexport ICP_HOME={}\nexport XDG_CONFIG_HOME={}\nexport ICP_TELEMETRY_DISABLED=1\nexec {} \"$@\"\n", quoted(&directory.join("icp")), quoted(&directory.join("config")), quoted(&repo.join(".tools/ic/bin/icp")))).expect("fixture signer");
    std::fs::set_permissions(&signer, std::fs::Permissions::from_mode(0o700))
        .expect("private fixture signer");
    let created = Command::new(&signer)
        .args([
            "identity",
            "new",
            identity,
            "--storage",
            "plaintext",
            "--output-seed",
        ])
        .arg(directory.join("fixture-seed.txt"))
        .output()
        .expect("prepared ICP signer");
    assert!(
        created.status.success(),
        "isolated identity creation failed"
    );
    let principal = Command::new(&signer)
        .args(["identity", "principal", "--identity", identity])
        .output()
        .expect("fixture public principal");
    assert!(principal.status.success(), "fixture principal failed");
    (
        signer,
        String::from_utf8(principal.stdout)
            .expect("principal text")
            .trim()
            .to_owned(),
    )
}
