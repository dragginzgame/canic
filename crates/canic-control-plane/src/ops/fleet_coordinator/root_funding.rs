//! Module: ops::fleet_coordinator::root_funding
//!
//! Responsibility: validate and transition the Coordinator's bounded Root-funding ledger.
//! Does not own: endpoint authorization, clocks, live balances, or inter-canister calls.
//! Boundary: workflow supplies authenticated callers and ambient observations after admission.

use super::FleetCoordinatorOps;
use crate::{
    dto::fleet_coordinator::{
        CoordinatorFundingStatusResponse, CoordinatorFundingWindowStatusResponse,
        CoordinatorRootFundingStatusResponse,
    },
    storage::stable::fleet_coordinator::{
        CoordinatorRootGrantRecord, CoordinatorRootGrantResultRecord,
        FLEET_COORDINATOR_FUNDING_SCHEMA_VERSION, FleetCoordinatorCommitError,
        FleetCoordinatorCommitOutcome, FleetCoordinatorFundingRecord, FleetCoordinatorFundingStore,
        FleetCoordinatorRegistryRecord, FleetCoordinatorRegistryStore,
        FleetRootFundingLedgerRecord, FleetRootFundingWindowRecord,
    },
    view::fleet_coordinator::{FleetRootFundingCallView, FleetRootFundingDisposition},
};
use candid::Principal;
use canic_core::{
    control_plane_support::{
        error::InternalError,
        ops::fleet_registry::FleetRegistryOps,
        policy::fleet_funding::{
            FleetFundingAutomaticUsageSnapshot, FleetFundingWindowSnapshot,
            FleetRootGrantAuthorityMatch, FleetRootGrantAvailability, FleetRootGrantDecision,
            FleetRootGrantDecisionInput, FleetRootGrantNoGrantReason, decide_fleet_root_grant,
        },
    },
    dto::{
        fleet_funding::{
            FleetFundingPolicyRotationPlan, FleetRootFundingAcceptanceReceipt,
            FleetRootFundingAcceptanceRequest, FleetRootFundingNoGrantReason,
            FleetRootFundingNoGrantReceipt, FleetRootFundingRequest, FleetRootFundingResponse,
            MAX_FLEET_FUNDING_POLICY_ROTATION_HISTORY_ROOTS,
        },
        fleet_registry::{FleetRegistryVersion, FleetSubnetRootEntry, FleetSubnetRootStatus},
        state::SetStateResponse,
    },
    ids::{FLEET_ROOT_FUNDING_CALL_RESERVATION_CYCLES, MAX_FLEET_ROOT_FUNDING_SLOTS},
    shared_support::fleet_funding_policy::{
        coordinator_root_funding_policy_hash, fleet_funding_policy_rotation_operation_id,
        fleet_funding_policy_rotation_plan_digest, fleet_funding_policy_rotation_roots_digest,
        fleet_funding_policy_rotation_successor_policy_set_hash, fleet_root_funding_operation_id,
        fleet_subnet_root_funding_policy_hash, validate_fleet_funding_policy_rotation_plan,
    },
};
use std::collections::BTreeMap;

impl FleetCoordinatorOps {
    pub(crate) fn compile_funding_genesis() -> FleetCoordinatorFundingRecord {
        FleetCoordinatorFundingRecord::default()
    }

    pub(crate) fn commit_funding_genesis(
        record: FleetCoordinatorFundingRecord,
    ) -> Result<FleetCoordinatorCommitOutcome, InternalError> {
        let registry = Self::current()?;
        let record = validate_funding_record(&registry, record)?;
        FleetCoordinatorFundingStore::commit_genesis(record).map_err(|_| InternalError::conflict())
    }

    /// Authenticate the transport caller against raw Registry membership before treasury reads.
    pub(crate) fn authorize_root_funding_caller(caller: Principal) -> Result<(), InternalError> {
        if caller == Principal::anonymous() {
            return Err(InternalError::forbidden());
        }
        let registry = FleetCoordinatorRegistryStore::export()
            .current
            .ok_or_else(InternalError::unavailable)?;
        if caller == registry.authority.binding.coordinator {
            return Err(InternalError::forbidden());
        }
        let mut matches = registry
            .registry
            .fleet_subnet_roots
            .iter()
            .filter(|root| root.fleet_subnet_root == caller);
        let Some(_) = matches.next() else {
            return Err(InternalError::forbidden());
        };
        if matches.next().is_some() {
            return Err(InternalError::invariant());
        }
        Ok(())
    }

    pub(crate) fn prepare_root_funding(
        caller: Principal,
        request: FleetRootFundingRequest,
        coordinator_balance: u128,
        now_ns: u64,
    ) -> Result<FleetRootFundingDisposition, InternalError> {
        let registry = Self::current()?;
        if FleetCoordinatorFundingStore::export()
            .current
            .is_some_and(|funding| funding.rotation_current.is_some())
        {
            return Err(InternalError::conflict());
        }
        let root = exact_registry_root(&registry, caller)?;
        let coordinator = registry.authority.binding.coordinator;
        let policy_hash = fleet_subnet_root_funding_policy_hash(&root.funding);
        let expected_operation_id = fleet_root_funding_operation_id(
            coordinator,
            caller,
            request.operation_sequence,
            &request.expected_registry,
            request.observed_balance.to_u128(),
            request.requested_cycles.to_u128(),
            request.policy_hash,
        );
        if request.operation_id != expected_operation_id {
            return Err(InternalError::invalid_input());
        }
        if request.expected_registry.authority != registry.authority {
            return Err(InternalError::invalid_input());
        }

        let current = current_funding(&registry)?;
        let mut next = current.clone();
        let root_index = ensure_root_slot(&mut next, caller)?;
        let root_ledger = &next.roots[root_index];

        if let Some(active) = root_ledger.current.as_ref() {
            if active.request == request {
                return Ok(FleetRootFundingDisposition::Reconcile(funding_call(active)));
            }
            return Err(InternalError::conflict());
        }
        if let Some(last) = root_ledger.last.as_ref()
            && last.request == request
        {
            return Ok(FleetRootFundingDisposition::Current(last.response.clone()));
        }

        let expected_sequence = root_ledger
            .last
            .as_ref()
            .map_or(Some(1), |last| {
                last.request.operation_sequence.checked_add(1)
            })
            .ok_or_else(InternalError::conflict)?;
        if request.operation_sequence != expected_sequence {
            return Err(InternalError::conflict());
        }

        let decision = decide_root_funding(
            &registry,
            &next,
            root_ledger,
            root,
            &request,
            policy_hash,
            FleetRootFundingObservation {
                coordinator_balance,
                now_ns,
            },
        )?;

        match decision {
            FleetRootGrantDecision::NoGrant(reason) => {
                let response = FleetRootFundingResponse::NoGrant(FleetRootFundingNoGrantReceipt {
                    request: request.clone(),
                    reason: map_no_grant_reason(reason),
                    decided_at_ns: now_ns,
                });
                next.roots[root_index].last = Some(CoordinatorRootGrantResultRecord {
                    request,
                    response: response.clone(),
                    fleet_window_start_secs: None,
                    root_window_start_secs: None,
                    prepared_at_ns: now_ns,
                    completed_at_ns: now_ns,
                });
                let next = validate_funding_record(&registry, next)?;
                commit_funding_transition(&current, next)?;
                Ok(FleetRootFundingDisposition::Current(response))
            }
            FleetRootGrantDecision::Grant {
                fleet_window_start_secs,
                root_window_start_secs,
            } => {
                let active = CoordinatorRootGrantRecord {
                    request,
                    fleet_subnet_root: caller,
                    fleet_window_start_secs,
                    root_window_start_secs,
                    call_reservation_cycles: FLEET_ROOT_FUNDING_CALL_RESERVATION_CYCLES.into(),
                    prepared_at_ns: now_ns,
                };
                let call = funding_call(&active);
                next.roots[root_index].current = Some(active);
                let next = validate_funding_record(&registry, next)?;
                commit_funding_transition(&current, next)?;
                Ok(FleetRootFundingDisposition::Invoke(call))
            }
        }
    }

