//! Read retained authority, collect independent replies and project bounded views.

pub mod presentation;
pub mod transport;

use crate::{
    fleet_ensure::{CurrentFleetResolution, policy::expected_plan_sha256, resolve_current_fleet},
    network::resolve_canonical_network_id_from_root,
    observatory::{ObservatoryError, model::ObservatoryOptions, view::*},
    registry::RegistryEntry,
};
use std::{
    path::Path,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use transport::ObservatoryTransport;

/// Capture validated retained authority without contacting or changing a canister.
pub fn authority(
    root: &Path,
    options: &ObservatoryOptions,
) -> Result<CurrentFleetResolution, ObservationFailure> {
    let current = resolve_current_fleet(root, &options.environment, &options.fleet)
        .map_err(|_| ObservationFailure::AuthorityUnavailable)?;
    let registry = current
        .initial_active_registry(&options.fleet)
        .map_err(|_| ObservationFailure::AuthorityUnavailable)?;
    let network = resolve_canonical_network_id_from_root(root, &options.environment)
        .map_err(|_| ObservationFailure::AuthorityUnavailable)?;
    if expected_plan_sha256(&current.plan) != current.plan.plan_sha256
        || registry.authority.binding.fleet.fleet.canonical_network_id != network
    {
        return Err(ObservationFailure::AuthorityUnavailable);
    }
    if current.registry.entries.len() > options.maximum_canisters {
        return Err(ObservationFailure::BudgetExceeded);
    }
    Ok(current)
}

/// Collect each selector independently; failure never erases another selector's result.
pub fn collect(
    source: Result<&CurrentFleetResolution, ObservationFailure>,
    options: &ObservatoryOptions,
    transport: &mut impl ObservatoryTransport,
) -> Result<ObservatorySnapshotView, ObservatoryError> {
    let started = Instant::now();
    let now = now_ms();
    let attempts = transport.attempts();
    let mut snapshot = ObservatorySnapshotView {
        schema_version: 1,
        environment: options.environment.clone(),
        fleet: options.fleet.clone(),
        collected_at_unix_ms: now,
        freshness_secs: options.freshness_secs,
        collection_elapsed_ms: 0,
        remote_call_attempts: 0,
        authority: unavailable(now, ObservationFailure::AuthorityUnavailable),
        operation: unavailable(now, ObservationFailure::AuthorityUnavailable),
        roles: Vec::new(),
    };
    match source {
        Ok(source) => {
            if source.registry.entries.len() > options.maximum_canisters {
                return Err(ObservatoryError::Bound("canister selection"));
            }
            let registry = source
                .initial_active_registry(&options.fleet)
                .map_err(|_| ObservatoryError::AuthorityChanged)?;
            snapshot.authority = observed(
                now,
                ObservationSource::RetainedTerminalReview,
                ObservatoryAuthorityView {
                    app: registry.authority.binding.fleet.app.to_string(),
                    canonical_network_id: registry
                        .authority
                        .binding
                        .fleet
                        .fleet
                        .canonical_network_id
                        .to_string(),
                    plan_sha256: source.plan.plan_sha256.clone(),
                    registry_revision: registry.revision,
                    admission_principals: registry.admission.fleet_principals.len(),
                },
            );
            for entry in &source.registry.entries {
                let mut role = collect_role(entry, transport);
                role.subnet_id = placement(source, entry);
                snapshot.roles.push(role);
            }
        }
        Err(failure) => snapshot.authority = unavailable(now, failure),
    }
    snapshot.collection_elapsed_ms =
        u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    snapshot.remote_call_attempts = transport.attempts().saturating_sub(attempts);
    Ok(snapshot)
}

pub(super) fn collect_role(
    entry: &RegistryEntry,
    transport: &mut impl ObservatoryTransport,
) -> ObservatoryRoleView {
    ObservatoryRoleView {
        role: entry.role.clone().unwrap_or_else(|| "unknown".into()),
        canister_id: entry.pid.clone(),
        parent_canister_id: entry.parent_pid.clone(),
        subnet_id: None,
        release_identity: entry
            .protocol_binding
            .as_ref()
            .map(|binding| binding.release_identity.clone()),
        expected_module_sha256: entry.module_hash.clone(),
        overview: outcome(
            transport.overview(entry),
            ObservationSource::PublicRoleOverview,
        ),
        funding: outcome(
            transport.funding(entry),
            ObservationSource::ProtectedRoleStatus,
        ),
        estate: outcome(
            transport.estate(entry),
            ObservationSource::ProtectedRoleStatus,
        ),
        store: outcome(
            transport.store(entry),
            ObservationSource::ProtectedRoleStatus,
        ),
    }
}

fn placement(source: &CurrentFleetResolution, entry: &RegistryEntry) -> Option<String> {
    let registry = source.active_registry.as_ref()?;
    let mut current = entry;
    let mut visited = std::collections::BTreeSet::new();
    while visited.insert(current.pid.as_str()) {
        if let Some(root) = registry
            .fleet_subnet_roots
            .iter()
            .find(|root| root.fleet_subnet_root.to_text() == current.pid)
        {
            return Some(root.placement_subnet.to_string());
        }
        current = source
            .registry
            .entries
            .iter()
            .find(|candidate| Some(&candidate.pid) == current.parent_pid.as_ref())?;
    }
    None
}

fn outcome<T>(result: Result<T, ObservationFailure>, source: ObservationSource) -> Observation<T> {
    let now = now_ms();
    match result {
        Ok(value) => observed(now, source, value),
        Err(failure) => unavailable(now, failure),
    }
}

const fn observed<T>(now: u64, source: ObservationSource, value: T) -> Observation<T> {
    Observation::Observed {
        observed_at_unix_ms: now,
        source,
        value,
    }
}

const fn unavailable<T>(now: u64, failure: ObservationFailure) -> Observation<T> {
    Observation::Unavailable {
        observed_at_unix_ms: now,
        failure,
    }
}

/// Host wall time accompanies each reply; callers decide freshness at presentation time.
#[must_use]
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|time| u64::try_from(time.as_millis()).ok())
        .unwrap_or(0)
}

