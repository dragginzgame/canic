//! Module: ops::fleet_coordinator::root_funding
//!
//! Responsibility: validate and transition the Coordinator's bounded Root-funding ledger.
//! Does not own: endpoint authorization, clocks, live balances, or inter-canister calls.
//! Boundary: workflow supplies authenticated callers and ambient observations after admission.

use super::FleetCoordinatorOps;
use crate::{
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
            FleetFundingWindowSnapshot, FleetRootGrantAuthorityMatch, FleetRootGrantAvailability,
            FleetRootGrantDecision, FleetRootGrantDecisionInput, FleetRootGrantNoGrantReason,
            decide_fleet_root_grant,
        },
    },
    dto::{
        fleet_funding::{
            FleetRootFundingAcceptanceReceipt, FleetRootFundingAcceptanceRequest,
            FleetRootFundingNoGrantReason, FleetRootFundingNoGrantReceipt, FleetRootFundingRequest,
            FleetRootFundingResponse,
        },
        fleet_registry::{FleetRegistryVersion, FleetSubnetRootEntry, FleetSubnetRootStatus},
    },
    ids::{FLEET_ROOT_FUNDING_CALL_RESERVATION_CYCLES, MAX_FLEET_ROOT_FUNDING_SLOTS},
    shared_support::fleet_funding_policy::{
        fleet_root_funding_operation_id, fleet_subnet_root_funding_policy_hash,
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
            root_is_eligible: root.status == FleetSubnetRootStatus::Active,
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

fn validate_funding_record(
    registry: &FleetCoordinatorRegistryRecord,
    funding: FleetCoordinatorFundingRecord,
) -> Result<FleetCoordinatorFundingRecord, InternalError> {
    if funding.schema_version != FLEET_COORDINATOR_FUNDING_SCHEMA_VERSION
        || funding.roots.len() > MAX_FLEET_ROOT_FUNDING_SLOTS
        || funding.roots.len() > registry.registry.fleet_subnet_roots.len()
    {
        return Err(InternalError::invariant());
    }
    let coordinator_policy = registry
        .root_funding
        .as_ref()
        .ok_or_else(InternalError::invariant)?;
    validate_window(
        funding.fleet_window.as_ref(),
        coordinator_policy.budget.window_secs,
        coordinator_policy.budget.maximum_cycles.to_u128(),
    )?;

    let mut previous_root = None;
    let mut fleet_reservations = BTreeMap::<u64, u128>::new();
    for root_ledger in &funding.roots {
        if previous_root.is_some_and(|previous| previous >= root_ledger.fleet_subnet_root) {
            return Err(InternalError::invariant());
        }
        previous_root = Some(root_ledger.fleet_subnet_root);
        let root = exact_registry_root(registry, root_ledger.fleet_subnet_root)?;
        let policy_hash = fleet_subnet_root_funding_policy_hash(&root.funding);
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
    Ok(funding)
}

fn validate_root_ledger(
    registry: &FleetCoordinatorRegistryRecord,
    root: &FleetSubnetRootEntry,
    ledger: &FleetRootFundingLedgerRecord,
    policy_hash: [u8; 32],
) -> Result<(), InternalError> {
    if let Some(last) = ledger.last.as_ref() {
        validate_request(registry, root, &last.request, policy_hash)?;
        validate_terminal_result(registry, root, last)?;
    }
    if let Some(active) = ledger.current.as_ref() {
        if active.fleet_subnet_root != root.fleet_subnet_root
            || active.call_reservation_cycles.to_u128()
                != FLEET_ROOT_FUNDING_CALL_RESERVATION_CYCLES
        {
            return Err(InternalError::invariant());
        }
        validate_request(registry, root, &active.request, policy_hash)?;
        let expected_sequence = ledger.last.as_ref().map_or(Some(1), |last| {
            last.request.operation_sequence.checked_add(1)
        });
        if Some(active.request.operation_sequence) != expected_sequence {
            return Err(InternalError::invariant());
        }
        let root_budget = &root.funding.root_funding.budget;
        if active.root_window_start_secs % root_budget.window_secs != 0
            || active.request.requested_cycles.to_u128() > root_budget.maximum_cycles.to_u128()
        {
            return Err(InternalError::invariant());
        }
        let fleet_budget = &registry
            .root_funding
            .as_ref()
            .ok_or_else(InternalError::invariant)?
            .budget;
        if active.fleet_window_start_secs % fleet_budget.window_secs != 0
            || active.request.requested_cycles.to_u128() > fleet_budget.maximum_cycles.to_u128()
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
    policy_hash: [u8; 32],
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
    if request.operation_sequence == 0
        || request.operation_id != expected
        || request.policy_hash != policy_hash
    {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_terminal_result(
    registry: &FleetCoordinatorRegistryRecord,
    root: &FleetSubnetRootEntry,
    result: &CoordinatorRootGrantResultRecord,
) -> Result<(), InternalError> {
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
            let has_unexpected_window_authority = receipt.reason
                != FleetRootFundingNoGrantReason::RootRejected
                && (result.fleet_window_start_secs.is_some()
                    || result.root_window_start_secs.is_some());
            if !request_matches || !decision_time_matches || has_unexpected_window_authority {
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
        FleetRootGrantNoGrantReason::RootWindowExhausted => {
            FleetRootFundingNoGrantReason::RootWindowExhausted
        }
    }
}