    pub(crate) fn set_root_funding_enabled(
        enabled: bool,
    ) -> Result<SetStateResponse<bool>, InternalError> {
        let registry = Self::current()?;
        if FleetCoordinatorFundingStore::export()
            .current
            .is_some_and(|funding| funding.rotation_current.is_some())
        {
            return Err(InternalError::conflict());
        }
        let current = current_funding(&registry)?;
        let previous = current.funding_enabled;
        if previous != enabled {
            let mut next = current.clone();
            next.funding_enabled = enabled;
            let next = validate_funding_record(&registry, next)?;
            commit_funding_transition(&current, next)?;
        }
        Ok(SetStateResponse {
            previous,
            current: enabled,
            changed: previous != enabled,
        })
    }

    /// Reject Coordinator authority capture while one attached-cycles grant is unresolved.
    pub(crate) fn require_root_funding_snapshot_resumable() -> Result<(), InternalError> {
        let registry = Self::current()?;
        let funding = current_funding(&registry)?;
        if funding.rotation_current.is_some()
            || funding.roots.iter().any(|root| root.current.is_some())
        {
            return Err(InternalError::conflict());
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the status projection keeps Coordinator, history-capacity and per-Root accounting fields together"
    )]
    pub(crate) fn root_funding_status(
        coordinator_balance: u128,
        now_ns: u64,
    ) -> Result<CoordinatorFundingStatusResponse, InternalError> {
        let registry = Self::current()?;
        let funding = current_funding(&registry)?;
        let policy = registry.root_funding.clone();
        let fleet_window = policy
            .as_ref()
            .map(|policy| {
                let window_start_secs = epoch_window_start(now_ns, policy.budget.window_secs)?;
                Ok::<_, InternalError>(CoordinatorFundingWindowStatusResponse {
                    window_start_secs,
                    spent_cycles: funding
                        .fleet_window
                        .as_ref()
                        .filter(|window| window.window_start_secs == window_start_secs)
                        .map_or(0_u128, |window| window.spent_cycles.to_u128())
                        .into(),
                    reserved_cycles: reserved_fleet_cycles(&funding, window_start_secs)?.into(),
                })
            })
            .transpose()?;
        let mut roots = Vec::with_capacity(registry.registry.fleet_subnet_roots.len());
        for root in &registry.registry.fleet_subnet_roots {
            let policy = &root.funding.root_funding;
            let window_start_secs = epoch_window_start(now_ns, policy.budget.window_secs)?;
            let ledger = funding
                .roots
                .binary_search_by_key(&root.fleet_subnet_root, |ledger| ledger.fleet_subnet_root)
                .ok()
                .map(|index| &funding.roots[index]);
            roots.push(CoordinatorRootFundingStatusResponse {
                fleet_subnet_root: root.fleet_subnet_root,
                lifecycle_status: root.status,
                policy_hash: fleet_subnet_root_funding_policy_hash(&root.funding),
                policy: policy.clone(),
                window: CoordinatorFundingWindowStatusResponse {
                    window_start_secs,
                    spent_cycles: ledger
                        .and_then(|ledger| ledger.window.as_ref())
                        .filter(|window| window.window_start_secs == window_start_secs)
                        .map_or(0_u128, |window| window.spent_cycles.to_u128())
                        .into(),
                    reserved_cycles: ledger
                        .map_or(0, |ledger| reserved_root_cycles(ledger, window_start_secs))
                        .into(),
                },
                historical_automatic_grants: ledger
                    .map_or(0, |ledger| ledger.historical_automatic_grants),
                historical_automatic_cycles: ledger
                    .map_or(0_u128, |ledger| {
                        ledger.historical_automatic_cycles.to_u128()
                    })
                    .into(),
                automatic_grants: ledger.map_or(0, |ledger| ledger.automatic_grants),
                automatic_cycles: ledger
                    .map_or(0_u128, |ledger| ledger.automatic_cycles.to_u128())
                    .into(),
                last_successful_grant_at_ns: ledger
                    .and_then(|ledger| ledger.last_successful_grant_at_ns),
                current_operation: ledger
                    .and_then(|ledger| ledger.current.as_ref())
                    .map(|current| current.request.clone()),
                last_result: ledger
                    .and_then(|ledger| ledger.last.as_ref())
                    .map(|last| last.response.clone()),
            });
        }
        let rotation = funding
            .rotation_current
            .as_ref()
            .map(|rotation| Self::funding_policy_rotation_status(rotation.operation_id))
            .transpose()?
            .flatten();
        let rotation_checkpoint_count = u32::try_from(funding.rotation_history.len())
            .map_err(|_| InternalError::invariant())?;
        let rotation_checkpoint_root_count =
            funding
                .rotation_history
                .iter()
                .try_fold(0_usize, |count, checkpoint| {
                    count
                        .checked_add(checkpoint.roots.len())
                        .ok_or_else(InternalError::invariant)
                })?;
        let rotation_checkpoint_root_capacity_remaining =
            MAX_FLEET_FUNDING_POLICY_ROTATION_HISTORY_ROOTS
                .checked_sub(rotation_checkpoint_root_count)
                .and_then(|remaining| u32::try_from(remaining).ok())
                .ok_or_else(InternalError::invariant)?;
        Ok(CoordinatorFundingStatusResponse {
            coordinator: registry.authority.binding.coordinator,
            current_cycles: coordinator_balance.into(),
            policy_generation: funding.policy_generation,
            funding_enabled: funding.funding_enabled,
            funding_profile: policy.as_ref().map(|policy| policy.funding_profile),
            policy,
            fleet_window,
            historical_automatic_grants: funding.historical_automatic_grants,
            historical_automatic_cycles: funding.historical_automatic_cycles,
            automatic_grants: funding.automatic_grants,
            automatic_cycles: funding.automatic_cycles,
            rotation_checkpoint_count,
            rotation_checkpoint_root_count: u32::try_from(rotation_checkpoint_root_count)
                .map_err(|_| InternalError::invariant())?,
            rotation_checkpoint_root_capacity_remaining,
            rotation,
            roots,
        })
    }

