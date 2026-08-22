//! Module: ops::root_funding
//!
//! Responsibility: validate and transition one Root's bounded Coordinator-funding journal.
//! Does not own: caller authentication, Registry mirror validation, clocks, balances, or cycles.
//! Boundary: workflow supplies validated authority and ambient observations around each transition.

use crate::{
    dto::root::{RootFundingStatusResponse, RootIcpRefillStatusResponse},
    storage::stable::root_funding::{
        ROOT_FUNDING_SCHEMA_VERSION, RootFundingActiveOperationRecord,
        RootFundingActivePhaseRecord, RootFundingCommitError, RootFundingCommitOutcome,
        RootFundingRecord, RootFundingStore, RootFundingTerminalOperationRecord,
    },
    view::root_funding::{RootFundingAcceptanceDisposition, RootFundingAuthorityView},
};
use canic_core::{
    cdk::types::Cycles,
    control_plane_support::{error::InternalError, ops::icp_refill::IcpRefillStoreOps},
    dto::fleet_funding::{
        FleetRootFundingAcceptanceReceipt, FleetRootFundingAcceptanceRequest,
        FleetRootFundingRequest, FleetRootFundingResponse,
    },
    shared_support::fleet_funding_policy::{
        fleet_root_funding_operation_id, fleet_subnet_root_funding_policy_hash,
    },
};

/// Deterministic Root funding journal operations.
pub struct RootFundingOps;

impl RootFundingOps {
    #[must_use]
    pub(crate) fn compile_genesis() -> RootFundingRecord {
        RootFundingRecord::default()
    }

    pub(crate) fn commit_genesis(
        record: RootFundingRecord,
    ) -> Result<RootFundingCommitOutcome, InternalError> {
        let record = validate_record_without_authority(record)?;
        RootFundingStore::commit_genesis(record).map_err(map_commit_error)
    }

    pub(crate) fn prepare_request(
        authority: &RootFundingAuthorityView,
        observed_balance: u128,
        opened_at_ns: u64,
    ) -> Result<FleetRootFundingRequest, InternalError> {
        let current = current(authority)?;
        if let Some(active) = current.current.as_ref() {
            return Ok(active.request.clone());
        }
        if !authority.funding_eligible {
            return Err(InternalError::conflict());
        }
        let policy = &authority.funding.root_funding;
        if observed_balance > policy.request_threshold.to_u128() {
            return Err(InternalError::invalid_input());
        }
        let requested_cycles = policy
            .target_balance
            .to_u128()
            .checked_sub(observed_balance)
            .filter(|requested| *requested != 0)
            .ok_or_else(InternalError::invalid_input)?;
        let operation_sequence = current
            .last
            .as_ref()
            .map_or(Some(1), |last| {
                last.request.operation_sequence.checked_add(1)
            })
            .ok_or_else(InternalError::resource_exhausted)?;
        let coordinator = authority.registry.authority.binding.coordinator;
        let policy_hash = fleet_subnet_root_funding_policy_hash(&authority.funding);
        let request = FleetRootFundingRequest {
            operation_id: fleet_root_funding_operation_id(
                coordinator,
                authority.fleet_subnet_root,
                operation_sequence,
                &authority.registry,
                observed_balance,
                requested_cycles,
                policy_hash,
            ),
            operation_sequence,
            expected_registry: authority.registry.clone(),
            observed_balance: observed_balance.into(),
            requested_cycles: requested_cycles.into(),
            policy_hash,
        };
        let mut next = current.clone();
        next.current = Some(RootFundingActiveOperationRecord {
            request: request.clone(),
            phase: RootFundingActivePhaseRecord::CoordinatorRequested,
            opened_at_ns,
            updated_at_ns: opened_at_ns,
        });
        let next = validate_record(authority, next)?;
        commit_transition(&current, next)?;
        Ok(request)
    }

    pub(crate) fn current_request(
        authority: &RootFundingAuthorityView,
    ) -> Result<Option<FleetRootFundingRequest>, InternalError> {
        Ok(current(authority)?.current.map(|active| active.request))
    }

