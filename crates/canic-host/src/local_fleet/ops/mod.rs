//! Exact directory locking, durable identity publication and PocketIC projection.

pub mod bindings;
pub mod checkpoint;
pub mod discovery;
pub mod prepare;
pub mod reset;
pub mod runtime;

use crate::{
    durable_io,
    local_fleet::{
        LocalFleetError,
        model::{
            LocalAllocationInput, LocalAllocationIntentRecord, LocalAllocationRecord,
            LocalCheckpoint, LocalFleetConfig, LocalFleetRecord,
        },
        view::{LocalCanisterView, LocalFleetView},
    },
};
use candid::Principal;
use canic_core::cdk::utils::hash::hex_bytes;
use ic_testkit::pocket_ic::{PocketIc, common::rest::Topology};
use sha2_host::{Digest, Sha256};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
};

/// Own one exact named local state directory; reject symlinks before creating descendants.
pub fn lock_directory(root: &Path, name: &str) -> Result<(PathBuf, File), LocalFleetError> {
    crate::component_operation::policy::validate_label(name)
        .map_err(|_| LocalFleetError::Configuration)?;
    let mut directory = root.canonicalize()?;
    for part in [".canic", "local-fleets", name] {
        directory.push(part);
        match fs::symlink_metadata(&directory) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => return Err(LocalFleetError::UnsafePath),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let mut builder = fs::DirBuilder::new();
                #[cfg(unix)]
                {
                    builder.mode(0o700);
                }
                builder.create(&directory)?;
            }
            Err(error) => return Err(error.into()),
        }
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        options.mode(0o600).custom_flags(
            i32::try_from(rustix::fs::OFlags::NOFOLLOW.bits())
                .map_err(|_| LocalFleetError::Configuration)?,
        );
    }
    let lock = options.open(directory.join("owner.lock"))?;
    if !lock.metadata()?.is_file() {
        return Err(LocalFleetError::UnsafePath);
    }
    lock.try_lock().map_err(|_| LocalFleetError::Busy)?;
    Ok((directory, lock))
}

