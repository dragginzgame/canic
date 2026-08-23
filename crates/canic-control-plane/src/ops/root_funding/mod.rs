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
        RootFundingPolicyRotationRecord, RootFundingPolicyRotationTerminalRecord,
        RootFundingRecord, RootFundingStore, RootFundingTerminalOperationRecord,
    },
    view::root_funding::{RootFundingAcceptanceDisposition, RootFundingAuthorityView},
};
use canic_core::{
    cdk::types::Cycles,
    control_plane_support::{error::InternalError, ops::icp_refill::IcpRefillStoreOps},
    dto::fleet_funding::{
        FleetFundingPolicyRotationRootPrepareRequest, FleetFundingPolicyRotationRootReceipt,
        FleetRootFundingAcceptanceReceipt, FleetRootFundingAcceptanceRequest,
        FleetRootFundingRequest, FleetRootFundingResponse,
    },
    shared_support::fleet_funding_policy::{
        fleet_root_funding_operation_id, fleet_subnet_root_funding_policy_hash,
        validate_fleet_subnet_root_funding_authority,
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
        if current.rotation_current.is_some() {
            return Err(InternalError::conflict());
        }
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

    pub(crate) fn policy_rotation_in_progress() -> bool {
        RootFundingStore::export()
            .current
            .is_some_and(|record| record.rotation_current.is_some())
    }

    /// Return an exact prepare replay without disturbing the already-correct timer state.
    pub(crate) fn policy_rotation_prepare_replay(
        authority: &RootFundingAuthorityView,
        request: &FleetFundingPolicyRotationRootPrepareRequest,
    ) -> Result<Option<FleetFundingPolicyRotationRootReceipt>, InternalError> {
        let current = current(authority)?;
        if let Some(last) = current.rotation_last.as_ref()
            && &last.request == request
        {
            return Ok(Some(last.receipt.clone()));
        }
        if let Some(active) = current.rotation_current.as_ref() {
            if &active.request == request {
                return Ok(Some(active.prepared_receipt.clone()));
            }
            return Err(InternalError::conflict());
        }
        Ok(None)
    }

    /// Prepare or exactly replay the sole Root-owned rotation fence.
    pub(crate) fn prepare_policy_rotation(
        authority: &RootFundingAuthorityView,
        request: FleetFundingPolicyRotationRootPrepareRequest,
        recorded_at_ns: u64,
    ) -> Result<FleetFundingPolicyRotationRootReceipt, InternalError> {
        let current = current(authority)?;
        if let Some(last) = current.rotation_last.as_ref()
            && last.request == request
        {
            return Ok(last.receipt.clone());
        }
        if let Some(active) = current.rotation_current.as_ref() {
            if active.request == request {
                return Ok(active.prepared_receipt.clone());
            }
            return Err(InternalError::conflict());
        }
        let policy_hash = fleet_subnet_root_funding_policy_hash(&authority.funding);
        let usage = &request.root.predecessor_usage;
        let current_usage = canic_core::dto::fleet_funding::FleetFundingPolicyUsage {
            historical_automatic_grants: current.historical_automatic_grants,
            historical_automatic_cycles: current.historical_automatic_cycles.clone(),
            generation_automatic_grants: current.automatic_grants,
            generation_automatic_cycles: current.automatic_cycles.clone(),
        };
        let usage_matches = usage == &current_usage;
        if request.operation_id == [0; 32]
            || request.plan_digest == [0; 32]
            || request.root.fleet_subnet_root != authority.fleet_subnet_root
            || request.predecessor_registry != authority.registry
            || request.predecessor_generation != current.policy_generation
            || request
                .predecessor_generation
                .checked_add(1)
                .is_none_or(|generation| generation != request.successor_generation)
            || request.root.predecessor_policy_hash != policy_hash
            || !usage_matches
            || current.current.is_some()
            || !authority.funding_eligible
        {
            return Err(InternalError::conflict());
        }
        let mut next_authority = authority.funding.clone();
        next_authority.root_funding = request.root.proposed_policy.clone();
        validate_fleet_subnet_root_funding_authority(&next_authority, false)
            .map_err(|_| InternalError::invalid_input())?;
        let receipt = FleetFundingPolicyRotationRootReceipt {
            operation_id: request.operation_id,
            plan_digest: request.plan_digest,
            fleet_subnet_root: authority.fleet_subnet_root,
            predecessor_generation: request.predecessor_generation,
            successor_generation: request.successor_generation,
            prepared: true,
            activated: false,
            recorded_at_ns,
        };
        let mut next = current.clone();
        next.rotation_current = Some(RootFundingPolicyRotationRecord {
            request,
            prepared_receipt: receipt.clone(),
        });
        let next = validate_record(authority, next)?;
        commit_transition(&current, next)?;
        Ok(receipt)
    }

    /// Return the exact prepared rotation without consulting a possibly mixed mirror.
    pub(crate) fn prepared_policy_rotation()
    -> Result<RootFundingPolicyRotationRecord, InternalError> {
        RootFundingStore::export()
            .current
            .and_then(|record| record.rotation_current)
            .ok_or_else(InternalError::unavailable)
    }

    /// Return an exact completed activation replay before consulting active state.
    pub(crate) fn completed_policy_rotation(
        authority: &RootFundingAuthorityView,
        request: &canic_core::dto::fleet_funding::FleetFundingPolicyRotationRootActivateRequest,
    ) -> Result<Option<FleetFundingPolicyRotationRootReceipt>, InternalError> {
        let raw = RootFundingStore::export()
            .current
            .ok_or_else(InternalError::unavailable)?;
        let Some(last) = raw.rotation_last.as_ref() else {
            return Ok(None);
        };
        let prepared = &last.request;
        if last.receipt.operation_id != request.operation_id
            || last.receipt.plan_digest != request.plan_digest
            || last.receipt.fleet_subnet_root != request.fleet_subnet_root
            || last.receipt.predecessor_generation != request.predecessor_generation
            || last.receipt.successor_generation != request.successor_generation
        {
            return Ok(None);
        }
        if prepared.operation_id != request.operation_id
            || prepared.plan_digest != request.plan_digest
            || prepared.predecessor_registry != request.predecessor_registry
            || prepared.predecessor_generation != request.predecessor_generation
            || prepared.successor_generation != request.successor_generation
            || prepared.root.fleet_subnet_root != request.fleet_subnet_root
            || authority.registry != request.successor_registry
            || authority.fleet_subnet_root != request.fleet_subnet_root
            || raw.policy_generation != request.successor_generation
        {
            return Err(InternalError::conflict());
        }
        Ok(Some(last.receipt.clone()))
    }

    /// Commit the successor generation after protected authority and mirror converge.
    pub(crate) fn complete_policy_rotation(
        authority: &RootFundingAuthorityView,
        operation_id: [u8; 32],
        plan_digest: [u8; 32],
        recorded_at_ns: u64,
    ) -> Result<FleetFundingPolicyRotationRootReceipt, InternalError> {
        let raw = RootFundingStore::export()
            .current
            .ok_or_else(InternalError::unavailable)?;
        if let Some(last) = raw.rotation_last.as_ref()
            && last.receipt.operation_id == operation_id
            && last.receipt.plan_digest == plan_digest
            && last.receipt.activated
        {
            return Ok(last.receipt.clone());
        }
        let rotation = raw
            .rotation_current
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        let request = &rotation.request;
        let mut expected_authority = authority.funding.clone();
        expected_authority.root_funding = request.root.proposed_policy.clone();
        if request.operation_id != operation_id
            || request.plan_digest != plan_digest
            || request.root.fleet_subnet_root != authority.fleet_subnet_root
            || request.successor_generation != raw.policy_generation.saturating_add(1)
            || expected_authority != authority.funding
        {
            return Err(InternalError::conflict());
        }
        let mut next = raw.clone();
        next.policy_generation = request.successor_generation;
        next.historical_automatic_grants = next
            .historical_automatic_grants
            .checked_add(u64::from(next.automatic_grants))
            .ok_or_else(InternalError::invariant)?;
        next.historical_automatic_cycles = next
            .historical_automatic_cycles
            .to_u128()
            .checked_add(next.automatic_cycles.to_u128())
            .ok_or_else(InternalError::invariant)?
            .into();
        next.automatic_grants = 0;
        next.automatic_cycles = 0_u128.into();
        let receipt = FleetFundingPolicyRotationRootReceipt {
            operation_id,
            plan_digest,
            fleet_subnet_root: authority.fleet_subnet_root,
            predecessor_generation: request.predecessor_generation,
            successor_generation: request.successor_generation,
            prepared: true,
            activated: true,
            recorded_at_ns,
        };
        next.rotation_current = None;
        next.rotation_last = Some(RootFundingPolicyRotationTerminalRecord {
            request: request.clone(),
            receipt: receipt.clone(),
        });
        let next = validate_record(authority, next)?;
        commit_transition(&raw, next)?;
        Ok(receipt)
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
            policy_generation: funding.policy_generation,
            funding_profile: authority.funding.root_funding.funding_profile,
            policy_hash: fleet_subnet_root_funding_policy_hash(&authority.funding),
            root_policy: authority.funding.root_funding.clone(),
            current_operation: funding.current.map(|active| active.request),
            last_result: funding.last.map(|last| last.response),
            historical_automatic_grants: funding.historical_automatic_grants,
            historical_automatic_cycles: funding.historical_automatic_cycles,
            automatic_grants: funding.automatic_grants,
            automatic_cycles: funding.automatic_cycles,
            rotation_current: funding
                .rotation_current
                .map(|rotation| rotation.prepared_receipt),
            rotation_last: funding.rotation_last.map(|rotation| rotation.receipt),
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
        || record.policy_generation != 1
        || record.historical_automatic_grants != 0
        || record.historical_automatic_cycles.to_u128() != 0
        || record.automatic_grants != 0
        || record.automatic_cycles.to_u128() != 0
        || record.current.is_some()
        || record.last.is_some()
        || record.rotation_current.is_some()
        || record.rotation_last.is_some()
    {
        return Err(InternalError::invariant());
    }
    Ok(record)
}

#[expect(
    clippy::too_many_lines,
    reason = "one Root journal validator composes funding and rotation invariants"
)]
fn validate_record(
    authority: &RootFundingAuthorityView,
    record: RootFundingRecord,
) -> Result<RootFundingRecord, InternalError> {
    if record.schema_version != ROOT_FUNDING_SCHEMA_VERSION || record.policy_generation == 0 {
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
        validate_request_identity(authority, &last.request)?;
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
    if let Some(last) = record.rotation_last.as_ref()
        && (last.receipt.operation_id == [0; 32]
            || last.receipt.plan_digest == [0; 32]
            || !last.receipt.prepared
            || !last.receipt.activated
            || last.receipt.fleet_subnet_root != authority.fleet_subnet_root
            || last.receipt.successor_generation > record.policy_generation
            || last.receipt.predecessor_generation.checked_add(1)
                != Some(last.receipt.successor_generation)
            || !rotation_receipt_matches(&last.receipt, &last.request)
            || last.request.successor_generation != record.policy_generation
            || last.request.root.proposed_policy != authority.funding.root_funding
            || last.request.predecessor_registry.authority != authority.registry.authority)
    {
        return Err(InternalError::invariant());
    }
    if let Some(rotation) = record.rotation_current.as_ref() {
        let request = &rotation.request;
        let usage = &request.root.predecessor_usage;
        if record.current.is_some()
            || request.operation_id == [0; 32]
            || request.plan_digest == [0; 32]
            || request.root.fleet_subnet_root != authority.fleet_subnet_root
            || request.predecessor_registry != authority.registry
            || request.predecessor_generation != record.policy_generation
            || request.predecessor_generation.checked_add(1) != Some(request.successor_generation)
            || request.root.predecessor_policy_hash
                != fleet_subnet_root_funding_policy_hash(&authority.funding)
            || usage.historical_automatic_grants != record.historical_automatic_grants
            || usage.historical_automatic_cycles != record.historical_automatic_cycles
            || usage.generation_automatic_grants != record.automatic_grants
            || usage.generation_automatic_cycles != record.automatic_cycles
            || !rotation_receipt_matches(&rotation.prepared_receipt, request)
            || !rotation.prepared_receipt.prepared
            || rotation.prepared_receipt.activated
        {
            return Err(InternalError::invariant());
        }
        let mut successor = authority.funding.clone();
        successor.root_funding = request.root.proposed_policy.clone();
        validate_fleet_subnet_root_funding_authority(&successor, false)
            .map_err(|_| InternalError::invariant())?;
    }
    Ok(record)
}

fn validate_request_identity(
    authority: &RootFundingAuthorityView,
    request: &FleetRootFundingRequest,
) -> Result<(), InternalError> {
    let coordinator = authority.registry.authority.binding.coordinator;
    let expected_id = fleet_root_funding_operation_id(
        coordinator,
        authority.fleet_subnet_root,
        request.operation_sequence,
        &request.expected_registry,
        request.observed_balance.to_u128(),
        request.requested_cycles.to_u128(),
        request.policy_hash,
    );
    if request.operation_sequence == 0
        || request.operation_id != expected_id
        || request.expected_registry.authority != authority.registry.authority
        || request.requested_cycles.to_u128() == 0
    {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_request(
    authority: &RootFundingAuthorityView,
    request: &FleetRootFundingRequest,
) -> Result<(), InternalError> {
    validate_request_identity(authority, request)?;
    let policy_hash = fleet_subnet_root_funding_policy_hash(&authority.funding);
    let target_matches = request
        .observed_balance
        .to_u128()
        .checked_add(request.requested_cycles.to_u128())
        == Some(authority.funding.root_funding.target_balance.to_u128());
    if request.operation_sequence == 0
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

fn rotation_receipt_matches(
    receipt: &FleetFundingPolicyRotationRootReceipt,
    request: &FleetFundingPolicyRotationRootPrepareRequest,
) -> bool {
    receipt.operation_id == request.operation_id
        && receipt.plan_digest == request.plan_digest
        && receipt.fleet_subnet_root == request.root.fleet_subnet_root
        && receipt.predecessor_generation == request.predecessor_generation
        && receipt.successor_generation == request.successor_generation
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