    pub(crate) fn status(
        authority: &RootFundingAuthorityView,
        cycles_funding_enabled: bool,
        current_cycles: u128,
        now_secs: u64,
    ) -> Result<RootFundingStatusResponse, InternalError> {
        let funding = current(authority)?;
        let (
            icp_window_start_secs,
            icp_window_reserved_e8s,
            automatic_icp_refills,
            automatic_icp_refill_e8s,
        ) = authority.funding.icp_refill.as_ref().map_or_else(
            || (None, 0, 0, 0),
            |policy| {
                let window_start_secs = now_secs / policy.window_secs * policy.window_secs;
                let usage = IcpRefillStoreOps::policy_usage(window_start_secs);
                (
                    Some(window_start_secs),
                    usage.window_reserved_e8s,
                    usage.automatic_completed_refills,
                    usage.automatic_completed_refill_e8s,
                )
            },
        );
        let latest_icp_refill =
            IcpRefillStoreOps::latest_operation()?.map(|operation| RootIcpRefillStatusResponse {
                trigger: operation.trigger,
                amount_e8s: operation.amount_e8s,
                fee_e8s: operation.fee_e8s,
                budget_window_start_secs: operation.budget_window_start_secs,
                resumable: IcpRefillStoreOps::is_resumable(&operation),
                response: IcpRefillStoreOps::to_response(&operation),
            });
        Ok(RootFundingStatusResponse {
            fleet_subnet_root: authority.fleet_subnet_root,
            lifecycle_status: authority.status,
            funding_eligible: authority.funding_eligible,
            cycles_funding_enabled,
            current_cycles: Cycles::new(current_cycles),
            funding_profile: authority.funding.root_funding.funding_profile,
            policy_hash: fleet_subnet_root_funding_policy_hash(&authority.funding),
            root_policy: authority.funding.root_funding.clone(),
            current_operation: funding.current.map(|active| active.request),
            last_result: funding.last.map(|last| last.response),
            automatic_grants: funding.automatic_grants,
            automatic_cycles: funding.automatic_cycles,
            icp_refill_policy: authority.funding.icp_refill.clone(),
            icp_window_start_secs,
            icp_window_reserved_e8s,
            automatic_icp_refills,
            automatic_icp_refill_e8s,
            latest_icp_refill,
        })
    }

    pub(crate) fn prepare_acceptance(
        authority: &RootFundingAuthorityView,
        request: &FleetRootFundingAcceptanceRequest,
        incoming_cycles: u128,
        current_balance: u128,
    ) -> Result<RootFundingAcceptanceDisposition, InternalError> {
        let funding = current(authority)?;
        if incoming_cycles != request.granted_cycles.to_u128() {
            return Err(InternalError::invalid_input());
        }

        if let Some(receipt) = accepted_receipt(&funding, request) {
            return Ok(RootFundingAcceptanceDisposition::Replay(Box::new(
                receipt.clone(),
            )));
        }

        let active = funding
            .current
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        if !matches!(
            active.phase,
            RootFundingActivePhaseRecord::CoordinatorRequested
        ) || acceptance_request(&active.request) != *request
            || request.expected_registry != authority.registry
            || request.policy_hash != fleet_subnet_root_funding_policy_hash(&authority.funding)
            || current_balance > authority.funding.root_funding.request_threshold.to_u128()
        {
            return Err(InternalError::conflict());
        }
        Ok(RootFundingAcceptanceDisposition::Fresh)
    }

    pub(crate) fn record_acceptance(
        authority: &RootFundingAuthorityView,
        request: &FleetRootFundingAcceptanceRequest,
        accepted_at_ns: u64,
    ) -> Result<FleetRootFundingAcceptanceReceipt, InternalError> {
        let current = current(authority)?;
        if let Some(receipt) = accepted_receipt(&current, request) {
            return Ok(receipt.clone());
        }
        let mut next = current.clone();
        let active = next.current.as_mut().ok_or_else(InternalError::conflict)?;
        if !matches!(
            active.phase,
            RootFundingActivePhaseRecord::CoordinatorRequested
        ) || acceptance_request(&active.request) != *request
            || accepted_at_ns < active.opened_at_ns
        {
            return Err(InternalError::conflict());
        }
        let receipt = FleetRootFundingAcceptanceReceipt {
            request: request.clone(),
            fleet_subnet_root: authority.fleet_subnet_root,
            coordinator: authority.registry.authority.binding.coordinator,
            accepted_at_ns,
        };
        next.automatic_grants = next
            .automatic_grants
            .checked_add(1)
            .ok_or_else(InternalError::invariant)?;
        next.automatic_cycles = next
            .automatic_cycles
            .to_u128()
            .checked_add(request.granted_cycles.to_u128())
            .ok_or_else(InternalError::invariant)?
            .into();
        active.phase = RootFundingActivePhaseRecord::GrantAccepted(Box::new(receipt.clone()));
        active.updated_at_ns = accepted_at_ns;
        let next = validate_record(authority, next)?;
        commit_transition(&current, next)?;
        Ok(receipt)
    }