    pub(crate) fn record_root_funding_acceptance(
        fleet_subnet_root: Principal,
        request: &FleetRootFundingAcceptanceRequest,
        receipt: FleetRootFundingAcceptanceReceipt,
        completed_at_ns: u64,
    ) -> Result<FleetRootFundingResponse, InternalError> {
        let registry = Self::current()?;
        let current = current_funding(&registry)?;
        let mut next = current.clone();
        let root_index = root_slot(&next, fleet_subnet_root)?;
        let active = next.roots[root_index]
            .current
            .clone()
            .ok_or_else(InternalError::conflict)?;
        if funding_acceptance_request(&active) != *request
            || receipt.request != *request
            || receipt.fleet_subnet_root != fleet_subnet_root
            || receipt.coordinator != registry.authority.binding.coordinator
            || receipt.accepted_at_ns > completed_at_ns
        {
            return Err(InternalError::conflict());
        }

        let granted_cycles = request.granted_cycles.to_u128();
        commit_window_spend(
            &mut next.fleet_window,
            active.fleet_window_start_secs,
            granted_cycles,
        )?;
        next.automatic_grants = next
            .automatic_grants
            .checked_add(1)
            .ok_or_else(InternalError::invariant)?;
        next.automatic_cycles = next
            .automatic_cycles
            .to_u128()
            .checked_add(granted_cycles)
            .ok_or_else(InternalError::invariant)?
            .into();
        next.roots[root_index].automatic_grants = next.roots[root_index]
            .automatic_grants
            .checked_add(1)
            .ok_or_else(InternalError::invariant)?;
        next.roots[root_index].automatic_cycles = next.roots[root_index]
            .automatic_cycles
            .to_u128()
            .checked_add(granted_cycles)
            .ok_or_else(InternalError::invariant)?
            .into();
        commit_window_spend(
            &mut next.roots[root_index].window,
            active.root_window_start_secs,
            granted_cycles,
        )?;
        next.roots[root_index].last_successful_grant_at_ns = Some(receipt.accepted_at_ns);
        next.roots[root_index].current = None;
        let response = FleetRootFundingResponse::Granted(receipt);
        next.roots[root_index].last = Some(CoordinatorRootGrantResultRecord {
            request: active.request,
            response: response.clone(),
            fleet_window_start_secs: Some(active.fleet_window_start_secs),
            root_window_start_secs: Some(active.root_window_start_secs),
            prepared_at_ns: active.prepared_at_ns,
            completed_at_ns,
        });
        let next = validate_funding_record(&registry, next)?;
        commit_funding_transition(&current, next)?;
        Ok(response)
    }

    pub(crate) fn record_root_funding_rejection(
        fleet_subnet_root: Principal,
        request: &FleetRootFundingAcceptanceRequest,
        decided_at_ns: u64,
    ) -> Result<FleetRootFundingResponse, InternalError> {
        let registry = Self::current()?;
        let current = current_funding(&registry)?;
        let mut next = current.clone();
        let root_index = root_slot(&next, fleet_subnet_root)?;
        let active = next.roots[root_index]
            .current
            .clone()
            .ok_or_else(InternalError::conflict)?;
        if funding_acceptance_request(&active) != *request {
            return Err(InternalError::conflict());
        }

        next.roots[root_index].current = None;
        let response = FleetRootFundingResponse::NoGrant(FleetRootFundingNoGrantReceipt {
            request: active.request.clone(),
            reason: FleetRootFundingNoGrantReason::RootRejected,
            decided_at_ns,
        });
        next.roots[root_index].last = Some(CoordinatorRootGrantResultRecord {
            request: active.request,
            response: response.clone(),
            fleet_window_start_secs: Some(active.fleet_window_start_secs),
            root_window_start_secs: Some(active.root_window_start_secs),
            prepared_at_ns: active.prepared_at_ns,
            completed_at_ns: decided_at_ns,
        });
        let next = validate_funding_record(&registry, next)?;
        commit_funding_transition(&current, next)?;
        Ok(response)
    }
}

#[derive(Clone, Copy)]
struct FleetRootFundingObservation {
    coordinator_balance: u128,
    now_ns: u64,
}

