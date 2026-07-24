//! Module: install_root::fleet_activation_journal
//!
//! Responsibility: own canonical host recovery evidence for one fresh Fleet activation.
//! Does not own: Canister activation, catalog publication, or release-build construction.
//! Boundary: a journal is admitted only from finalized release-build evidence and is durable
//! before any Canister mutation.

#![expect(
    dead_code,
    reason = "the journal authority is staged and tested before live install mutation is admitted"
)]

#[cfg(test)]
mod tests;

use super::state::validate_state_name;
use crate::{
    durable_io::{
        RegularFileReadError, create_new_bytes_with_parents, read_optional_regular_bytes,
    },
    entropy::{EntropyError, random_bytes_32},
    release_build::{
        FinalizedReleaseBuild, ReleaseBuildPlanError, ReleaseBuildPlanState,
        load_finalized_release_build,
    },
};
use canic_core::{
    dto::fleet_activation::{FleetActivationHostRecord, FleetActivationIdentity},
    ids::{AppId, CanonicalNetworkId, FleetBinding, FleetId, FleetKey, FleetName},
};
use ciborium::Value;
use sha2::{Digest, Sha256};
use std::{
    io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const JOURNAL_HASH_DOMAIN: &[u8] = b"canic:fleet-install:activation-journal\0";
const RANDOM_ATTEMPTS: usize = 16;

///
/// FleetInstallActivationPhase
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FleetInstallActivationPhase {
    Planned,
    RootInstalled,
    CanistersPrepared,
    CanistersActivated,
    HostAuthorityCommitted,
}

///
/// FleetInstallActivationJournal
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct FleetInstallActivationJournal {
    pub sequence: u64,
    pub phase: FleetInstallActivationPhase,
    pub fleet_name: FleetName,
    pub release_build_plan_hash: [u8; 32],
    pub release_set_manifest_digest: [u8; 32],
    pub root_install_receipt_hash: Option<[u8; 32]>,
    pub activation: FleetActivationHostRecord,
    pub committed_fleet_catalog_hash: Option<[u8; 32]>,
}

///
/// PlannedFleetInstallActivation
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PlannedFleetInstallActivation {
    pub journal: FleetInstallActivationJournal,
    pub journal_hash: [u8; 32],
    pub path: PathBuf,
}

///
/// PlanFleetInstallActivationRequest
///

pub(super) struct PlanFleetInstallActivationRequest<'a> {
    pub root: &'a Path,
    pub canonical_network_id: CanonicalNetworkId,
    pub fleet_name: FleetName,
    pub app: AppId,
    pub finalized_release_build: &'a FinalizedReleaseBuild,
}

///
/// FleetInstallActivationJournalError
///

#[derive(Debug, ThisError)]
pub(super) enum FleetInstallActivationJournalError {
    #[error("failed to access Fleet install activation journal {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Fleet install activation journal is not a regular no-follow file: {path}")]
    UnsafeFile { path: PathBuf },

    #[error("Fleet install activation journal is missing: {path}")]
    Missing { path: PathBuf },

    #[error("invalid Fleet install activation journal {path}: {reason}")]
    InvalidDocument { path: PathBuf, reason: String },

    #[error("source App identity {app:?} is invalid: {reason}")]
    InvalidApp { app: String, reason: String },

    #[error("finalized release-build evidence changed before Fleet activation planning")]
    FinalizedReleaseBuildMismatch,

    #[error("cryptographic random source returned only {actual} of 32 required bytes")]
    ShortRandomRead { actual: usize },

    #[error(
        "could not allocate a unique Fleet activation identity after {RANDOM_ATTEMPTS} attempts"
    )]
    IdentityAllocationExhausted,

    #[error(transparent)]
    ReleaseBuild(#[from] ReleaseBuildPlanError),
}