    pub(crate) fn record_response(
        authority: &RootFundingAuthorityView,
        response: FleetRootFundingResponse,
        completed_at_ns: u64,
    ) -> Result<FleetRootFundingResponse, InternalError> {
        let current = current(authority)?;
        if current
            .last
            .as_ref()
            .is_some_and(|last| last.response == response)
        {
            return Ok(response);
        }
        let active = current
            .current
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        if response_request(&response) != active.request || completed_at_ns < active.updated_at_ns {
            return Err(InternalError::conflict());
        }
        match (&active.phase, &response) {
            (
                RootFundingActivePhaseRecord::GrantAccepted(expected),
                FleetRootFundingResponse::Granted(receipt),
            ) if expected.as_ref() == receipt && receipt.accepted_at_ns <= completed_at_ns => {}
            (
                RootFundingActivePhaseRecord::CoordinatorRequested,
                FleetRootFundingResponse::NoGrant(receipt),
            ) if receipt.decided_at_ns <= completed_at_ns => {}
            _ => return Err(InternalError::conflict()),
        }

        let mut next = current.clone();
        next.current = None;
        next.last = Some(RootFundingTerminalOperationRecord {
            request: active.request.clone(),
            response: response.clone(),
            opened_at_ns: active.opened_at_ns,
            completed_at_ns,
        });
        let next = validate_record(authority, next)?;
        commit_transition(&current, next)?;
        Ok(response)
    }

    #[cfg(test)]
    pub(crate) fn current_for_test(
        authority: &RootFundingAuthorityView,
    ) -> Result<RootFundingRecord, InternalError> {
        current(authority)
    }
}

fn current(authority: &RootFundingAuthorityView) -> Result<RootFundingRecord, InternalError> {
    let record = RootFundingStore::export()
        .current
        .ok_or_else(InternalError::unavailable)?;
    validate_record(authority, record)
}

fn validate_record_without_authority(
    record: RootFundingRecord,
) -> Result<RootFundingRecord, InternalError> {
    if record.schema_version != ROOT_FUNDING_SCHEMA_VERSION
        || record.automatic_grants != 0
        || record.automatic_cycles.to_u128() != 0
        || record.current.is_some()
        || record.last.is_some()
    {
        return Err(InternalError::invariant());
    }
    Ok(record)
}

fn validate_record(
    authority: &RootFundingAuthorityView,
    record: RootFundingRecord,
) -> Result<RootFundingRecord, InternalError> {
    if record.schema_version != ROOT_FUNDING_SCHEMA_VERSION {
        return Err(InternalError::invariant());
    }
    if record.automatic_grants > authority.funding.root_funding.maximum_automatic_grants
        || record.automatic_cycles.to_u128()
            > authority
                .funding
                .root_funding
                .maximum_automatic_cycles
                .to_u128()
    {
        return Err(InternalError::invariant());
    }
    if authority.registry.authority.binding.coordinator == authority.fleet_subnet_root
        || authority.registry.authority.binding.coordinator == candid::Principal::anonymous()
        || authority.fleet_subnet_root == candid::Principal::anonymous()
    {
        return Err(InternalError::invariant());
    }
    if let Some(last) = record.last.as_ref() {
        validate_request(authority, &last.request)?;
        if response_request(&last.response) != last.request
            || last.completed_at_ns < last.opened_at_ns
        {
            return Err(InternalError::invariant());
        }
        match &last.response {
            FleetRootFundingResponse::Granted(receipt)
                if receipt.fleet_subnet_root == authority.fleet_subnet_root
                    && receipt.coordinator == authority.registry.authority.binding.coordinator
                    && receipt.accepted_at_ns >= last.opened_at_ns
                    && receipt.accepted_at_ns <= last.completed_at_ns => {}
            FleetRootFundingResponse::NoGrant(receipt)
                if receipt.decided_at_ns >= last.opened_at_ns
                    && receipt.decided_at_ns <= last.completed_at_ns => {}
            _ => return Err(InternalError::invariant()),
        }
    }
    if let Some(active) = record.current.as_ref() {
        validate_request(authority, &active.request)?;
        if active.updated_at_ns < active.opened_at_ns {
            return Err(InternalError::invariant());
        }
        let expected_sequence = record.last.as_ref().map_or(Some(1), |last| {
            last.request.operation_sequence.checked_add(1)
        });
        if Some(active.request.operation_sequence) != expected_sequence {
            return Err(InternalError::invariant());
        }
        if let RootFundingActivePhaseRecord::GrantAccepted(receipt) = &active.phase
            && (receipt.request != acceptance_request(&active.request)
                || receipt.fleet_subnet_root != authority.fleet_subnet_root
                || receipt.coordinator != authority.registry.authority.binding.coordinator
                || receipt.accepted_at_ns < active.opened_at_ns
                || receipt.accepted_at_ns != active.updated_at_ns)
        {
            return Err(InternalError::invariant());
        }
    }
    Ok(record)
}

