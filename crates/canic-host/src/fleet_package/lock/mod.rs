//! Refresh the generated graph's parent seed before Cargo resolves the wrapper workspace.
//!
//! Canic owns this derivation; Cargo owns the resolved lock. Unchanged inputs preserve resolution.

#[cfg(test)]
mod tests;

use canic_core::cdk::utils::hash::sha256_hex;
use ic_host_artifacts::artifact::BoundedWriter;
use ic_host_fs::durable::{PublicationMode, WriteOptions, write_bytes, write_typed_with};
use ic_host_fs::read::{read_file_no_follow, read_optional_file_no_follow};
use serde::{Deserialize, Serialize};
use std::path::Path;

const MAX_SEED_BYTES: usize = 4096;

/// Derivation identity committed after the complete parent lock is durably installed.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct GeneratedLockSeedRecord {
    schema_version: u8,
    parent_lock_sha256: String,
    manifest_sha256: String,
}

pub(super) fn refresh_seed(
    directory: &Path,
    parent: &Path,
    manifest: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = read_file_no_follow(parent, 16 * 1024 * 1024).map_err(std::io::Error::from)?;
    let expected = GeneratedLockSeedRecord {
        schema_version: 1,
        parent_lock_sha256: sha256_hex(&bytes),
        manifest_sha256: sha256_hex(manifest),
    };
    let record = directory.join("lock-seed.json");
    let retained = read_optional_file_no_follow(&record, MAX_SEED_BYTES)
        .map_err(|error| {
            format!(
                "cannot read generated lock seed {}: {error:?}",
                record.display()
            )
        })?
        .and_then(|bytes| serde_json::from_slice::<GeneratedLockSeedRecord>(&bytes).ok());
    let lock = directory.join("Cargo.lock");
    if retained.as_ref() == Some(&expected)
        && read_optional_file_no_follow(&lock, 16 * 1024 * 1024)
            .map_err(|error| format!("cannot read generated lock {}: {error:?}", lock.display()))?
            .is_some()
    {
        return Ok(());
    }
    // A crash before the derivation record commits causes an identical reseed on retry.
    // Cargo resolution follows materialization and may legitimately change the lock bytes.
    write_bytes(&lock, &bytes)?;
    write_typed_with(
        &record,
        WriteOptions {
            mode: PublicationMode::Replace,
            permissions: 0o666,
        },
        |file| {
            serde_json::to_writer_pretty(BoundedWriter::new(file, MAX_SEED_BYTES as u64), &expected)
        },
    )?;
    Ok(())
}
