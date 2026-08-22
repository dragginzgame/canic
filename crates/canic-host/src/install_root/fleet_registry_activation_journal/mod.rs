//! Module: install_root::fleet_registry_activation_journal
//!
//! Responsibility: own host recovery authority for the initial atomic Registry activation.
//! Does not own: Coordinator state, root mirror publication, Directory activation, or runtime activation.
//! Boundary: the exact all-Joining source and all-Active target are durable before mutation.

#[cfg(test)]
mod tests;

use crate::{
    durable_io::{
        BoundedRegularFileReadError, CanonicalJsonEncodeError, CanonicalJsonStyle,
        ExactReplaceError, RegularFileLockError, RegularFileReadError,
        create_new_bytes_with_parents, encode_canonical_json, lock_regular_file_with_parents,
        read_optional_bounded_regular_bytes, replace_bytes_exact,
    },
    fleet_install_plan::PersistedFleetInstallPlan,
};
use canic_core::{
    control_plane_support::{config::ComponentTopology, ops::fleet_registry::FleetRegistryOps},
    dto::fleet_registry::{
        FleetRegistry, FleetRegistryActivationRequest, FleetRegistryActivationResponse,
        FleetRegistryManifest, FleetRegistryVersion,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const JOURNAL_FILE: &str = "fleet-registry-activation-journal.json";
const JOURNAL_LOCK_FILE: &str = "fleet-registry-activation-journal.lock";
const JOURNAL_SCHEMA_VERSION: u32 = 1;
// JSON can expand the bounded topology and two bounded Registry snapshots
// substantially beyond their canonical binary sizes.
const MAX_JOURNAL_BYTES: usize = 67_108_864;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum FleetRegistryActivationPhase {
    Planned,
    ActivationInFlight,
    Activated,
    Verified,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FleetRegistryActivationJournal {
    pub schema_version: u32,
    pub sequence: u64,
    pub phase: FleetRegistryActivationPhase,
    pub fleet_install_plan_digest: [u8; 32],
    pub component_topology: ComponentTopology,
    pub joining_registry: FleetRegistry,
    pub active_registry: FleetRegistry,
    pub request: FleetRegistryActivationRequest,
    pub response: Option<FleetRegistryActivationResponse>,
    pub verified_manifest: Option<FleetRegistryManifest>,
    pub verified_version: Option<FleetRegistryVersion>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ResolvedFleetRegistryActivation {
    pub journal: FleetRegistryActivationJournal,
    pub path: PathBuf,
    pub advanced: bool,
}

pub(super) struct PlanFleetRegistryActivationRequest<'a> {
    pub fleet_install_plan: &'a PersistedFleetInstallPlan,
    pub component_topology: ComponentTopology,
    pub joining_registry: FleetRegistry,
}

#[derive(Debug, ThisError)]
pub(super) enum FleetRegistryActivationJournalError {
    #[error("Fleet Registry activation journal already has different immutable authority: {path}")]
    ConflictingAuthority { path: PathBuf },

    #[error("invalid Fleet Registry activation journal {path}: {reason}")]
    InvalidDocument { path: PathBuf, reason: String },

    #[error(
        "Fleet Registry activation journal cannot transition from {observed:?} to {requested:?}"
    )]
    InvalidTransition {
        observed: FleetRegistryActivationPhase,
        requested: FleetRegistryActivationPhase,
    },

    #[error("failed to access Fleet Registry activation journal {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Fleet Registry activation journal is not a regular no-follow file: {path}")]
    UnsafeFile { path: PathBuf },

    #[error("Fleet Registry activation journal lock is not a regular no-follow file: {path}")]
    UnsafeLock { path: PathBuf },
}

pub(super) fn plan_fleet_registry_activation(
    request: PlanFleetRegistryActivationRequest<'_>,
) -> Result<ResolvedFleetRegistryActivation, FleetRegistryActivationJournalError> {
    let path = journal_path(&request.fleet_install_plan.path);
    let _lock = lock_journal(&path)?;
    let expected = planned_journal(&path, request)?;

    if let Some(observed) = load_optional_journal(&path)? {
        if same_immutable_authority(&observed, &expected) {
            return Ok(resolved(observed, path, false));
        }
        return Err(FleetRegistryActivationJournalError::ConflictingAuthority { path });
    }

    let bytes = encode_journal(&path, &expected)?;
    if let Err(source) = create_new_bytes_with_parents(&path, &bytes) {
        match load_optional_journal(&path)? {
            Some(observed) if same_immutable_authority(&observed, &expected) => {
                return Ok(resolved(observed, path, false));
            }
            Some(_) if source.kind() == io::ErrorKind::AlreadyExists => {
                return Err(FleetRegistryActivationJournalError::ConflictingAuthority { path });
            }
            _ => return Err(FleetRegistryActivationJournalError::Io { path, source }),
        }
    }

    let durable = load_required_journal(&path)?;
    if durable != expected {
        return Err(invalid(
            &path,
            "published journal differs from planned activation authority",
        ));
    }
    Ok(resolved(durable, path, true))
}

pub(super) fn begin_registry_activation(
    current: &ResolvedFleetRegistryActivation,
) -> Result<ResolvedFleetRegistryActivation, FleetRegistryActivationJournalError> {
    advance_without_evidence(
        current,
        FleetRegistryActivationPhase::Planned,
        FleetRegistryActivationPhase::ActivationInFlight,
    )
}

pub(super) fn record_registry_activated(
    current: &ResolvedFleetRegistryActivation,
    response: FleetRegistryActivationResponse,
) -> Result<ResolvedFleetRegistryActivation, FleetRegistryActivationJournalError> {
    validate_response(&current.path, &current.journal, &response)?;
    transition(
        current,
        FleetRegistryActivationPhase::ActivationInFlight,
        FleetRegistryActivationPhase::Activated,
        |next| next.response = Some(response),
    )
}

pub(super) fn record_registry_activation_verified(
    current: &ResolvedFleetRegistryActivation,
    manifest: FleetRegistryManifest,
    version: FleetRegistryVersion,
) -> Result<ResolvedFleetRegistryActivation, FleetRegistryActivationJournalError> {
    let expected_manifest = FleetRegistryOps::manifest(
        &current.journal.active_registry.authority,
        &current.journal.component_topology,
        &current.journal.active_registry,
    )
    .map_err(|error| invalid(&current.path, error.to_string()))?;
    let expected_version = FleetRegistryOps::version(
        &current.journal.active_registry.authority,
        &current.journal.component_topology,
        &current.journal.active_registry,
    )
    .map_err(|error| invalid(&current.path, error.to_string()))?;
    if manifest != expected_manifest || version != expected_version {
        return Err(invalid(
            &current.path,
            "verified active Registry evidence differs from planned authority",
        ));
    }
    transition(
        current,
        FleetRegistryActivationPhase::Activated,
        FleetRegistryActivationPhase::Verified,
        |next| {
            next.verified_manifest = Some(manifest);
            next.verified_version = Some(version);
        },
    )
}

pub(super) fn load_verified_installed_registry(
    fleet_install_plan: &PersistedFleetInstallPlan,
) -> Result<FleetRegistry, FleetRegistryActivationJournalError> {
    let path = journal_path(&fleet_install_plan.path);
    let journal = load_required_journal(&path)?;
    if journal.phase != FleetRegistryActivationPhase::Verified
        || journal.fleet_install_plan_digest != fleet_install_plan.digest
        || journal.active_registry.authority.binding.fleet != fleet_install_plan.plan.fleet
        || journal.active_registry.authority.binding.coordinator_subnet
            != fleet_install_plan.plan.coordinator.coordinator_subnet
    {
        return Err(invalid(
            &path,
            "verified active Registry differs from the Fleet install plan",
        ));
    }
    Ok(journal.active_registry)
}

fn planned_journal(
    path: &Path,
    request: PlanFleetRegistryActivationRequest<'_>,
) -> Result<FleetRegistryActivationJournal, FleetRegistryActivationJournalError> {
    if request.joining_registry.authority.binding.fleet != request.fleet_install_plan.plan.fleet
        || request
            .joining_registry
            .authority
            .binding
            .coordinator_subnet
            != request
                .fleet_install_plan
                .plan
                .coordinator
                .coordinator_subnet
    {
        return Err(invalid(
            path,
            "all-Joining Registry authority differs from the Fleet install plan",
        ));
    }
    let active_registry = FleetRegistryOps::compile_active(
        &request.joining_registry.authority,
        &request.component_topology,
        &request.joining_registry,
    )
    .map_err(|error| invalid(path, error.to_string()))?;
    let expected_registry = FleetRegistryOps::version(
        &request.joining_registry.authority,
        &request.component_topology,
        &request.joining_registry,
    )
    .map_err(|error| invalid(path, error.to_string()))?;
    let journal = FleetRegistryActivationJournal {
        schema_version: JOURNAL_SCHEMA_VERSION,
        sequence: 0,
        phase: FleetRegistryActivationPhase::Planned,
        fleet_install_plan_digest: request.fleet_install_plan.digest,
        component_topology: request.component_topology,
        joining_registry: request.joining_registry,
        active_registry,
        request: FleetRegistryActivationRequest { expected_registry },
        response: None,
        verified_manifest: None,
        verified_version: None,
    };
    validate_journal(path, &journal)?;
    Ok(journal)
}

fn advance_without_evidence(
    current: &ResolvedFleetRegistryActivation,
    expected: FleetRegistryActivationPhase,
    requested: FleetRegistryActivationPhase,
) -> Result<ResolvedFleetRegistryActivation, FleetRegistryActivationJournalError> {
    if current.journal.phase == requested {
        return Ok(resolved(
            current.journal.clone(),
            current.path.clone(),
            false,
        ));
    }
    transition(current, expected, requested, |_| {})
}

fn transition(
    current: &ResolvedFleetRegistryActivation,
    expected: FleetRegistryActivationPhase,
    requested: FleetRegistryActivationPhase,
    apply: impl FnOnce(&mut FleetRegistryActivationJournal),
) -> Result<ResolvedFleetRegistryActivation, FleetRegistryActivationJournalError> {
    if current.journal.phase != expected {
        return Err(FleetRegistryActivationJournalError::InvalidTransition {
            observed: current.journal.phase,
            requested,
        });
    }
    let _lock = lock_journal(&current.path)?;
    let observed = load_required_journal(&current.path)?;
    if observed != current.journal {
        return Err(invalid(
            &current.path,
            "journal changed before the requested transition",
        ));
    }

    let mut next = observed;
    next.sequence = next
        .sequence
        .checked_add(1)
        .ok_or_else(|| invalid(&current.path, "journal sequence exhausted"))?;
    next.phase = requested;
    apply(&mut next);
    let bytes = encode_journal(&current.path, &next)?;
    replace_bytes_exact(&current.path, &bytes)
        .map_err(|error| replace_error(&current.path, error))?;
    let durable = load_required_journal(&current.path)?;
    if durable != next {
        return Err(invalid(
            &current.path,
            "published transition differs from requested activation journal",
        ));
    }
    Ok(resolved(durable, current.path.clone(), true))
}

fn validate_journal(
    path: &Path,
    journal: &FleetRegistryActivationJournal,
) -> Result<(), FleetRegistryActivationJournalError> {
    if journal.schema_version != JOURNAL_SCHEMA_VERSION {
        return Err(invalid(path, "unsupported journal schema version"));
    }
    FleetRegistryOps::validate(
        &journal.joining_registry.authority,
        &journal.component_topology,
        &journal.joining_registry,
    )
    .map_err(|error| invalid(path, error.to_string()))?;
    let active = FleetRegistryOps::compile_active(
        &journal.joining_registry.authority,
        &journal.component_topology,
        &journal.joining_registry,
    )
    .map_err(|error| invalid(path, error.to_string()))?;
    if active != journal.active_registry {
        return Err(invalid(
            path,
            "active Registry is not the canonical transition target",
        ));
    }
    let expected_joining = FleetRegistryOps::version(
        &journal.joining_registry.authority,
        &journal.component_topology,
        &journal.joining_registry,
    )
    .map_err(|error| invalid(path, error.to_string()))?;
    if journal.request.expected_registry != expected_joining {
        return Err(invalid(
            path,
            "activation request differs from all-Joining authority",
        ));
    }
    validate_phase_evidence(path, journal)?;
    if let Some(response) = &journal.response {
        validate_response(path, journal, response)?;
    }
    if let Some(manifest) = &journal.verified_manifest {
        let expected = FleetRegistryOps::manifest(
            &journal.active_registry.authority,
            &journal.component_topology,
            &journal.active_registry,
        )
        .map_err(|error| invalid(path, error.to_string()))?;
        if manifest != &expected {
            return Err(invalid(path, "verified active Registry manifest differs"));
        }
    }
    if let Some(version) = &journal.verified_version {
        let expected = FleetRegistryOps::version(
            &journal.active_registry.authority,
            &journal.component_topology,
            &journal.active_registry,
        )
        .map_err(|error| invalid(path, error.to_string()))?;
        if version != &expected {
            return Err(invalid(path, "verified active Registry version differs"));
        }
    }
    Ok(())
}

fn validate_phase_evidence(
    path: &Path,
    journal: &FleetRegistryActivationJournal,
) -> Result<(), FleetRegistryActivationJournalError> {
    let expected_sequence = match journal.phase {
        FleetRegistryActivationPhase::Planned => 0,
        FleetRegistryActivationPhase::ActivationInFlight => 1,
        FleetRegistryActivationPhase::Activated => 2,
        FleetRegistryActivationPhase::Verified => 3,
    };
    let expected_evidence = match journal.phase {
        FleetRegistryActivationPhase::Planned
        | FleetRegistryActivationPhase::ActivationInFlight => (false, false, false),
        FleetRegistryActivationPhase::Activated => (true, false, false),
        FleetRegistryActivationPhase::Verified => (true, true, true),
    };
    let observed_evidence = (
        journal.response.is_some(),
        journal.verified_manifest.is_some(),
        journal.verified_version.is_some(),
    );
    if journal.sequence != expected_sequence || observed_evidence != expected_evidence {
        return Err(invalid(
            path,
            "phase differs from activation sequence or retained evidence",
        ));
    }
    Ok(())
}

fn validate_response(
    path: &Path,
    journal: &FleetRegistryActivationJournal,
    response: &FleetRegistryActivationResponse,
) -> Result<(), FleetRegistryActivationJournalError> {
    let version = FleetRegistryOps::version(
        &journal.active_registry.authority,
        &journal.component_topology,
        &journal.active_registry,
    )
    .map_err(|error| invalid(path, error.to_string()))?;
    if response.previous_version != journal.request.expected_registry || response.version != version
    {
        return Err(invalid(
            path,
            "activation response differs from planned source or target authority",
        ));
    }
    Ok(())
}

fn load_required_journal(
    path: &Path,
) -> Result<FleetRegistryActivationJournal, FleetRegistryActivationJournalError> {
    load_optional_journal(path)?.ok_or_else(|| invalid(path, "journal is missing"))
}

fn load_optional_journal(
    path: &Path,
) -> Result<Option<FleetRegistryActivationJournal>, FleetRegistryActivationJournalError> {
    let bytes = match read_optional_bounded_regular_bytes(path, MAX_JOURNAL_BYTES) {
        Ok(bytes) => bytes,
        Err(BoundedRegularFileReadError::TooLarge) => {
            return Err(invalid(path, "journal exceeds its byte bound"));
        }
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::NotRegular)) => {
            return Err(FleetRegistryActivationJournalError::UnsafeFile {
                path: path.to_path_buf(),
            });
        }
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::Io(source))) => {
            return Err(FleetRegistryActivationJournalError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
        #[cfg(not(unix))]
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::UnsupportedPlatform)) => {
            return Err(FleetRegistryActivationJournalError::Io {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Fleet Registry activation journal reads are unsupported",
                ),
            });
        }
    };
    let Some(bytes) = bytes else {
        return Ok(None);
    };
    let journal = serde_json::from_slice::<FleetRegistryActivationJournal>(&bytes)
        .map_err(|error| invalid(path, error.to_string()))?;
    validate_journal(path, &journal)?;
    if encode_journal(path, &journal)? != bytes {
        return Err(invalid(path, "journal bytes are not canonical"));
    }
    Ok(Some(journal))
}