fn decide_root_funding(
    registry: &FleetCoordinatorRegistryRecord,
    funding: &FleetCoordinatorFundingRecord,
    root_ledger: &FleetRootFundingLedgerRecord,
    root: &FleetSubnetRootEntry,
    request: &FleetRootFundingRequest,
    policy_hash: [u8; 32],
    observation: FleetRootFundingObservation,
) -> Result<FleetRootGrantDecision, InternalError> {
    let coordinator_policy = registry
        .root_funding
        .as_ref()
        .ok_or_else(InternalError::invariant)?;
    let fleet_window_start_secs =
        epoch_window_start(observation.now_ns, coordinator_policy.budget.window_secs)?;
    let root_window_start_secs = epoch_window_start(
        observation.now_ns,
        root.funding.root_funding.budget.window_secs,
    )?;
    let fleet_reserved_cycles = reserved_fleet_cycles(funding, fleet_window_start_secs)?;
    let root_reserved_cycles = reserved_root_cycles(root_ledger, root_window_start_secs);
    Ok(decide_fleet_root_grant(&FleetRootGrantDecisionInput {
        availability: FleetRootGrantAvailability {
            funding_enabled: funding.funding_enabled,
            root_is_eligible: matches!(
                root.status,
                FleetSubnetRootStatus::Active | FleetSubnetRootStatus::Draining
            ),
        },
        authority_match: FleetRootGrantAuthorityMatch {
            registry_matches: request.expected_registry == registry_version(registry)?,
            policy_matches: request.policy_hash == policy_hash,
        },
        observed_balance: request.observed_balance.to_u128(),
        requested_cycles: request.requested_cycles.to_u128(),
        now_ns: observation.now_ns,
        coordinator_balance: observation.coordinator_balance,
        call_reservation_cycles: FLEET_ROOT_FUNDING_CALL_RESERVATION_CYCLES,
        coordinator_policy,
        root_policy: &root.funding.root_funding,
        fleet_window: Some(window_snapshot(
            funding.fleet_window.as_ref(),
            fleet_window_start_secs,
            fleet_reserved_cycles,
        )),
        root_window: Some(window_snapshot(
            root_ledger.window.as_ref(),
            root_window_start_secs,
            root_reserved_cycles,
        )),
        fleet_automatic_usage: FleetFundingAutomaticUsageSnapshot {
            successful_grants: funding.automatic_grants,
            granted_cycles: funding.automatic_cycles.to_u128(),
        },
        root_automatic_usage: FleetFundingAutomaticUsageSnapshot {
            successful_grants: root_ledger.automatic_grants,
            granted_cycles: root_ledger.automatic_cycles.to_u128(),
        },
        last_accepted_at_ns: root_ledger.last_successful_grant_at_ns,
    }))
}

fn exact_registry_root(
    registry: &FleetCoordinatorRegistryRecord,
    fleet_subnet_root: Principal,
) -> Result<&FleetSubnetRootEntry, InternalError> {
    let mut matches = registry
        .registry
        .fleet_subnet_roots
        .iter()
        .filter(|root| root.fleet_subnet_root == fleet_subnet_root);
    let root = matches.next().ok_or_else(InternalError::forbidden)?;
    if matches.next().is_some() {
        return Err(InternalError::invariant());
    }
    Ok(root)
}

fn registry_version(
    registry: &FleetCoordinatorRegistryRecord,
) -> Result<FleetRegistryVersion, InternalError> {
    FleetRegistryOps::version(
        &registry.authority,
        &registry
            .component_deployment_configuration
            .component_topology,
        &registry.registry,
    )
}

fn current_funding(
    registry: &FleetCoordinatorRegistryRecord,
) -> Result<FleetCoordinatorFundingRecord, InternalError> {
    let current = FleetCoordinatorFundingStore::export()
        .current
        .ok_or_else(InternalError::unavailable)?;
    validate_funding_record(registry, current)
}

fn ensure_root_slot(
    funding: &mut FleetCoordinatorFundingRecord,
    fleet_subnet_root: Principal,
) -> Result<usize, InternalError> {
    match funding
        .roots
        .binary_search_by_key(&fleet_subnet_root, |root| root.fleet_subnet_root)
    {
        Ok(index) => Ok(index),
        Err(index) => {
            if funding.roots.len() >= MAX_FLEET_ROOT_FUNDING_SLOTS {
                return Err(InternalError::resource_exhausted());
            }
            funding.roots.insert(
                index,
                FleetRootFundingLedgerRecord {
                    fleet_subnet_root,
                    historical_automatic_grants: 0,
                    historical_automatic_cycles: 0_u128.into(),
                    automatic_grants: 0,
                    automatic_cycles: 0_u128.into(),
                    window: None,
                    last_successful_grant_at_ns: None,
                    current: None,
                    last: None,
                },
            );
            Ok(index)
        }
    }
}

fn root_slot(
    funding: &FleetCoordinatorFundingRecord,
    fleet_subnet_root: Principal,
) -> Result<usize, InternalError> {
    funding
        .roots
        .binary_search_by_key(&fleet_subnet_root, |root| root.fleet_subnet_root)
        .map_err(|_| InternalError::invariant())
}

fn funding_call(record: &CoordinatorRootGrantRecord) -> FleetRootFundingCallView {
    FleetRootFundingCallView {
        fleet_subnet_root: record.fleet_subnet_root,
        request: funding_acceptance_request(record),
    }
}

fn funding_acceptance_request(
    record: &CoordinatorRootGrantRecord,
) -> FleetRootFundingAcceptanceRequest {
    FleetRootFundingAcceptanceRequest {
        operation_id: record.request.operation_id,
        operation_sequence: record.request.operation_sequence,
        expected_registry: record.request.expected_registry.clone(),
        observed_balance: record.request.observed_balance.clone(),
        granted_cycles: record.request.requested_cycles.clone(),
        policy_hash: record.request.policy_hash,
    }
}

fn epoch_window_start(now_ns: u64, window_secs: u64) -> Result<u64, InternalError> {
    let now_secs = now_ns / 1_000_000_000;
    (window_secs != 0)
        .then(|| (now_secs / window_secs) * window_secs)
        .ok_or_else(InternalError::invariant)
}

fn window_snapshot(
    spent: Option<&FleetRootFundingWindowRecord>,
    window_start_secs: u64,
    reserved_cycles: u128,
) -> FleetFundingWindowSnapshot {
    let spent_cycles = spent
        .filter(|window| window.window_start_secs == window_start_secs)
        .map_or(0, |window| window.spent_cycles.to_u128());
    FleetFundingWindowSnapshot {
        window_start_secs,
        spent_cycles,
        reserved_cycles,
    }
}

fn reserved_fleet_cycles(
    funding: &FleetCoordinatorFundingRecord,
    window_start_secs: u64,
) -> Result<u128, InternalError> {
    funding
        .roots
        .iter()
        .filter_map(|root| root.current.as_ref())
        .filter(|current| current.fleet_window_start_secs == window_start_secs)
        .try_fold(0_u128, |total, current| {
            total
                .checked_add(current.request.requested_cycles.to_u128())
                .ok_or_else(InternalError::invariant)
        })
}

fn reserved_root_cycles(root: &FleetRootFundingLedgerRecord, window_start_secs: u64) -> u128 {
    root.current
        .as_ref()
        .filter(|current| current.root_window_start_secs == window_start_secs)
        .map_or(0, |current| current.request.requested_cycles.to_u128())
}