/// Create and durably publish one new `Planned` activation journal.
pub(super) fn plan_fleet_install_activation(
    request: PlanFleetInstallActivationRequest<'_>,
) -> Result<PlannedFleetInstallActivation, FleetInstallActivationJournalError> {
    validate_app(&request.app)?;
    let finalized = load_finalized_release_build(
        request.root,
        request.finalized_release_build.record.release_build_id,
    )?;
    if finalized.record != request.finalized_release_build.record
        || finalized.plan_hash != request.finalized_release_build.plan_hash
    {
        return Err(FleetInstallActivationJournalError::FinalizedReleaseBuildMismatch);
    }
    let ReleaseBuildPlanState::Finalized {
        release_set_manifest_digest,
    } = finalized.record.state
    else {
        unreachable!("load_finalized_release_build admits only finalized records");
    };

    for _ in 0..RANDOM_ATTEMPTS {
        let fleet_id = FleetId::from_generated_bytes(random_identity_bytes()?);
        let operation_id = random_identity_bytes()?;
        match plan_fleet_install_activation_with_ids(
            &request,
            &finalized,
            release_set_manifest_digest,
            fleet_id,
            operation_id,
        ) {
            Ok(planned) => return Ok(planned),
            Err(FleetInstallActivationJournalError::Io { source, .. })
                if source.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }

    Err(FleetInstallActivationJournalError::IdentityAllocationExhausted)
}

/// Load one exact journal and verify every path identity component.
pub(super) fn load_fleet_install_activation_journal(
    root: &Path,
    canonical_network_id: CanonicalNetworkId,
    fleet_id: FleetId,
    operation_id: [u8; 32],
) -> Result<FleetInstallActivationJournal, FleetInstallActivationJournalError> {
    let path =
        fleet_install_activation_journal_path(root, canonical_network_id, fleet_id, operation_id);
    let bytes = read_journal_bytes(&path)?;
    let journal = decode_journal(&path, &bytes)?;
    if journal.activation.identity.fleet.fleet.network != canonical_network_id {
        return Err(invalid(
            &path,
            "path canonical network does not match activation identity",
        ));
    }
    if journal.activation.identity.fleet.fleet.fleet_id != fleet_id {
        return Err(invalid(
            &path,
            "path Fleet ID does not match activation identity",
        ));
    }
    if journal.activation.identity.operation_id != operation_id {
        return Err(invalid(
            &path,
            "path operation ID does not match activation identity",
        ));
    }
    Ok(journal)
}

/// Return the canonical path for one fresh-install activation journal.
#[must_use]
pub(super) fn fleet_install_activation_journal_path(
    root: &Path,
    canonical_network_id: CanonicalNetworkId,
    fleet_id: FleetId,
    operation_id: [u8; 32],
) -> PathBuf {
    root.join(".canic")
        .join("recovery")
        .join("fleet-install-activations")
        .join(canonical_network_id.to_string())
        .join(fleet_id.to_string())
        .join(hex_digest(operation_id))
        .join("journal.cbor")
}

/// Hash one exact journal under the frozen activation-journal separator.
#[must_use]
pub(super) fn fleet_install_activation_journal_hash(
    journal: &FleetInstallActivationJournal,
) -> [u8; 32] {
    let bytes = encode_journal(journal)
        .expect("an admitted activation journal must retain canonical encodable fields");
    domain_hash(JOURNAL_HASH_DOMAIN, &bytes)
}

fn plan_fleet_install_activation_with_ids(
    request: &PlanFleetInstallActivationRequest<'_>,
    finalized_release_build: &FinalizedReleaseBuild,
    release_set_manifest_digest: [u8; 32],
    fleet_id: FleetId,
    operation_id: [u8; 32],
) -> Result<PlannedFleetInstallActivation, FleetInstallActivationJournalError> {
    let canonical_network_id = request.canonical_network_id;
    let journal = FleetInstallActivationJournal {
        sequence: 0,
        phase: FleetInstallActivationPhase::Planned,
        fleet_name: request.fleet_name.clone(),
        release_build_plan_hash: finalized_release_build.plan_hash,
        release_set_manifest_digest,
        root_install_receipt_hash: None,
        activation: FleetActivationHostRecord {
            identity: FleetActivationIdentity {
                fleet: FleetBinding {
                    fleet: FleetKey {
                        network: canonical_network_id,
                        fleet_id,
                    },
                    app: request.app.clone(),
                },
                operation_id,
                release_build_id: finalized_release_build.record.release_build_id,
            },
            cascade_manifest: None,
            credential: None,
            credential_manifest: None,
            canisters: Vec::new(),
        },
        committed_fleet_catalog_hash: None,
    };
    let path = fleet_install_activation_journal_path(
        request.root,
        canonical_network_id,
        fleet_id,
        operation_id,
    );
    let bytes = encode_journal(&journal)?;
    create_new_bytes_with_parents(&path, &bytes).map_err(|source| {
        FleetInstallActivationJournalError::Io {
            path: path.clone(),
            source,
        }
    })?;
    let observed = load_fleet_install_activation_journal(
        request.root,
        canonical_network_id,
        fleet_id,
        operation_id,
    )?;
    if observed != journal {
        return Err(invalid(
            &path,
            "published journal differs from the planned record",
        ));
    }
    let journal_hash = fleet_install_activation_journal_hash(&journal);
    Ok(PlannedFleetInstallActivation {
        journal,
        journal_hash,
        path,
    })
}

fn encode_journal(
    journal: &FleetInstallActivationJournal,
) -> Result<Vec<u8>, FleetInstallActivationJournalError> {
    let path = Path::new("<candidate Fleet install activation journal>");
    validate_planned_journal(path, journal)?;
    Ok(encode_value(&Value::Array(vec![
        integer(journal.sequence),
        integer(phase_discriminant(journal.phase)),
        Value::Text(journal.fleet_name.to_string()),
        digest(journal.release_build_plan_hash),
        digest(journal.release_set_manifest_digest),
        Value::Null,
        encode_planned_activation(&journal.activation),
        Value::Null,
    ])))
}

fn decode_journal(
    path: &Path,
    bytes: &[u8],
) -> Result<FleetInstallActivationJournal, FleetInstallActivationJournalError> {
    let value: Value =
        ciborium::de::from_reader(bytes).map_err(|error| invalid(path, error.to_string()))?;
    let fields = exact_array(path, value, 8, "journal")?;
    let journal = FleetInstallActivationJournal {
        sequence: exact_u64(path, &fields[0], "sequence")?,
        phase: decode_phase(path, exact_u64(path, &fields[1], "phase")?)?,
        fleet_name: exact_text(path, &fields[2], "fleet_name")?
            .parse()
            .map_err(|error| invalid(path, format!("invalid fleet_name: {error}")))?,
        release_build_plan_hash: exact_digest(path, &fields[3], "release_build_plan_hash")?,
        release_set_manifest_digest: exact_digest(path, &fields[4], "release_set_manifest_digest")?,
        root_install_receipt_hash: exact_null(path, &fields[5], "root_install_receipt_hash")?,
        activation: decode_planned_activation(path, &fields[6])?,
        committed_fleet_catalog_hash: exact_null(path, &fields[7], "committed_fleet_catalog_hash")?,
    };
    validate_planned_journal(path, &journal)?;
    if encode_journal(&journal)? != bytes {
        return Err(invalid(path, "CBOR bytes are not canonical"));
    }
    Ok(journal)
}

fn validate_planned_journal(
    path: &Path,
    journal: &FleetInstallActivationJournal,
) -> Result<(), FleetInstallActivationJournalError> {
    validate_app(&journal.activation.identity.fleet.app)?;
    if journal.phase != FleetInstallActivationPhase::Planned {
        return Err(invalid(
            path,
            "only the Planned phase is admitted by the current implementation",
        ));
    }
    if journal.sequence != 0 {
        return Err(invalid(path, "Planned phase requires sequence 0"));
    }
    if journal.root_install_receipt_hash.is_some()
        || journal.committed_fleet_catalog_hash.is_some()
        || journal.activation.cascade_manifest.is_some()
        || journal.activation.credential.is_some()
        || journal.activation.credential_manifest.is_some()
        || !journal.activation.canisters.is_empty()
    {
        return Err(invalid(
            path,
            "Planned phase contains evidence legal only after Canister mutation",
        ));
    }
    Ok(())
}

fn encode_planned_activation(record: &FleetActivationHostRecord) -> Value {
    Value::Array(vec![
        encode_activation_identity(&record.identity),
        Value::Null,
        Value::Null,
        Value::Null,
        Value::Array(Vec::new()),
    ])
}

fn decode_planned_activation(
    path: &Path,
    value: &Value,
) -> Result<FleetActivationHostRecord, FleetInstallActivationJournalError> {
    let fields = exact_array_ref(path, value, 5, "activation")?;
    let _: Option<[u8; 32]> = exact_null(path, &fields[1], "cascade_manifest")?;
    let _: Option<[u8; 32]> = exact_null(path, &fields[2], "credential")?;
    let _: Option<[u8; 32]> = exact_null(path, &fields[3], "credential_manifest")?;
    if !exact_array_ref(path, &fields[4], 0, "canisters")?.is_empty() {
        unreachable!("exact zero-length array was validated");
    }
    Ok(FleetActivationHostRecord {
        identity: decode_activation_identity(path, &fields[0])?,
        cascade_manifest: None,
        credential: None,
        credential_manifest: None,
        canisters: Vec::new(),
    })
}

fn encode_activation_identity(identity: &FleetActivationIdentity) -> Value {
    Value::Array(vec![
        Value::Array(vec![
            Value::Array(vec![
                digest(*identity.fleet.fleet.network.as_bytes()),
                digest(*identity.fleet.fleet.fleet_id.as_bytes()),
            ]),
            Value::Text(identity.fleet.app.to_string()),
        ]),
        digest(identity.operation_id),
        digest(*identity.release_build_id.as_bytes()),
    ])
}

fn decode_activation_identity(
    path: &Path,
    value: &Value,
) -> Result<FleetActivationIdentity, FleetInstallActivationJournalError> {
    let fields = exact_array_ref(path, value, 3, "activation identity")?;
    let binding = exact_array_ref(path, &fields[0], 2, "Fleet binding")?;
    let key = exact_array_ref(path, &binding[0], 2, "Fleet key")?;
    let app = AppId::owned(exact_text(path, &binding[1], "app")?.to_string());
    validate_app(&app)?;
    Ok(FleetActivationIdentity {
        fleet: FleetBinding {
            fleet: FleetKey {
                network: id_from_digest(
                    exact_digest(path, &key[0], "canonical_network_id")?,
                    "canonical_network_id",
                    path,
                )?,
                fleet_id: FleetId::from_generated_bytes(exact_digest(path, &key[1], "fleet_id")?),
            },
            app,
        },
        operation_id: exact_digest(path, &fields[1], "operation_id")?,
        release_build_id: id_from_digest(
            exact_digest(path, &fields[2], "release_build_id")?,
            "release_build_id",
            path,
        )?,
    })
}

fn decode_phase(
    path: &Path,
    discriminant: u64,
) -> Result<FleetInstallActivationPhase, FleetInstallActivationJournalError> {
    match discriminant {
        0 => Ok(FleetInstallActivationPhase::Planned),
        1 => Ok(FleetInstallActivationPhase::RootInstalled),
        2 => Ok(FleetInstallActivationPhase::CanistersPrepared),
        3 => Ok(FleetInstallActivationPhase::CanistersActivated),
        4 => Ok(FleetInstallActivationPhase::HostAuthorityCommitted),
        _ => Err(invalid(path, "phase has an unknown discriminant")),
    }
}

const fn phase_discriminant(phase: FleetInstallActivationPhase) -> u64 {
    match phase {
        FleetInstallActivationPhase::Planned => 0,
        FleetInstallActivationPhase::RootInstalled => 1,
        FleetInstallActivationPhase::CanistersPrepared => 2,
        FleetInstallActivationPhase::CanistersActivated => 3,
        FleetInstallActivationPhase::HostAuthorityCommitted => 4,
    }
}

fn read_journal_bytes(path: &Path) -> Result<Vec<u8>, FleetInstallActivationJournalError> {
    match read_optional_regular_bytes(path) {
        Ok(Some(bytes)) => Ok(bytes),
        Ok(None) => Err(FleetInstallActivationJournalError::Missing {
            path: path.to_path_buf(),
        }),
        Err(RegularFileReadError::NotRegular) => {
            Err(FleetInstallActivationJournalError::UnsafeFile {
                path: path.to_path_buf(),
            })
        }
        Err(RegularFileReadError::Io(source)) => Err(FleetInstallActivationJournalError::Io {
            path: path.to_path_buf(),
            source,
        }),
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            Err(FleetInstallActivationJournalError::Io {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "no-follow activation-journal reads are unsupported on this platform",
                ),
            })
        }
    }
}