fn encode_journal(
    path: &Path,
    journal: &FleetRegistryActivationJournal,
) -> Result<Vec<u8>, FleetRegistryActivationJournalError> {
    validate_journal(path, journal)?;
    encode_canonical_json(journal, CanonicalJsonStyle::Compact, MAX_JOURNAL_BYTES).map_err(
        |error| match error {
            CanonicalJsonEncodeError::Serialization(error) => invalid(path, error.to_string()),
            CanonicalJsonEncodeError::TooLarge => invalid(path, "journal exceeds its byte bound"),
        },
    )
}

fn replace_error(path: &Path, error: ExactReplaceError) -> FleetRegistryActivationJournalError {
    match error {
        ExactReplaceError::Write(source)
        | ExactReplaceError::Read(RegularFileReadError::Io(source)) => {
            FleetRegistryActivationJournalError::Io {
                path: path.to_path_buf(),
                source,
            }
        }
        ExactReplaceError::Read(RegularFileReadError::NotRegular) => {
            FleetRegistryActivationJournalError::UnsafeFile {
                path: path.to_path_buf(),
            }
        }
        #[cfg(not(unix))]
        ExactReplaceError::Read(RegularFileReadError::UnsupportedPlatform) => {
            FleetRegistryActivationJournalError::Io {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Fleet Registry activation journal reads are unsupported",
                ),
            }
        }
    }
}

