//! Module: install_root::fleet_component_provisioning_journal::persistence
//!
//! Responsibility: persist and reload the exact canonical provisioning journal document.
//! Does not own: phase decisions, authority semantics, remote effects, or Fleet catalog policy.
//! Boundary: no-follow reads, exclusive locking and atomic publication are complete here.

use super::{
    model::{
        FleetComponentProvisioningInstallJournal, FleetComponentProvisioningInstallJournalError,
        ResolvedFleetComponentProvisioningInstall, resolved,
    },
    validation::{invalid, same_immutable_authority, validate_journal},
};
use crate::durable_io::{
    BoundedRegularFileReadError, CanonicalJsonEncodeError, CanonicalJsonStyle, ExactReplaceError,
    RegularFileLockError, RegularFileReadError, create_new_bytes_with_parents,
    encode_canonical_json, lock_regular_file_with_parents, read_optional_bounded_regular_bytes,
    replace_bytes_exact,
};
use std::{
    io,
    path::{Path, PathBuf},
};

const JOURNAL_FILE: &str = "fleet-component-provisioning-install-journal.json";
const JOURNAL_LOCK_FILE: &str = "fleet-component-provisioning-install-journal.lock";
// The bounded 8 MiB canonical provisioning plan expands substantially in JSON.
const MAX_JOURNAL_BYTES: usize = 67_108_864;

pub(super) fn create_or_load(
    path: PathBuf,
    expected: FleetComponentProvisioningInstallJournal,
) -> Result<ResolvedFleetComponentProvisioningInstall, FleetComponentProvisioningInstallJournalError>
{
    let _lock = lock_journal(&path)?;
    if let Some(observed) = load_optional_journal(&path)? {
        if same_immutable_authority(&observed, &expected) {
            return Ok(resolved(observed, path));
        }
        return Err(FleetComponentProvisioningInstallJournalError::ConflictingAuthority { path });
    }

    let bytes = encode_journal(&path, &expected)?;
    if let Err(source) = create_new_bytes_with_parents(&path, &bytes) {
        return resolve_create_failure(path, expected, source);
    }
    let durable = load_required_journal(&path)?;
    if durable != expected {
        return Err(invalid(
            &path,
            "published journal differs from the planned Component provisioning authority",
        ));
    }
    Ok(resolved(durable, path))
}

pub(super) fn replace_exact(
    current: &ResolvedFleetComponentProvisioningInstall,
    next: FleetComponentProvisioningInstallJournal,
) -> Result<ResolvedFleetComponentProvisioningInstall, FleetComponentProvisioningInstallJournalError>
{
    let _lock = lock_journal(&current.path)?;
    let observed = load_required_journal(&current.path)?;
    if observed != current.journal {
        return Err(invalid(&current.path, "journal changed before transition"));
    }
    let bytes = encode_journal(&current.path, &next)?;
    replace_bytes_exact(&current.path, &bytes)
        .map_err(|error| replace_error(&current.path, error))?;
    let durable = load_required_journal(&current.path)?;
    if durable != next {
        return Err(invalid(
            &current.path,
            "durable transition differs from request",
        ));
    }
    Ok(resolved(durable, current.path.clone()))
}

pub(super) fn journal_path(plan_path: &Path) -> PathBuf {
    plan_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(JOURNAL_FILE)
}

fn resolve_create_failure(
    path: PathBuf,
    expected: FleetComponentProvisioningInstallJournal,
    source: io::Error,
) -> Result<ResolvedFleetComponentProvisioningInstall, FleetComponentProvisioningInstallJournalError>
{
    match load_optional_journal(&path)? {
        Some(observed) if same_immutable_authority(&observed, &expected) => {
            Ok(resolved(observed, path))
        }
        Some(_) if source.kind() == io::ErrorKind::AlreadyExists => {
            Err(FleetComponentProvisioningInstallJournalError::ConflictingAuthority { path })
        }
        _ => Err(FleetComponentProvisioningInstallJournalError::Io { path, source }),
    }
}

fn load_optional_journal(
    path: &Path,
) -> Result<
    Option<FleetComponentProvisioningInstallJournal>,
    FleetComponentProvisioningInstallJournalError,