/// Read only the maintained record, with a finite no-follow document bound.
pub fn read_record(directory: &Path) -> Result<Option<LocalFleetRecord>, LocalFleetError> {
    match durable_io::read_regular_bytes(&directory.join("environment.json"), 1024 * 1024) {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Publish a complete record before another local effect can be attempted.
pub fn write_record(directory: &Path, record: &LocalFleetRecord) -> Result<(), LocalFleetError> {
    let bytes = serde_json::to_vec_pretty(record)?;
    if bytes.len() > 1024 * 1024 {
        return Err(LocalFleetError::Capacity);
    }
    durable_io::write_bytes(&directory.join("environment.json"), &bytes)?;
    Ok(())
}

/// Bind the new or resumed instance to its exact configuration and trust bytes.
pub fn instance_record(
    config: &LocalFleetConfig,
    previous: Option<LocalFleetRecord>,
    pic: &PocketIc,
) -> Result<LocalFleetRecord, LocalFleetError> {
    let topology = pic.topology();
    let subnets = topology.get_app_subnets();
    let root_key_der_hex =
        canic_core::cdk::utils::hash::hex_bytes(pic.root_key().ok_or(LocalFleetError::Identity)?);
    if let Some(record) = previous {
        let expected = InstanceAuthority {
            version: env!("CARGO_PKG_VERSION"),
            configuration: config,
            subnets: &subnets,
            root_key: &root_key_der_hex,
        };
        let retained = InstanceAuthority {
            version: &record.canic_version,
            configuration: &record.configuration,
            subnets: &record.application_subnets,
            root_key: &record.root_key_der_hex,
        };
        if record.schema_version != 1 || retained != expected {
            return Err(LocalFleetError::Identity);
        }
        validate_allocations(&record, &topology)?;
        return Ok(record);
    }
    let mut record = initial_record(config)?;
    record.application_subnets = subnets;
    record.root_key_der_hex = root_key_der_hex;
    Ok(record)
}

/// Observe only the process-owned allocation set; no live Toko or mainnet discovery occurs.
#[must_use]
pub fn status(record: &LocalFleetRecord, pic: &PocketIc, gateway: &str) -> LocalFleetView {
    LocalFleetView {
        schema_version: 1,
        session_id: record.session_id.clone(),
        name: record.configuration.name.clone(),
        environment: environment_name(record),
        gateway: gateway.into(),
        root_key_der_hex: record.root_key_der_hex.clone(),
        application_subnets: record.application_subnets.clone(),
        maximum_canisters: record.configuration.maximum_canisters,
        canister_memory_bytes: record.configuration.canister_memory_bytes,
        simulated_time_ns: pic.get_time().as_nanos_since_unix_epoch().to_string(),
        canisters: record
            .allocations
            .iter()
            .map(|entry| {
                let exists = entry.canister_id.is_some_and(|id| pic.canister_exists(id));
                LocalCanisterView {
                    name: entry.input.name.clone(),
                    allocation_role: entry.input.role.clone(),
                    canister_id: entry.canister_id,
                    subnet_id: entry.subnet_id,
                    exists,
                    native_cycles: if exists {
                        entry
                            .canister_id
                            .map(|id| pic.cycle_balance(id).to_string())
                            .unwrap_or_default()
                    } else {
                        "0".into()
                    },
                }
            })
            .collect(),
    }
}

/// Retain the complete idempotent Ledger request before its first submission.
pub fn reserve(
    directory: &Path,
    record: &LocalFleetRecord,
    input: &LocalAllocationInput,
    now: u64,
) -> Result<(LocalFleetRecord, LocalAllocationIntentRecord), LocalFleetError> {
    if let Some(existing) = record
        .allocations
        .iter()
        .find(|entry| entry.input.name == input.name)
    {
        if existing.input != *input {
            return Err(LocalFleetError::Identity);
        }
        return Ok((record.clone(), existing.clone()));
    }
    if record.release_build_id.is_some() {
        return Err(LocalFleetError::Identity);
    }
    if record.allocations.len() >= usize::from(record.configuration.maximum_canisters) {
        return Err(LocalFleetError::Capacity);
    }
    let subnet_id = *record
        .application_subnets
        .get(usize::from(input.application_subnet))
        .ok_or(LocalFleetError::Configuration)?;
    let previous = record
        .allocations
        .last()
        .map_or(0, |entry| entry.created_at_time);
    let created_at_time = now.max(previous.checked_add(1).ok_or(LocalFleetError::Capacity)?);
    let intent = LocalAllocationIntentRecord {
        input: input.clone(),
        subnet_id,
        created_at_time,
        canister_id: None,
    };
    let mut next = record.clone();
    next.allocations.push(intent.clone());
    write_record(directory, &next)?;
    Ok((next, intent))
}

/// Persist the observed Ledger result without changing any request identity.
pub fn complete_allocation(
    directory: &Path,
    record: &LocalFleetRecord,
    intent: &LocalAllocationIntentRecord,
    id: Principal,
) -> Result<(LocalFleetRecord, LocalAllocationRecord), LocalFleetError> {
    let mut next = record.clone();
    let entry = next
        .allocations
        .iter_mut()
        .find(|entry| entry.input.name == intent.input.name)
        .ok_or(LocalFleetError::Identity)?;
    if entry != intent || entry.canister_id.is_some_and(|previous| previous != id) {
        return Err(LocalFleetError::Identity);
    }
    entry.canister_id = Some(id);
    let allocation = LocalAllocationRecord {
        input: entry.input.clone(),
        canister_id: id,
        subnet_id: entry.subnet_id,
    };
    write_record(directory, &next)?;
    Ok((next, allocation))
}

/// Reject state trees that could escape the one owned directory during load or reset.
pub fn validate_tree(directory: &Path) -> Result<(), LocalFleetError> {
    let mut pending = vec![(directory.to_path_buf(), 0_u16)];
    let mut count = 0_usize;
    while let Some((path, depth)) = pending.pop() {
        count += 1;
        if count > 100_000 || depth > 64 {
            return Err(LocalFleetError::Capacity);
        }
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() || (!metadata.is_dir() && !metadata.is_file()) {
            return Err(LocalFleetError::UnsafePath);
        }
        if metadata.is_dir() {
            for entry in fs::read_dir(path)? {
                if pending.len() + count >= 100_000 {
                    return Err(LocalFleetError::Capacity);
                }
                pending.push((entry?.path(), depth + 1));
            }
        }
    }
    Ok(())
}

/// Publish checkpoint state without inventing a successful simulator save.
pub fn checkpoint(
    directory: &Path,
    record: &LocalFleetRecord,
    checkpoint: LocalCheckpoint,
) -> Result<LocalFleetRecord, LocalFleetError> {
    let mut next = record.clone();
    next.checkpoint = checkpoint;
    write_record(directory, &next)?;
    Ok(next)
}

/// Exact loaded-instance authority, compared as one named contract.
#[derive(Eq, PartialEq)]
struct InstanceAuthority<'a> {
    version: &'a str,
    configuration: &'a LocalFleetConfig,
    subnets: &'a [Principal],
    root_key: &'a str,
}

fn validate_allocations(
    record: &LocalFleetRecord,
    topology: &Topology,
) -> Result<(), LocalFleetError> {
    if record.session_id.len() != 64
        || !record
            .session_id
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(LocalFleetError::Identity);
    }
    if record.allocations.len() > usize::from(record.configuration.maximum_canisters) {
        return Err(LocalFleetError::Capacity);
    }
    let mut names = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut last_time = 0;
    for entry in &record.allocations {
        crate::local_fleet::policy::validate_allocation(&entry.input, &record.configuration)?;
        let expected_subnet = record
            .application_subnets
            .get(usize::from(entry.input.application_subnet));
        if expected_subnet != Some(&entry.subnet_id)
            || !names.insert(&entry.input.name)
            || entry.created_at_time <= last_time
        {
            return Err(LocalFleetError::Identity);
        }
        if let Some(id) = entry.canister_id
            && (!ids.insert(id) || topology.get_subnet(id) != Some(entry.subnet_id))
        {
            return Err(LocalFleetError::Identity);
        }
        last_time = entry.created_at_time;
    }
    Ok(())
}