fn commit_window_spend(
    window: &mut Option<FleetRootFundingWindowRecord>,
    reservation_window_start_secs: u64,
    granted_cycles: u128,
) -> Result<(), InternalError> {
    match window {
        Some(current) if current.window_start_secs == reservation_window_start_secs => {
            current.spent_cycles = current
                .spent_cycles
                .to_u128()
                .checked_add(granted_cycles)
                .ok_or_else(InternalError::invariant)?
                .into();
        }
        Some(current) if current.window_start_secs > reservation_window_start_secs => {}
        Some(_) | None => {
            *window = Some(FleetRootFundingWindowRecord {
                window_start_secs: reservation_window_start_secs,
                spent_cycles: granted_cycles.into(),
            });
        }
    }
    Ok(())
}

pub(super) fn validate_funding_record(
    registry: &FleetCoordinatorRegistryRecord,
    funding: FleetCoordinatorFundingRecord,
) -> Result<FleetCoordinatorFundingRecord, InternalError> {
    if funding.schema_version != FLEET_COORDINATOR_FUNDING_SCHEMA_VERSION
        || funding.policy_generation == 0
        || funding.roots.len() > MAX_FLEET_ROOT_FUNDING_SLOTS
        || funding.roots.len() > registry.registry.fleet_subnet_roots.len()
    {
        return Err(InternalError::invariant());
    }
    let Some(coordinator_policy) = registry.root_funding.as_ref() else {
        if registry.registry.fleet_subnet_roots.is_empty()
            && funding.roots.is_empty()
            && funding.fleet_window.is_none()
        {
            return Ok(funding);
        }
        return Err(InternalError::invariant());
    };
    validate_window(
        funding.fleet_window.as_ref(),
        coordinator_policy.budget.window_secs,
        coordinator_policy.budget.maximum_cycles.to_u128(),
    )?;
    if funding.automatic_grants > coordinator_policy.maximum_automatic_grants
        || funding.automatic_cycles.to_u128()
            > coordinator_policy.maximum_automatic_cycles.to_u128()
    {
        return Err(InternalError::invariant());
    }

    let mut previous_root = None;
    let mut fleet_reservations = BTreeMap::<u64, u128>::new();
    let mut root_automatic_grants = 0_u32;
    let mut root_automatic_cycles = 0_u128;
    let mut root_historical_grants = 0_u64;
    let mut root_historical_cycles = 0_u128;
    for root_ledger in &funding.roots {
        if previous_root.is_some_and(|previous| previous >= root_ledger.fleet_subnet_root) {
            return Err(InternalError::invariant());
        }
        previous_root = Some(root_ledger.fleet_subnet_root);
        let root = exact_registry_root(registry, root_ledger.fleet_subnet_root)?;
        let policy_hash = fleet_subnet_root_funding_policy_hash(&root.funding);
        if root_ledger.automatic_grants > root.funding.root_funding.maximum_automatic_grants
            || root_ledger.automatic_cycles.to_u128()
                > root.funding.root_funding.maximum_automatic_cycles.to_u128()
        {
            return Err(InternalError::invariant());
        }
        root_automatic_grants = root_automatic_grants
            .checked_add(root_ledger.automatic_grants)
            .ok_or_else(InternalError::invariant)?;
        root_automatic_cycles = root_automatic_cycles
            .checked_add(root_ledger.automatic_cycles.to_u128())
            .ok_or_else(InternalError::invariant)?;
        root_historical_grants = root_historical_grants
            .checked_add(root_ledger.historical_automatic_grants)
            .ok_or_else(InternalError::invariant)?;
        root_historical_cycles = root_historical_cycles
            .checked_add(root_ledger.historical_automatic_cycles.to_u128())
            .ok_or_else(InternalError::invariant)?;
        validate_window(
            root_ledger.window.as_ref(),
            root.funding.root_funding.budget.window_secs,
            root.funding.root_funding.budget.maximum_cycles.to_u128(),
        )?;
        validate_root_ledger(registry, root, root_ledger, policy_hash)?;
        if let Some(active) = root_ledger.current.as_ref() {
            let reserved = fleet_reservations
                .entry(active.fleet_window_start_secs)
                .or_default();
            *reserved = reserved
                .checked_add(active.request.requested_cycles.to_u128())
                .ok_or_else(InternalError::invariant)?;
        }
    }

    if funding.automatic_grants != root_automatic_grants
        || funding.automatic_cycles.to_u128() != root_automatic_cycles
        || funding.historical_automatic_grants != root_historical_grants
        || funding.historical_automatic_cycles.to_u128() != root_historical_cycles
    {
        return Err(InternalError::invariant());
    }

    for (window_start_secs, reserved_cycles) in fleet_reservations {
        let spent_cycles = funding
            .fleet_window
            .as_ref()
            .filter(|window| window.window_start_secs == window_start_secs)
            .map_or(0, |window| window.spent_cycles.to_u128());
        if spent_cycles
            .checked_add(reserved_cycles)
            .is_none_or(|total| total > coordinator_policy.budget.maximum_cycles.to_u128())
        {
            return Err(InternalError::invariant());
        }
    }
    validate_rotation_record(registry, &funding)?;
    Ok(funding)
}

