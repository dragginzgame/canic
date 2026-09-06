//! Module: fleet_ensure::ops::continuation
//!
//! Responsibility: bind fresh continuation inputs and retain immutable phase plans.
//! Boundary: reads release evidence and durable files; never issues a remote effect.

use crate::{
    fleet_ensure::{
        model::{
            DesiredFleet, FleetEnsureContinuationAuthority, FleetEnsureJournalRecord,
            FleetEnsurePlan, FleetEnsureSuccessorPhaseRecord, MAX_FLEET_ENSURE_PROTOCOL_STEPS,
        },
        ops::{EnsurePaths, EnsureStateError, artifact_sha256, is_sha256, read_plan, write_plan},
        policy::expected_plan_sha256,
    },
    release_set::load_persisted_application_artifact_union,
};
use canic_core::{
    cdk::utils::hash::sha256_hex, dto::root_store::ROOT_STORE_RELEASE_SET_MANIFEST_MAX_BYTES,
};
use std::path::Path;

pub(super) fn resolve_authority(
    root: &Path,
    desired: &DesiredFleet,
) -> Result<Option<FleetEnsureContinuationAuthority>, EnsureStateError> {
    let Some(bootstrap) = desired.bootstrap.as_ref() else {
        return Ok(None);
    };
    let Some(protocol) = &desired.protocol else {
        return Ok(None);
    };
    let persisted = load_persisted_application_artifact_union(
        root,
        &bootstrap
            .component_deployment_configuration
            .component_topology,
        bootstrap.release_build_id,
    )
    .map_err(|error| invalid(error.to_string()))?;
    let union_bytes =
        serde_json::to_vec(&persisted.union).map_err(|error| invalid(error.to_string()))?;
    let chunk_bytes = u64::try_from(canic_core::CANIC_WASM_CHUNK_BYTES)
        .map_err(|error| invalid(error.to_string()))?;
    // Each role needs one manifest, one chunk-set preparation and its exact chunks.
    let artifact_steps = persisted
        .union
        .entries
        .iter()
        .try_fold(0_u64, |total, entry| {
            total
                .checked_add(2 + entry.wasm_gz_size_bytes.div_ceil(chunk_bytes))
                .ok_or_else(|| invalid("artifact step bound overflow"))
        })?;
    // The release manifest has a protocol-owned size limit. Root adoption/bootstrap,
    // joining, synchronization, activation, Component preparation and readiness follow.
    let per_root = artifact_steps
        .checked_add(ROOT_STORE_RELEASE_SET_MANIFEST_MAX_BYTES.div_ceil(chunk_bytes) + 8)
        .ok_or_else(|| invalid("Root step bound overflow"))?;
    let roots = u64::try_from(bootstrap.roots.len()).map_err(|error| invalid(error.to_string()))?;
    let imports = bootstrap.roots.iter().try_fold(0_u64, |total, input| {
        total
            .checked_add(input.canister_pool_imports.len() as u64)
            .ok_or_else(|| invalid("import step bound overflow"))
    })?;
    let maximum = per_root
        .checked_mul(roots)
        .and_then(|total| total.checked_add(imports))
        .and_then(|total| total.checked_add(2))
        .ok_or_else(|| invalid("successor step bound overflow"))?;
    if maximum == 0 || maximum > MAX_FLEET_ENSURE_PROTOCOL_STEPS as u64 {
        return Err(invalid(
            "fresh protocol exceeds the bounded successor catalogue",
        ));
    }
    Ok(Some(FleetEnsureContinuationAuthority {
        app_config_sha256: artifact_sha256(root, &protocol.app_config)?,
        application_artifact_union_sha256: sha256_hex(&union_bytes),
        coordinator_candid_sha256: artifact_sha256(root, &protocol.coordinator_candid)?,
        maximum_successor_actions: u32::try_from(maximum)
            .map_err(|error| invalid(error.to_string()))?,
        root_candid_sha256: artifact_sha256(root, &protocol.root_candid)?,
        store_candid_sha256: artifact_sha256(root, &protocol.store_candid)?,
    }))
}

/// Construct the candidate durable record before workflow validates its full authority.
pub(in crate::fleet_ensure) fn candidate_journal(
    journal: &FleetEnsureJournalRecord,
    phase: &FleetEnsurePlan,
    execution_burn_before_phase: u128,
) -> FleetEnsureJournalRecord {
    let mut candidate = journal.clone();
    candidate
        .successor_phases
        .push(FleetEnsureSuccessorPhaseRecord {
            execution_burn_before_phase,
            plan_sha256: phase.plan_sha256.clone(),
            plan: Some(Box::new(phase.clone())),
        });
    candidate
}

pub(in crate::fleet_ensure) fn retain_phase(
    paths: &EnsurePaths,
    phase: &FleetEnsurePlan,
) -> Result<(), EnsureStateError> {
    if phase.plan_sha256 != expected_plan_sha256(phase) {
        return Err(invalid("successor plan digest differs"));
    }
    let paths = phase_paths(paths, &phase.plan_sha256)?;
    if let Some(retained) = read_plan(&paths)? {
        if retained != *phase {
            return Err(invalid("immutable successor plan differs"));
        }
        return Ok(());
    }
    write_plan(&paths, phase)
}

pub(super) fn hydrate_phases(
    paths: &EnsurePaths,
    journal: &mut FleetEnsureJournalRecord,
) -> Result<(), EnsureStateError> {
    if journal.successor_phases.len() > MAX_FLEET_ENSURE_PROTOCOL_STEPS {
        return Err(invalid("too many retained successor phases"));
    }
    for phase in &mut journal.successor_phases {
        let paths = phase_paths(paths, &phase.plan_sha256)?;
        let plan = read_plan(&paths)?.ok_or_else(|| invalid("successor plan is missing"))?;
        if plan.plan_sha256 != phase.plan_sha256 || expected_plan_sha256(&plan) != phase.plan_sha256
        {
            return Err(invalid("retained successor plan digest differs"));
        }
        phase.plan = Some(Box::new(plan));
    }
    Ok(())
}

fn phase_paths(paths: &EnsurePaths, digest: &str) -> Result<EnsurePaths, EnsureStateError> {
    if !is_sha256(digest) {
        return Err(invalid("successor digest is not SHA-256"));
    }
    let mut phase = paths.clone();
    phase.plan = paths
        .plan
        .with_file_name("phases")
        .join(format!("{digest}.json"));
    Ok(phase)
}

fn invalid(reason: impl Into<String>) -> EnsureStateError {
    EnsureStateError::ContinuationAuthority {
        reason: reason.into(),
    }
}