/// Attach local progress even when nonterminal authority correctly prevents remote discovery.
pub fn observe_local_operation(
    root: &Path,
    options: &ObservatoryOptions,
    snapshot: &mut ObservatorySnapshotView,
) {
    use crate::fleet_ensure::{
        model::{EffectState, FleetEnsureCompletion},
        ops::{EnsurePaths, read_journal},
    };
    let now = now_ms();
    let Ok(Some(journal)) = read_journal(&EnsurePaths::under(
        root,
        &options.environment,
        &options.fleet,
    )) else {
        return;
    };
    if journal.fleet != options.fleet {
        return;
    }
    let completion = match journal.completion {
        FleetEnsureCompletion::Converged => LocalOperationCompletion::Converged,
        FleetEnsureCompletion::Prepared => LocalOperationCompletion::Prepared,
        FleetEnsureCompletion::InProgress => LocalOperationCompletion::InProgress,
        FleetEnsureCompletion::ReplanRequired => LocalOperationCompletion::ReplanRequired,
    };
    snapshot.operation = observed(
        now,
        ObservationSource::LocalOperationJournal,
        LocalOperationView {
            operation_id: journal.operation_id,
            plan_sha256: journal.plan_sha256,
            completion,
            applied_effects: journal
                .effects
                .iter()
                .filter(|effect| effect.state == EffectState::Applied)
                .count(),
            pending_effects: journal
                .effects
                .iter()
                .filter(|effect| effect.state != EffectState::Applied)
                .count(),
            funding_review_required: journal.estate_funding_required.is_some(),
            stalled_observations: journal.stalled_observations,
        },
    );
}