fn same_immutable_authority(
    observed: &FleetRegistryActivationJournal,
    expected: &FleetRegistryActivationJournal,
) -> bool {
    observed.schema_version == expected.schema_version
        && observed.fleet_install_plan_digest == expected.fleet_install_plan_digest
        && observed.component_topology == expected.component_topology
        && observed.joining_registry == expected.joining_registry
        && observed.active_registry == expected.active_registry
        && observed.request == expected.request
}

fn journal_path(plan_path: &Path) -> PathBuf {
    plan_path
        .parent()
        .expect("validated Fleet install plan path has an identity directory")
        .join(JOURNAL_FILE)
}

fn lock_journal(path: &Path) -> Result<std::fs::File, FleetRegistryActivationJournalError> {
    let lock_path = path.with_file_name(JOURNAL_LOCK_FILE);
    match lock_regular_file_with_parents(&lock_path) {
        Ok(lock) => Ok(lock),
        Err(RegularFileLockError::NotRegular) => {
            Err(FleetRegistryActivationJournalError::UnsafeLock { path: lock_path })
        }
        Err(RegularFileLockError::Io(source)) => Err(FleetRegistryActivationJournalError::Io {
            path: lock_path,
            source,
        }),
        #[cfg(windows)]
        Err(RegularFileLockError::UnsupportedPlatform) => {
            Err(FleetRegistryActivationJournalError::Io {
                path: lock_path,
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Fleet Registry activation journal locking is unsupported",
                ),
            })
        }
    }
}

const fn resolved(
    journal: FleetRegistryActivationJournal,
    path: PathBuf,
    advanced: bool,
) -> ResolvedFleetRegistryActivation {
    ResolvedFleetRegistryActivation {
        journal,
        path,
        advanced,
    }
}

fn invalid(path: &Path, reason: impl Into<String>) -> FleetRegistryActivationJournalError {
    FleetRegistryActivationJournalError::InvalidDocument {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}