fn validate_app(app: &AppId) -> Result<(), FleetInstallActivationJournalError> {
    validate_state_name(app.as_str()).map_err(|error| {
        FleetInstallActivationJournalError::InvalidApp {
            app: app.to_string(),
            reason: error.to_string(),
        }
    })
}

fn random_identity_bytes() -> Result<[u8; 32], FleetInstallActivationJournalError> {
    random_bytes_32().map_err(|error| match error {
        EntropyError::Io(source) => FleetInstallActivationJournalError::Io {
            path: PathBuf::from("<operating-system random source>"),
            source,
        },
        EntropyError::ShortRead { actual } => {
            FleetInstallActivationJournalError::ShortRandomRead { actual }
        }
    })
}

fn exact_array(
    path: &Path,
    value: Value,
    len: usize,
    field: &str,
) -> Result<Vec<Value>, FleetInstallActivationJournalError> {
    let Value::Array(values) = value else {
        return Err(invalid(path, format!("{field} must be an array")));
    };
    if values.len() != len {
        return Err(invalid(
            path,
            format!("{field} must contain exactly {len} values"),
        ));
    }
    Ok(values)
}

fn exact_array_ref<'a>(
    path: &Path,
    value: &'a Value,
    len: usize,
    field: &str,
) -> Result<&'a [Value], FleetInstallActivationJournalError> {
    let Value::Array(values) = value else {
        return Err(invalid(path, format!("{field} must be an array")));
    };
    if values.len() != len {
        return Err(invalid(
            path,
            format!("{field} must contain exactly {len} values"),
        ));
    }
    Ok(values)
}

