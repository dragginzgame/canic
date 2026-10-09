//! External asset byte and native-cycle observations.
//!
//! This owner never uploads, reinstalls, tops up or transfers Ledger funds.

use crate::{
    frontend::{
        FrontendError,
        model::FrontendAssetCapacityInput,
        view::{FrontendAssetCapacityView, FrontendPayloadView},
    },
    icp::IcpCli,
};
use canic_core::cdk::utils::hash::hex_bytes;
use ic_host_artifacts::artifact::ArtifactError;
use ic_host_fs::read::hash_file_no_follow;
use sha2_host::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

/// Hash a finite selected payload without loading entire asset files into memory.
pub fn payload_inventory(
    root: &Path,
    maximum_bytes: u64,
    maximum_files: u32,
) -> Result<FrontendPayloadView, FrontendError> {
    if maximum_bytes == 0 || maximum_files == 0 {
        return Err(FrontendError::Bound("nonzero payload limits"));
    }
    if !fs::symlink_metadata(root)?.is_dir() {
        return Err(FrontendError::Integrity);
    }
    let mut pending = vec![root.to_path_buf()];
    let mut files = BTreeMap::new();
    let mut total = 0_u64;
    let mut entries = 0_u64;
    while let Some(path) = pending.pop() {
        entries += 1;
        if entries > u64::from(maximum_files) * 8 + 1 {
            return Err(FrontendError::Bound("payload tree entries"));
        }
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(FrontendError::Integrity);
        }
        if metadata.is_dir() {
            for child in fs::read_dir(path)? {
                if pending.len() as u64 > u64::from(maximum_files) * 8 {
                    return Err(FrontendError::Bound("payload tree entries"));
                }
                pending.push(child?.path());
            }
        } else if metadata.is_file() {
            if files.len() >= maximum_files as usize {
                return Err(FrontendError::Bound("payload files"));
            }
            total = total
                .checked_add(metadata.len())
                .ok_or(FrontendError::Bound("payload bytes"))?;
            if total > maximum_bytes {
                return Err(FrontendError::Bound("payload bytes"));
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|_| FrontendError::Integrity)?;
            let relative = relative
                .to_str()
                .ok_or(FrontendError::Integrity)?
                .to_string();
            let hash = hash_file(&path, metadata.len())?;
            files.insert(relative, (metadata.len(), hash));
        } else {
            return Err(FrontendError::Integrity);
        }
    }
    let mut hash = Sha256::new();
    hash.update(b"canic:frontend-payload:v1\0");
    for (name, (bytes, digest)) in &files {
        hash.update((name.len() as u64).to_be_bytes());
        hash.update(name.as_bytes());
        hash.update(bytes.to_be_bytes());
        hash.update(digest);
    }
    Ok(FrontendPayloadView {
        files: u32::try_from(files.len()).map_err(|_| FrontendError::Bound("payload files"))?,
        bytes: total,
        sha256: hex_bytes(hash.finalize()),
    })
}

fn hash_file(path: &Path, expected_bytes: u64) -> Result<[u8; 32], FrontendError> {
    let identity = hash_file_no_follow(path, expected_bytes).map_err(|error| match error {
        ArtifactError::Io(source) => FrontendError::Io(source),
        _ => FrontendError::Integrity,
    })?;
    if identity.bytes != expected_bytes {
        return Err(FrontendError::Integrity);
    }
    Ok(*identity.sha256.as_bytes())
}

/// Observe native canister cycles, distinct from any Cycles Ledger account balance.
pub fn asset_capacity(
    icp: &IcpCli,
    input: &FrontendAssetCapacityInput,
) -> Result<FrontendAssetCapacityView, FrontendError> {
    if icp.environment() != Some(input.environment.as_str()) {
        return Err(FrontendError::Environment);
    }
    if input.minimum_native_cycles == 0 {
        return Err(FrontendError::Bound("nonzero native cycle floor"));
    }
    if input.canister_id == candid::Principal::anonymous()
        || input.canister_id == candid::Principal::management_canister()
    {
        return Err(FrontendError::Principal);
    }
    let payload = payload_inventory(
        &input.payload_directory,
        input.maximum_payload_bytes,
        input.maximum_files,
    )?;
    let status = icp.canister_status_report(&input.canister_id.to_text())?;
    if status.id != input.canister_id.to_text() {
        return Err(FrontendError::Integrity);
    }
    let native = status
        .cycles
        .as_deref()
        .and_then(|value| value.replace('_', "").trim().parse::<u128>().ok())
        .ok_or(FrontendError::NativeBalance)?;
    let observed_at_unix_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| FrontendError::Integrity)?
        .as_secs();
    Ok(FrontendAssetCapacityView {
        environment: input.environment.clone(),
        canister_id: input.canister_id.to_text(),
        observed_at_unix_secs,
        native_cycles: native.to_string(),
        minimum_native_cycles: input.minimum_native_cycles.to_string(),
        payload,
        sufficient: native >= input.minimum_native_cycles,
    })
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_file_hash_requires_exact_bytes_and_preserves_native_errors() {
        let root = crate::test_support::temp_dir("frontend-file-hash");
        fs::create_dir_all(&root).unwrap();
        let file = root.join("asset");
        fs::write(&file, b"abc").unwrap();
        assert_eq!(
            hex_bytes(hash_file(&file, 3).unwrap()),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        for expected_bytes in [2, 4] {
            assert!(matches!(
                hash_file(&file, expected_bytes),
                Err(FrontendError::Integrity)
            ));
        }
        assert!(matches!(hash_file(&root, 3), Err(FrontendError::Integrity)));
        assert!(matches!(
            hash_file(&root.join("missing"), 3),
            Err(FrontendError::Io(source)) if source.kind() == std::io::ErrorKind::NotFound
        ));
        #[cfg(unix)]
        {
            let link = root.join("link");
            std::os::unix::fs::symlink(&file, &link).unwrap();
            assert!(matches!(
                hash_file(&link, 3),
                Err(FrontendError::Io(source))
                    if source.raw_os_error() == Some(rustix::io::Errno::LOOP.raw_os_error())
            ));
            let fifo = root.join("fifo");
            crate::test_support::create_fifo(&fifo);
            assert!(matches!(hash_file(&fifo, 3), Err(FrontendError::Integrity)));
        }
        fs::remove_dir_all(root).unwrap();
    }
}