/// Retain a resettable identity before the first simulator or filesystem state effect.
pub fn initial_record(config: &LocalFleetConfig) -> Result<LocalFleetRecord, LocalFleetError> {
    let mut nonce = [0; 32];
    getrandom::fill(&mut nonce).map_err(|error| LocalFleetError::Platform(error.to_string()))?;
    Ok(LocalFleetRecord {
        schema_version: 1,
        canic_version: env!("CARGO_PKG_VERSION").into(),
        session_id: canic_core::cdk::utils::hash::hex_bytes(nonce),
        configuration: config.clone(),
        application_subnets: Vec::new(),
        root_key_der_hex: String::new(),
        allocations: Vec::new(),
        checkpoint: LocalCheckpoint::Running,
        release_build_id: None,
    })
}

/// Complete first-instance authority while preserving its pre-startup reset identity.
pub fn admit_initial_instance(
    record: &LocalFleetRecord,
    pic: &PocketIc,
) -> Result<LocalFleetRecord, LocalFleetError> {
    let mut next = instance_record(&record.configuration, None, pic)?;
    next.session_id.clone_from(&record.session_id);
    Ok(next)
}

/// Fingerprint one bounded regular executable without following its final symlink.
pub fn binary_sha256(path: &Path) -> Result<String, LocalFleetError> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        options.custom_flags(
            i32::try_from(rustix::fs::OFlags::NOFOLLOW.bits())
                .map_err(|_| LocalFleetError::Configuration)?,
        );
    }
    let mut file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > 512 * 1024 * 1024 {
        return Err(LocalFleetError::UnsafePath);
    }
    let mut hash = Sha256::new();
    let mut buffer = [0; 16_384];
    let mut read = 0_u64;
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        read += count as u64;
        if read > 512 * 1024 * 1024 {
            return Err(LocalFleetError::Capacity);
        }
        hash.update(&buffer[..count]);
    }
    Ok(hex_bytes(hash.finalize()))
}

/// A reset receives a fresh environment namespace, so old Ensure journals cannot be resumed there.
#[must_use]
pub fn environment_name(record: &LocalFleetRecord) -> String {
    let name = record
        .configuration
        .name
        .chars()
        .take(24)
        .collect::<String>();
    format!("local-{name}-{}", &record.session_id[..16])
}

/// Never reuse a simulator directory across reset generations, including late orphan writes.
pub fn instance_directory(directory: &Path, session: &str) -> Result<PathBuf, LocalFleetError> {
    if session.len() != 64 || !session.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(LocalFleetError::Identity);
    }
    Ok(directory.join(format!("instance-{session}")))
}