> {
    let bytes =
        read_optional_bounded_regular_bytes(path, MAX_JOURNAL_BYTES).map_err(
            |error| match error {
                BoundedRegularFileReadError::TooLarge => {
                    invalid(path, "journal exceeds size bound")
                }
                BoundedRegularFileReadError::Read(RegularFileReadError::NotRegular) => {
                    FleetComponentProvisioningInstallJournalError::UnsafeFile {
                        path: path.to_path_buf(),
                    }
                }
                BoundedRegularFileReadError::Read(RegularFileReadError::Io(source)) => {
                    FleetComponentProvisioningInstallJournalError::Io {
                        path: path.to_path_buf(),
                        source,
                    }
                }
                #[cfg(not(unix))]
                BoundedRegularFileReadError::Read(RegularFileReadError::UnsupportedPlatform) => {
                    FleetComponentProvisioningInstallJournalError::Io {
                        path: path.to_path_buf(),
                        source: io::Error::new(
                            io::ErrorKind::Unsupported,
                            "regular no-follow journal reads are unsupported",
                        ),
                    }
                }
            },
        )?;
    let Some(bytes) = bytes else {
        return Ok(None);
    };
    let journal = serde_json::from_slice::<FleetComponentProvisioningInstallJournal>(&bytes)
        .map_err(|error| invalid(path, error.to_string()))?;
    validate_journal(path, &journal)?;
    if encode_journal(path, &journal)? != bytes {
        return Err(invalid(path, "journal bytes are not canonical"));
    }
    Ok(Some(journal))
}

fn load_required_journal(
    path: &Path,
) -> Result<FleetComponentProvisioningInstallJournal, FleetComponentProvisioningInstallJournalError>
{
    load_optional_journal(path)?.ok_or_else(|| invalid(path, "journal is missing"))
}

fn encode_journal(
    path: &Path,
    journal: &FleetComponentProvisioningInstallJournal,
) -> Result<Vec<u8>, FleetComponentProvisioningInstallJournalError> {
    validate_journal(path, journal)?;
    encode_canonical_json(
        journal,
        CanonicalJsonStyle::PrettyNewline,
        MAX_JOURNAL_BYTES,
    )
    .map_err(|error| match error {
        CanonicalJsonEncodeError::Serialization(error) => invalid(path, error.to_string()),
        CanonicalJsonEncodeError::TooLarge => invalid(path, "journal exceeds size bound"),
    })
}

fn replace_error(
    path: &Path,
    error: ExactReplaceError,
) -> FleetComponentProvisioningInstallJournalError {
    match error {
        ExactReplaceError::Write(source)
        | ExactReplaceError::Read(RegularFileReadError::Io(source)) => {
            FleetComponentProvisioningInstallJournalError::Io {
                path: path.to_path_buf(),
                source,
            }
        }
        ExactReplaceError::Read(RegularFileReadError::NotRegular) => {
            FleetComponentProvisioningInstallJournalError::UnsafeFile {
                path: path.to_path_buf(),
            }
        }
        #[cfg(not(unix))]
        ExactReplaceError::Read(RegularFileReadError::UnsupportedPlatform) => {
            FleetComponentProvisioningInstallJournalError::Io {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "regular no-follow journal reads are unsupported",
                ),
            }
        }
    }
}

fn lock_path(path: &Path) -> PathBuf {
    path.parent()
        .unwrap_or_else(|| Path::new("."))
        .join(JOURNAL_LOCK_FILE)
}

fn lock_journal(
    path: &Path,
) -> Result<std::fs::File, FleetComponentProvisioningInstallJournalError> {
    let lock = lock_path(path);
    lock_regular_file_with_parents(&lock).map_err(|error| match error {
        RegularFileLockError::NotRegular => {
            FleetComponentProvisioningInstallJournalError::UnsafeLock { path: lock }
        }
        RegularFileLockError::Io(source) => {
            FleetComponentProvisioningInstallJournalError::Io { path: lock, source }
        }
        #[cfg(windows)]
        RegularFileLockError::UnsupportedPlatform => {
            FleetComponentProvisioningInstallJournalError::Io {
                path: lock,
                source: io::Error::new(io::ErrorKind::Unsupported, "file locking is unsupported"),
            }
        }
    })
}