#[expect(
    clippy::too_many_lines,
    reason = "one validator covers every durable phase of the sole rotation record"
)]
fn validate_rotation_record(
    registry: &FleetCoordinatorRegistryRecord,
    funding: &FleetCoordinatorFundingRecord,
) -> Result<(), InternalError> {
    let retained_rotation_roots = validate_rotation_history(funding)?;
    if let Some(last) = funding.rotation_last.as_ref()
        && (last.operation_id == [0; 32]
            || last.plan_digest == [0; 32]
            || last.successor_generation > funding.policy_generation
            || last.predecessor_generation.checked_add(1) != Some(last.successor_generation)
            || last.successor_policy_set_hash == [0; 32]
            || last.apply_operator_debit.to_u128() != 0)
    {
        return Err(InternalError::invariant());
    }
    let registry_still_predecessor = funding.rotation_current.as_ref().is_none_or(|rotation| {
        registry_version(registry).ok().as_ref() == Some(&rotation.header.predecessor_registry)
    });
    if let Some(last) = funding.rotation_last.as_ref().filter(|last| {
        last.successor_generation == funding.policy_generation && registry_still_predecessor
    }) {
        let coordinator_policy = registry
            .root_funding
            .as_ref()
            .ok_or_else(InternalError::invariant)?;
        let current_hash = fleet_funding_policy_rotation_successor_policy_set_hash(
            coordinator_policy,
            registry
                .registry
                .fleet_subnet_roots
                .iter()
                .map(|root| (root.fleet_subnet_root, &root.funding)),
        );
        if current_hash != last.successor_policy_set_hash {
            return Err(InternalError::invariant());
        }
    }
    let Some(rotation) = funding.rotation_current.as_ref() else {
        return Ok(());
    };
    if rotation.operation_id
        != fleet_funding_policy_rotation_operation_id(
            registry.authority.binding.coordinator,
            rotation.plan_digest,
        )
        || rotation.header.predecessor_generation.checked_add(1)
            != Some(rotation.header.successor_generation)
        || rotation.header.affected_root_count as usize
            != registry.registry.fleet_subnet_roots.len()
        || retained_rotation_roots
            .checked_add(rotation.header.affected_root_count as usize)
            .is_none_or(|count| count > MAX_FLEET_FUNDING_POLICY_ROTATION_HISTORY_ROOTS)
        || rotation.roots.len() > rotation.header.affected_root_count as usize
        || rotation
            .roots
            .windows(2)
            .any(|roots| roots[0].fleet_subnet_root >= roots[1].fleet_subnet_root)
        || funding.roots.iter().any(|root| root.current.is_some())
        || rotation.header.maximum_new_automatic_cycles
            != rotation
                .header
                .proposed_coordinator_policy
                .maximum_automatic_cycles
        || rotation.header.apply_operator_debit.to_u128() != 0
    {
        return Err(InternalError::invariant());
    }
    match &rotation.phase {
        crate::storage::stable::fleet_coordinator::FleetFundingPolicyRotationPhaseRecord::Staging => {
            if funding.policy_generation != rotation.header.predecessor_generation
                || registry_version(registry)? != rotation.header.predecessor_registry
                || registry.root_funding.as_ref().map(coordinator_root_funding_policy_hash)
                    != Some(rotation.header.predecessor_coordinator_policy_hash)
                || !rotation_predecessor_usage_matches(funding, rotation)
            {
                return Err(InternalError::invariant());
            }
            validate_staged_rotation_roots(registry, funding, rotation)?;
        }
        crate::storage::stable::fleet_coordinator::FleetFundingPolicyRotationPhaseRecord::PreparingRoots { prepared } => {
            if funding.policy_generation != rotation.header.predecessor_generation
                || registry_version(registry)? != rotation.header.predecessor_registry
                || registry.root_funding.as_ref().map(coordinator_root_funding_policy_hash)
                    != Some(rotation.header.predecessor_coordinator_policy_hash)
                || !rotation_predecessor_usage_matches(funding, rotation)
                || prepared.len() > rotation.roots.len()
            {
                return Err(InternalError::invariant());
            }
            validate_staged_rotation_roots(registry, funding, rotation)?;
            validate_rotation_receipts(rotation, prepared, false)?;
            if rotation.roots.len() == rotation.header.affected_root_count as usize {
                let plan = FleetFundingPolicyRotationPlan {
                    header: rotation.header.clone(),
                    roots: rotation.roots.clone(),
                };
                if fleet_funding_policy_rotation_roots_digest(&plan.roots)
                    != plan.header.roots_digest
                    || fleet_funding_policy_rotation_plan_digest(&plan) != rotation.plan_digest
                {
                    return Err(InternalError::invariant());
                }
            }
        }
        crate::storage::stable::fleet_coordinator::FleetFundingPolicyRotationPhaseRecord::ActivatingRoots { successor_registry, prepared, activated } => {
            if funding.policy_generation != rotation.header.successor_generation
                || prepared.len() != rotation.roots.len()
                || activated.len() > rotation.roots.len()
                || &registry_version(registry)? != successor_registry.as_ref()
                || registry.root_funding.as_ref()
                    != Some(&rotation.header.proposed_coordinator_policy)
            {
                return Err(InternalError::invariant());
            }
            for root in &rotation.roots {
                if exact_registry_root(registry, root.fleet_subnet_root)?
                    .funding
                    .root_funding
                    != root.proposed_policy
                {
                    return Err(InternalError::invariant());
                }
            }
            validate_rotation_receipts(rotation, prepared, false)?;
            validate_rotation_receipts(rotation, activated, true)?;
        }
    }
    Ok(())
}

fn validate_rotation_history(
    funding: &FleetCoordinatorFundingRecord,
) -> Result<usize, InternalError> {
    if funding
        .rotation_history
        .last()
        .map(|checkpoint| &checkpoint.receipt)
        != funding.rotation_last.as_ref()
    {
        return Err(InternalError::invariant());
    }
    let mut retained_roots = 0_usize;
    let mut previous_generation = None;
    for checkpoint in &funding.rotation_history {
        let receipt = &checkpoint.receipt;
        retained_roots = retained_roots
            .checked_add(checkpoint.roots.len())
            .ok_or_else(InternalError::invariant)?;
        let roots_are_ordered = checkpoint
            .roots
            .windows(2)
            .all(|roots| roots[0].fleet_subnet_root < roots[1].fleet_subnet_root);
        let authority_is_exact =
            receipt.predecessor_registry.authority == receipt.successor_registry.authority;
        let revision_is_exact = receipt.predecessor_registry.revision.checked_add(1)
            == Some(receipt.successor_registry.revision);
        let predecessor_follows = previous_generation
            .map_or(receipt.predecessor_generation == 1, |generation| {
                generation == receipt.predecessor_generation
            });
        let generation_is_exact = receipt.predecessor_generation.checked_add(1)
            == Some(receipt.successor_generation)
            && predecessor_follows;
        let operation_is_exact = receipt.operation_id
            == fleet_funding_policy_rotation_operation_id(
                receipt.successor_registry.authority.binding.coordinator,
                receipt.plan_digest,
            );
        let root_count_is_exact = !checkpoint.roots.is_empty()
            && checkpoint.roots.len() == receipt.affected_root_count as usize;
        let plan_is_exact = rotation_checkpoint_plan_is_exact(checkpoint);
        let policy_set_hash = fleet_funding_policy_rotation_successor_policy_set_hash(
            &checkpoint.coordinator_policy,
            checkpoint
                .roots
                .iter()
                .map(|root| (root.fleet_subnet_root, &root.funding)),
        );
        if ![
            roots_are_ordered,
            authority_is_exact,
            revision_is_exact,
            generation_is_exact,
            operation_is_exact,
            root_count_is_exact,
            plan_is_exact,
            policy_set_hash == receipt.successor_policy_set_hash,
            receipt.apply_operator_debit.to_u128() == 0,
        ]
        .into_iter()
        .all(|is_exact| is_exact)
        {
            return Err(InternalError::invariant());
        }
        previous_generation = Some(receipt.successor_generation);
    }
    if retained_roots > MAX_FLEET_FUNDING_POLICY_ROTATION_HISTORY_ROOTS {
        return Err(InternalError::invariant());
    }
    Ok(retained_roots)
}

