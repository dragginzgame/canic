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
