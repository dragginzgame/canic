//! Module: fleet_ensure::ops::continuation
//!
//! Responsibility: bind fresh continuation inputs and retain immutable phase plans.
//! Boundary: reads release evidence and durable files; never issues a remote effect.

use crate::{
    fleet_ensure::{
        model::{
            DesiredFleet, FleetEnsureContinuationAuthority, FleetEnsureJournalRecord,
            FleetEnsurePlan, FleetEnsurePlanScope, FleetEnsureSuccessorPhaseRecord,
            MAX_FLEET_ENSURE_PROTOCOL_STEPS, StartupFundingRequirement,
        },
        ops::{EnsurePaths, EnsureStateError, artifact_sha256, is_sha256, read_plan, write_plan},
        policy::expected_plan_sha256,
    },
    release_set::{
        CurrentReleaseSetManifestError, load_persisted_application_artifact_union,
        load_persisted_current_release_set_manifest,
    },
};
use canic_core::{
    cdk::utils::hash::sha256_hex, dto::root_store::ROOT_STORE_RELEASE_SET_MANIFEST_MAX_BYTES,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// Reject unsupported current-release policy before any paid platform observation.
pub(in crate::fleet_ensure) fn verify_release_transition(
    root: &Path,
    desired: &DesiredFleet,
    scope: FleetEnsurePlanScope,
) -> Result<(), EnsureStateError> {
    let (Some(bootstrap), Some(_)) = (&desired.bootstrap, &desired.protocol) else {
        return Ok(());
    };
    match load_persisted_current_release_set_manifest(root, bootstrap.release_build_id) {
        Ok(_) => Ok(()),
        // Starting an exactly retained installed Root does not consume a selected
        // application release. Missing build inputs cannot strand that recovery.
        Err(CurrentReleaseSetManifestError::Missing(_))
            if scope == FleetEnsurePlanScope::RootStartPrerequisite =>
        {
            Ok(())
        }
        Err(error) => Err(invalid(error.to_string())),
    }
}

pub(super) fn resolve_authority(
    root: &Path,
    desired: &DesiredFleet,
    startup: &mut BTreeMap<String, StartupFundingRequirement>,
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
    let fixture_steps = fixture_publication_steps(root, desired)?;
    let per_root = artifact_steps
        .checked_add(fixture_steps)
        .and_then(|total| {
            total.checked_add(ROOT_STORE_RELEASE_SET_MANIFEST_MAX_BYTES.div_ceil(chunk_bytes) + 8)
        })
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
    let fixture_publication_retry_attempts = fixture_steps
        .checked_mul(roots)
        .and_then(|steps| {
            steps.checked_mul(u64::from(
                desired.maximum_stalled_observations.saturating_sub(1),
            ))
        })
        .and_then(|attempts| u32::try_from(attempts).ok())
        .ok_or_else(|| invalid("fixture publication retry bound overflow"))?;
    bind_startup_steps(desired, startup, per_root, fixture_steps)?;
    Ok(Some(FleetEnsureContinuationAuthority {
        fixture_publication_retry_attempts,
        app_config_sha256: artifact_sha256(root, &protocol.app_config)?,
        application_artifact_union_sha256: sha256_hex(&union_bytes),
        coordinator_candid_sha256: artifact_sha256(root, &protocol.coordinator_candid)?,
        maximum_successor_actions: u32::try_from(maximum)
            .map_err(|error| invalid(error.to_string()))?,
        root_candid_sha256: artifact_sha256(root, &protocol.root_candid)?,
        store_candid_sha256: artifact_sha256(root, &protocol.store_candid)?,
    }))
}

/// Attribute each Root's own bounded catalogue, including its imports and shared orchestration.
fn bind_startup_steps(
    desired: &DesiredFleet,
    startup: &mut BTreeMap<String, StartupFundingRequirement>,
    per_root: u64,
    fixture_steps: u64,
) -> Result<(), EnsureStateError> {
    let Some(bootstrap) = &desired.bootstrap else {
        return Ok(());
    };
    let retries = fixture_steps
        .checked_mul(u64::from(
            desired.maximum_stalled_observations.saturating_sub(1),
        ))
        .ok_or_else(|| invalid("Root retry step bound overflow"))?;
    for root in &bootstrap.roots {
        let requirement = startup
            .get_mut(&root.root)
            .ok_or_else(|| invalid("Root startup requirement is missing"))?;
        // Store publication is included conservatively, but other Roots' catalogues are not.
        requirement.maximum_continuation_steps = per_root
            .checked_add(root.canister_pool_imports.len() as u64)
            .and_then(|steps| steps.checked_add(2))
            .and_then(|steps| steps.checked_add(retries))
            .and_then(|steps| u32::try_from(steps).ok())
            .ok_or_else(|| invalid("Root startup step bound overflow"))?;
    }
    Ok(())
}

fn fixture_publication_steps(root: &Path, desired: &DesiredFleet) -> Result<u64, EnsureStateError> {
    let bootstrap = desired
        .bootstrap
        .as_ref()
        .ok_or_else(|| invalid("missing bootstrap authority"))?;
    let complete = load_persisted_current_release_set_manifest(root, bootstrap.release_build_id)
        .map_err(|error| invalid(error.to_string()))?;
    let fixtures = complete
        .manifest
        .verify_fixtures(
            root,
            &bootstrap
                .component_deployment_configuration
                .component_topology,
        )
        .map_err(|error| invalid(error.to_string()))?;
    let mut contents = BTreeSet::new();
    fixtures
        .manifest
        .entries
        .iter()
        .try_fold(0_u64, |total, entry| {
            if !contents.insert(entry.content_id) {
                return Ok(total);
            }
            let chunks = u64::try_from(entry.descriptor.chunks.len())
                .map_err(|error| invalid(error.to_string()))?;
            total
                .checked_add(1)
                .and_then(|total| total.checked_add(chunks))
                .ok_or_else(|| invalid("fixture publication step bound overflow"))
        })
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
    phases: &mut [FleetEnsureSuccessorPhaseRecord],
) -> Result<(), EnsureStateError> {
    if phases.len() > MAX_FLEET_ENSURE_PROTOCOL_STEPS {
        return Err(invalid("too many retained successor phases"));
    }
    for phase in phases {
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