fn exact_u64(
    path: &Path,
    value: &Value,
    field: &str,
) -> Result<u64, FleetInstallActivationJournalError> {
    value
        .as_integer()
        .and_then(|value| u64::try_from(value).ok())
        .ok_or_else(|| invalid(path, format!("{field} must be an unsigned integer")))
}

fn exact_text<'a>(
    path: &Path,
    value: &'a Value,
    field: &str,
) -> Result<&'a str, FleetInstallActivationJournalError> {
    value
        .as_text()
        .ok_or_else(|| invalid(path, format!("{field} must be text")))
}

fn exact_digest(
    path: &Path,
    value: &Value,
    field: &str,
) -> Result<[u8; 32], FleetInstallActivationJournalError> {
    let Value::Bytes(bytes) = value else {
        return Err(invalid(path, format!("{field} must be a byte string")));
    };
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| invalid(path, format!("{field} must contain exactly 32 bytes")))
}

fn exact_null<T>(
    path: &Path,
    value: &Value,
    field: &str,
) -> Result<Option<T>, FleetInstallActivationJournalError> {
    if matches!(value, Value::Null) {
        Ok(None)
    } else {
        Err(invalid(path, format!("{field} must be null in Planned")))
    }
}

fn id_from_digest<T>(
    bytes: [u8; 32],
    field: &str,
    path: &Path,
) -> Result<T, FleetInstallActivationJournalError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    hex_digest(bytes)
        .parse()
        .map_err(|error| invalid(path, format!("{field} is invalid: {error}")))
}

fn encode_value(value: &Value) -> Vec<u8> {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(value, &mut bytes)
        .expect("serializing a validated activation-journal CBOR value cannot fail");
    bytes
}

fn integer(value: u64) -> Value {
    Value::Integer(value.into())
}

fn digest(value: [u8; 32]) -> Value {
    Value::Bytes(value.to_vec())
}

fn hex_digest(bytes: [u8; 32]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(64), |mut text, byte| {
            use std::fmt::Write as _;
            write!(text, "{byte:02x}").expect("writing to String cannot fail");
            text
        })
}

fn domain_hash(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(
        u64::try_from(bytes.len())
            .expect("host evidence fits u64")
            .to_be_bytes(),
    );
    hasher.update(bytes);
    hasher.finalize().into()
}

fn invalid(path: &Path, reason: impl Into<String>) -> FleetInstallActivationJournalError {
    FleetInstallActivationJournalError::InvalidDocument {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}