fn validate_request(
    authority: &RootFundingAuthorityView,
    request: &FleetRootFundingRequest,
) -> Result<(), InternalError> {
    let coordinator = authority.registry.authority.binding.coordinator;
    let policy_hash = fleet_subnet_root_funding_policy_hash(&authority.funding);
    let expected_id = fleet_root_funding_operation_id(
        coordinator,
        authority.fleet_subnet_root,
        request.operation_sequence,
        &request.expected_registry,
        request.observed_balance.to_u128(),
        request.requested_cycles.to_u128(),
        request.policy_hash,
    );
    let target_matches = request
        .observed_balance
        .to_u128()
        .checked_add(request.requested_cycles.to_u128())
        == Some(authority.funding.root_funding.target_balance.to_u128());
    if request.operation_sequence == 0
        || request.operation_id != expected_id
        || request.expected_registry.authority != authority.registry.authority
        || request.policy_hash != policy_hash
        || request.requested_cycles.to_u128() == 0
        || request.observed_balance.to_u128()
            > authority.funding.root_funding.request_threshold.to_u128()
        || !target_matches
    {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn acceptance_request(request: &FleetRootFundingRequest) -> FleetRootFundingAcceptanceRequest {
    FleetRootFundingAcceptanceRequest {
        operation_id: request.operation_id,
        operation_sequence: request.operation_sequence,
        expected_registry: request.expected_registry.clone(),
        observed_balance: request.observed_balance.clone(),
        granted_cycles: request.requested_cycles.clone(),
        policy_hash: request.policy_hash,
    }
}

fn response_request(response: &FleetRootFundingResponse) -> FleetRootFundingRequest {
    match response {
        FleetRootFundingResponse::Granted(receipt) => FleetRootFundingRequest {
            operation_id: receipt.request.operation_id,
            operation_sequence: receipt.request.operation_sequence,
            expected_registry: receipt.request.expected_registry.clone(),
            observed_balance: receipt.request.observed_balance.clone(),
            requested_cycles: receipt.request.granted_cycles.clone(),
            policy_hash: receipt.request.policy_hash,
        },
        FleetRootFundingResponse::NoGrant(receipt) => receipt.request.clone(),
    }
}

fn accepted_receipt<'a>(
    funding: &'a RootFundingRecord,
    request: &FleetRootFundingAcceptanceRequest,
) -> Option<&'a FleetRootFundingAcceptanceReceipt> {
    if let Some(RootFundingActiveOperationRecord {
        phase: RootFundingActivePhaseRecord::GrantAccepted(receipt),
        ..
    }) = funding.current.as_ref()
        && receipt.request == *request
    {
        return Some(receipt);
    }
    match funding.last.as_ref().map(|last| &last.response) {
        Some(FleetRootFundingResponse::Granted(receipt)) if receipt.request == *request => {
            Some(receipt)
        }
        _ => None,
    }
}

fn commit_transition(
    current: &RootFundingRecord,
    next: RootFundingRecord,
) -> Result<RootFundingCommitOutcome, InternalError> {
    RootFundingStore::commit_transition(current, next).map_err(map_commit_error)
}

const fn map_commit_error(error: RootFundingCommitError) -> InternalError {
    match error {
        RootFundingCommitError::ConflictingState => InternalError::conflict(),
        RootFundingCommitError::Uninitialized => InternalError::unavailable(),
    }
}

#[cfg(test)]
mod tests;