fn rotation_checkpoint_plan_is_exact(
    checkpoint: &crate::storage::stable::fleet_coordinator::FleetFundingPolicyRotationCheckpointRecord,
) -> bool {
    let receipt = &checkpoint.receipt;
    let usage = &checkpoint.plan.header.predecessor_usage;
    let expected_historical_grants = usage
        .historical_automatic_grants
        .checked_add(u64::from(usage.generation_automatic_grants));
    let expected_historical_cycles = usage
        .historical_automatic_cycles
        .to_u128()
        .checked_add(usage.generation_automatic_cycles.to_u128());
    validate_fleet_funding_policy_rotation_plan(&checkpoint.plan).is_ok()
        && fleet_funding_policy_rotation_plan_digest(&checkpoint.plan) == receipt.plan_digest
        && checkpoint.plan.header.predecessor_registry == receipt.predecessor_registry
        && checkpoint.plan.header.predecessor_generation == receipt.predecessor_generation
        && checkpoint.plan.header.successor_generation == receipt.successor_generation
        && checkpoint.plan.header.affected_root_count == receipt.affected_root_count
        && checkpoint.plan.header.proposed_coordinator_policy == checkpoint.coordinator_policy
        && checkpoint.plan.header.maximum_new_automatic_cycles
            == receipt.maximum_new_automatic_cycles
        && checkpoint.plan.header.apply_operator_debit == receipt.apply_operator_debit
        && expected_historical_grants == Some(receipt.retained_historical_automatic_grants)
        && expected_historical_cycles
            == Some(receipt.retained_historical_automatic_cycles.to_u128())
        && checkpoint
            .plan
            .roots
            .iter()
            .zip(&checkpoint.roots)
            .all(|(plan, root)| {
                plan.fleet_subnet_root == root.fleet_subnet_root
                    && plan.proposed_policy == root.funding.root_funding
            })
}

fn rotation_predecessor_usage_matches(
    funding: &FleetCoordinatorFundingRecord,
    rotation: &crate::storage::stable::fleet_coordinator::FleetFundingPolicyRotationRecord,
) -> bool {
    rotation
        .header
        .predecessor_usage
        .historical_automatic_grants
        == funding.historical_automatic_grants
        && rotation
            .header
            .predecessor_usage
            .historical_automatic_cycles
            == funding.historical_automatic_cycles
        && rotation
            .header
            .predecessor_usage
            .generation_automatic_grants
            == funding.automatic_grants
        && rotation
            .header
            .predecessor_usage
            .generation_automatic_cycles
            == funding.automatic_cycles
}

fn validate_staged_rotation_roots(
    registry: &FleetCoordinatorRegistryRecord,
    funding: &FleetCoordinatorFundingRecord,
    rotation: &crate::storage::stable::fleet_coordinator::FleetFundingPolicyRotationRecord,
) -> Result<(), InternalError> {
    for root in &rotation.roots {
        let registry_root = exact_registry_root(registry, root.fleet_subnet_root)?;
        let ledger = funding
            .roots
            .binary_search_by_key(&root.fleet_subnet_root, |entry| entry.fleet_subnet_root)
            .ok()
            .map(|index| &funding.roots[index]);
        if root.predecessor_policy_hash
            != fleet_subnet_root_funding_policy_hash(&registry_root.funding)
            || root.placement.subnet != registry_root.placement_subnet
            || root.predecessor_usage.historical_automatic_grants
                != ledger.map_or(0, |entry| entry.historical_automatic_grants)
            || root.predecessor_usage.historical_automatic_cycles.to_u128()
                != ledger.map_or(0, |entry| entry.historical_automatic_cycles.to_u128())
            || root.predecessor_usage.generation_automatic_grants
                != ledger.map_or(0, |entry| entry.automatic_grants)
            || root.predecessor_usage.generation_automatic_cycles.to_u128()
                != ledger.map_or(0, |entry| entry.automatic_cycles.to_u128())
        {
            return Err(InternalError::invariant());
        }
    }
    Ok(())
}

fn validate_rotation_receipts(
    rotation: &crate::storage::stable::fleet_coordinator::FleetFundingPolicyRotationRecord,
    receipts: &[canic_core::dto::fleet_funding::FleetFundingPolicyRotationRootReceipt],
    activated: bool,
) -> Result<(), InternalError> {
    for (root, receipt) in rotation.roots.iter().zip(receipts) {
        if receipt.operation_id != rotation.operation_id
            || receipt.plan_digest != rotation.plan_digest
            || receipt.fleet_subnet_root != root.fleet_subnet_root
            || receipt.predecessor_generation != rotation.header.predecessor_generation
            || receipt.successor_generation != rotation.header.successor_generation
            || !receipt.prepared
            || receipt.activated != activated
        {
            return Err(InternalError::invariant());
        }
    }
    Ok(())
}

