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
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Read as _,
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
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(
            i32::try_from(rustix::fs::OFlags::NOFOLLOW.bits())
                .map_err(|_| FrontendError::Integrity)?,
        );
    }
    let mut file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(FrontendError::Integrity);
    }
    let mut buffer = vec![0_u8; 64 * 1024];
    let mut total = 0_u64;
    let mut hash = Sha256::new();
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count as u64)
            .ok_or(FrontendError::Integrity)?;
        if total > expected_bytes {
            return Err(FrontendError::Integrity);
        }
        hash.update(&buffer[..count]);
    }
    if total != expected_bytes {
        return Err(FrontendError::Integrity);
    }
    Ok(hash.finalize().into())
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