fn validate_root_ledger(
    registry: &FleetCoordinatorRegistryRecord,
    root: &FleetSubnetRootEntry,
    ledger: &FleetRootFundingLedgerRecord,
    policy_hash: [u8; 32],
) -> Result<(), InternalError> {
    if let Some(last) = ledger.last.as_ref() {
        validate_request(registry, root, &last.request)?;
        validate_terminal_result(registry, root, last)?;
    }
    if let Some(active) = ledger.current.as_ref() {
        if active.fleet_subnet_root != root.fleet_subnet_root
            || active.call_reservation_cycles.to_u128()
                != FLEET_ROOT_FUNDING_CALL_RESERVATION_CYCLES
        {
            return Err(InternalError::invariant());
        }
        validate_request(registry, root, &active.request)?;
        if active.request.policy_hash != policy_hash {
            return Err(InternalError::invariant());
        }
        let expected_sequence = ledger.last.as_ref().map_or(Some(1), |last| {
            last.request.operation_sequence.checked_add(1)
        });
        if Some(active.request.operation_sequence) != expected_sequence {
            return Err(InternalError::invariant());
        }
        let root_budget = &root.funding.root_funding.budget;
        if epoch_window_start(active.prepared_at_ns, root_budget.window_secs)?
            != active.root_window_start_secs
            || active.request.requested_cycles.to_u128() > root_budget.maximum_cycles.to_u128()
        {
            return Err(InternalError::invariant());
        }
        let fleet_budget = &registry
            .root_funding
            .as_ref()
            .ok_or_else(InternalError::invariant)?
            .budget;
        if epoch_window_start(active.prepared_at_ns, fleet_budget.window_secs)?
            != active.fleet_window_start_secs
            || active.request.requested_cycles.to_u128() > fleet_budget.maximum_cycles.to_u128()
        {
            return Err(InternalError::invariant());
        }
        let root_spent_cycles = ledger
            .window
            .as_ref()
            .filter(|window| window.window_start_secs == active.root_window_start_secs)
            .map_or(0, |window| window.spent_cycles.to_u128());
        if root_spent_cycles
            .checked_add(active.request.requested_cycles.to_u128())
            .is_none_or(|total| total > root_budget.maximum_cycles.to_u128())
        {
            return Err(InternalError::invariant());
        }
    } else if ledger.last.is_none() && ledger.last_successful_grant_at_ns.is_some() {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_request(
    registry: &FleetCoordinatorRegistryRecord,
    root: &FleetSubnetRootEntry,
    request: &FleetRootFundingRequest,
) -> Result<(), InternalError> {
    let expected = fleet_root_funding_operation_id(
        registry.authority.binding.coordinator,
        root.fleet_subnet_root,
        request.operation_sequence,
        &request.expected_registry,
        request.observed_balance.to_u128(),
        request.requested_cycles.to_u128(),
        request.policy_hash,
    );
    if request.operation_sequence == 0 || request.operation_id != expected {
        return Err(InternalError::invariant());
    }
    if request.expected_registry.authority != registry.authority {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_terminal_result(
    registry: &FleetCoordinatorRegistryRecord,
    root: &FleetSubnetRootEntry,
    result: &CoordinatorRootGrantResultRecord,
) -> Result<(), InternalError> {
    if result.prepared_at_ns > result.completed_at_ns {
        return Err(InternalError::invariant());
    }
    match &result.response {
        FleetRootFundingResponse::Granted(receipt) => {
            let expected_request = FleetRootFundingAcceptanceRequest {
                operation_id: result.request.operation_id,
                operation_sequence: result.request.operation_sequence,
                expected_registry: result.request.expected_registry.clone(),
                observed_balance: result.request.observed_balance.clone(),
                granted_cycles: result.request.requested_cycles.clone(),
                policy_hash: result.request.policy_hash,
            };
            if receipt.request != expected_request
                || receipt.fleet_subnet_root != root.fleet_subnet_root
                || receipt.coordinator != registry.authority.binding.coordinator
                || receipt.accepted_at_ns < result.prepared_at_ns
                || receipt.accepted_at_ns > result.completed_at_ns
                || result.fleet_window_start_secs.is_none()
                || result.root_window_start_secs.is_none()
            {
                return Err(InternalError::invariant());
            }
        }
        FleetRootFundingResponse::NoGrant(receipt) => {
            let request_matches = receipt.request == result.request;
            let decision_time_matches = receipt.decided_at_ns == result.completed_at_ns;
            let window_authority_matches = if receipt.reason
                == FleetRootFundingNoGrantReason::RootRejected
            {
                result.fleet_window_start_secs.is_some() && result.root_window_start_secs.is_some()
            } else {
                result.fleet_window_start_secs.is_none() && result.root_window_start_secs.is_none()
            };
            if !request_matches || !decision_time_matches || !window_authority_matches {
                return Err(InternalError::invariant());
            }
        }
    }
    Ok(())
}

fn validate_window(
    window: Option<&FleetRootFundingWindowRecord>,
    window_secs: u64,
    maximum_cycles: u128,
) -> Result<(), InternalError> {
    if window_secs == 0
        || window.is_some_and(|window| {
            window.window_start_secs % window_secs != 0
                || window.spent_cycles.to_u128() > maximum_cycles
        })
    {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn commit_funding_transition(
    current: &FleetCoordinatorFundingRecord,
    next: FleetCoordinatorFundingRecord,
) -> Result<FleetCoordinatorCommitOutcome, InternalError> {
    FleetCoordinatorFundingStore::commit_transition(current, next).map_err(|error| match error {
        FleetCoordinatorCommitError::ConflictingState => InternalError::conflict(),
        FleetCoordinatorCommitError::Uninitialized => InternalError::unavailable(),
    })
}

const fn map_no_grant_reason(reason: FleetRootGrantNoGrantReason) -> FleetRootFundingNoGrantReason {
    match reason {
        FleetRootGrantNoGrantReason::CooldownActive => {
            FleetRootFundingNoGrantReason::CooldownActive
        }
        FleetRootGrantNoGrantReason::CoordinatorReserveUnavailable => {
            FleetRootFundingNoGrantReason::CoordinatorReserveUnavailable
        }
        FleetRootGrantNoGrantReason::FleetAutomaticCapExhausted => {
            FleetRootFundingNoGrantReason::FleetAutomaticCapExhausted
        }
        FleetRootGrantNoGrantReason::FleetWindowExhausted => {
            FleetRootFundingNoGrantReason::FleetWindowExhausted
        }
        FleetRootGrantNoGrantReason::FundingDisabled => {
            FleetRootFundingNoGrantReason::FundingDisabled
        }
        FleetRootGrantNoGrantReason::InvalidRequest => {
            FleetRootFundingNoGrantReason::InvalidRequest
        }
        FleetRootGrantNoGrantReason::PolicyMismatch => {
            FleetRootFundingNoGrantReason::PolicyMismatch
        }
        FleetRootGrantNoGrantReason::RegistryStale => FleetRootFundingNoGrantReason::RegistryStale,
        FleetRootGrantNoGrantReason::RootIneligible => {
            FleetRootFundingNoGrantReason::RootIneligible
        }
        FleetRootGrantNoGrantReason::RootAutomaticCapExhausted => {
            FleetRootFundingNoGrantReason::RootAutomaticCapExhausted
        }
        FleetRootGrantNoGrantReason::RootWindowExhausted => {
            FleetRootFundingNoGrantReason::RootWindowExhausted
        }
    }
}
