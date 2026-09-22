//! Early Fleet checks using existing signer, network and retained-state owners.
//!
//! No compilation, plan generation, journal mutation or paid effect occurs here.

#[cfg(test)]
mod tests;

use crate::{
    fleet_ensure::{
        model::FleetEnsureCompletion,
        ops::{
            EnsurePaths, EnsureStateError,
            operator_mint::transport::{OperatorMintTransport, OperatorMintTransportError},
            read_journal, read_plan,
        },
        policy::{EnsurePolicyError, validate_path_labels},
        view::readiness::{
            FleetReadiness, ReadinessBlocker, ReadinessConversionQuote, ReadinessUnresolved,
            RetainedReadinessOperation,
        },
    },
    icp::IcpCli,
    network::{NetworkIdentityError, resolve_canonical_network_id_from_root},
};
use candid::Principal;
use canic_core::ids::CanonicalNetworkId;
use std::path::Path;
use thiserror::Error;

/// Inputs available before any release build exists. Funding is a caller estimate.
pub struct FleetReadinessRequest<'a> {
    pub workspace: &'a Path,
    pub environment: &'a str,
    pub fleet: &'a str,
    pub icp_executable: &'a str,
    pub signing_identity: Option<&'a str>,
    pub operator: Principal,
    pub cycles_ledger: Principal,
    pub estimated_required_cycles: Option<u128>,
    pub desired: Option<&'a crate::fleet_ensure::dto::LoadedDesiredFleet>,
    pub conversion: Option<ReadinessConversionRequest>,
}

///
/// ReadinessConversionRequest
///
/// Exact public quote providers selected for advisory conversion only.
///

#[derive(Clone, Copy)]
pub struct ReadinessConversionRequest {
    pub icp_ledger: Principal,
    pub cmc: Principal,
}

/// Failure to establish even the read-only pre-build facts.
#[derive(Debug, Error)]
pub enum FleetReadinessError {
    #[error(transparent)]
    Policy(#[from] EnsurePolicyError),
    #[error(transparent)]
    State(#[from] EnsureStateError),
    #[error(transparent)]
    Network(#[from] NetworkIdentityError),
    #[error(transparent)]
    Transport(Box<OperatorMintTransportError>),
    #[error("selected signer or network does not match readiness authority")]
    AuthorityMismatch,
    #[error("readiness observation clock is unavailable")]
    Clock,
    #[error(
        "retained Fleet evidence is inconsistent; preserve it and use the existing recovery flow"
    )]
    RetainedEvidence,
}

impl From<OperatorMintTransportError> for FleetReadinessError {
    fn from(error: OperatorMintTransportError) -> Self {
        Self::Transport(Box::new(error))
    }
}

/// Observe current funding and local blockers without creating operator state.
pub fn inspect(request: &FleetReadinessRequest<'_>) -> Result<FleetReadiness, FleetReadinessError> {
    let started = now_ms()?;
    validate_path_labels(request.environment, request.fleet)?;
    let paths = EnsurePaths::under(request.workspace, request.environment, request.fleet);
    let retained_operation = retained(&paths, request)?;
    let expected_network =
        resolve_canonical_network_id_from_root(request.workspace, request.environment)?;
    if let Some(selected) = request.desired {
        let desired = &selected.desired;
        crate::fleet_ensure::policy::validate_path_identity(desired, request.fleet)?;
        let identity_matches = desired.environment == request.environment
            && desired.operator == request.operator.to_text()
            && desired.cycles_ledger == request.cycles_ledger.to_text();
        let network_matches = desired
            .bootstrap
            .as_ref()
            .is_none_or(|bootstrap| bootstrap.canonical_network_id == expected_network);
        if !identity_matches || !network_matches {
            return Err(FleetReadinessError::AuthorityMismatch);
        }
        crate::fleet_ensure::policy::validate_funding_policy(desired)?;
    }
    let icp = IcpCli::new(request.icp_executable, Some(request.environment.into()))
        .with_identity(request.signing_identity)
        .with_cwd(request.workspace);
    let transport = OperatorMintTransport::from_icp(&icp)?;
    verify_authority(
        request.operator,
        transport.operator()?,
        &expected_network,
        &transport.canonical_network_id()?,
    )?;
    let available = transport.operator_balance(request.cycles_ledger)?;
    let mut report = report(
        request,
        expected_network.to_string(),
        available,
        retained_operation,
    );
    report.observed_at_unix_ms = started;
    if let Some(selected) = request.desired {
        report.funding =
            crate::fleet_ensure::ops::readiness::roots(request.workspace, &selected.desired, &icp)?;
        report.funding.desired_sha256 = Some(selected.sha256.clone());
        if report
            .funding
            .roots
            .iter()
            .any(|root| root.floor_shortfall_cycles.is_some_and(|cycles| cycles > 0))
        {
            report.blockers.push(ReadinessBlocker::RootNativeShortfall);
        }
        if report
            .funding
            .roots
            .iter()
            .any(|root| root.unfunded_role.is_some())
        {
            report.blockers.push(ReadinessBlocker::StartupFundingPolicy);
        }
    }
    if let Some(conversion) = request.conversion {
        report.funding.unresolved.retain(|reason| *reason != crate::fleet_ensure::view::readiness::ReadinessUnresolved::ConversionNotRequested);
        let quote =
            transport.quote_blocking(conversion.icp_ledger, conversion.cmc, request.cycles_ledger);
        apply_quote(&mut report, conversion, quote.ok(), now_ms()?);
    }
    report.completed_at_unix_ms = now_ms()?;
    Ok(report)
}

fn retained(
    paths: &EnsurePaths,
    request: &FleetReadinessRequest<'_>,
) -> Result<Option<RetainedReadinessOperation>, FleetReadinessError> {
    let journal = match read_journal(paths) {
        Ok(Some(journal)) => journal,
        Ok(None) => return Ok(None),
        Err(error @ EnsureStateError::Decode { .. }) => {
            let source = crate::fleet_ensure::ops::reinstall::terminal::read(
                paths,
                request.environment,
                request.fleet,
            )
            .map_err(|_| FleetReadinessError::State(error))?;
            return Ok(Some(RetainedReadinessOperation {
                operation_id: source.documents.operation_id,
                plan_sha256: source.documents.plan_sha256,
                completion: FleetEnsureCompletion::Converged,
                terminal_review_required: true,
            }));
        }
        Err(error) => return Err(error.into()),
    };
    if journal.fleet != request.fleet {
        return Err(FleetReadinessError::RetainedEvidence);
    }
    if journal.completion != FleetEnsureCompletion::Converged {
        let plan = read_plan(paths)?.ok_or(FleetReadinessError::RetainedEvidence)?;
        if plan.plan_sha256 != journal.plan_sha256
            || plan.operation_id != journal.operation_id
            || plan.environment != request.environment
            || plan.fleet != request.fleet
        {
            return Err(FleetReadinessError::RetainedEvidence);
        }
    }
    Ok(Some(RetainedReadinessOperation {
        operation_id: journal.operation_id,
        plan_sha256: journal.plan_sha256,
        completion: journal.completion,
        terminal_review_required: false,
    }))
}

fn report(
    request: &FleetReadinessRequest<'_>,
    network_identity: String,
    available_cycles: u128,
    retained_operation: Option<RetainedReadinessOperation>,
) -> FleetReadiness {
    let estimated_shortfall_cycles = request
        .estimated_required_cycles
        .map(|amount| amount.saturating_sub(available_cycles));
    let mut blockers = Vec::new();
    if retained_operation
        .as_ref()
        .is_some_and(|operation| operation.completion != FleetEnsureCompletion::Converged)
    {
        blockers.push(ReadinessBlocker::RetainedOperation);
    }
    if retained_operation
        .as_ref()
        .is_some_and(|operation| operation.terminal_review_required)
    {
        blockers.push(ReadinessBlocker::RetainedTerminalReview);
    }
    if estimated_shortfall_cycles.is_some_and(|shortfall| shortfall > 0) {
        blockers.push(ReadinessBlocker::EstimatedFundingShortfall);
    }
    FleetReadiness {
        environment: request.environment.into(),
        fleet: request.fleet.into(),
        operator: request.operator.to_text(),
        cycles_ledger: request.cycles_ledger.to_text(),
        network_identity,
        available_cycles,
        estimated_required_cycles: request.estimated_required_cycles,
        estimated_shortfall_cycles,
        retained_operation,
        blockers,
        observed_at_unix_ms: 0,
        completed_at_unix_ms: 0,
        funding: crate::fleet_ensure::ops::readiness::unknown(),
    }
}

fn verify_authority(
    expected_operator: Principal,
    signer: Principal,
    expected_network: &CanonicalNetworkId,
    observed_network: &CanonicalNetworkId,
) -> Result<(), FleetReadinessError> {
    let valid_signer = expected_operator != Principal::anonymous() && signer == expected_operator;
    if valid_signer && expected_network == observed_network {
        Ok(())
    } else {
        Err(FleetReadinessError::AuthorityMismatch)
    }
}

fn now_ms() -> Result<u64, FleetReadinessError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|value| u64::try_from(value.as_millis()).ok())
        .ok_or(FleetReadinessError::Clock)
}

fn apply_quote(
    report: &mut FleetReadiness,
    selected: ReadinessConversionRequest,
    quote: Option<crate::fleet_ensure::view::operator_mint::OperatorMintRateQuote>,
    observed_at_ms: u64,
) {
    let Some(rate) = quote.filter(|rate| {
        rate.rate_timestamp_seconds <= observed_at_ms / 1000 && rate.xdr_permyriad_per_icp > 0
    }) else {
        report
            .funding
            .unresolved
            .push(ReadinessUnresolved::ConversionObservationFailed);
        return;
    };
    let amount = report.estimated_shortfall_cycles.and_then(|shortfall| {
        if shortfall == 0 {
            return Some(0);
        }
        crate::fleet_ensure::policy::operator_mint::quote::amount_e8s(
            shortfall,
            rate.estimated_deposit_fee_cycles,
            rate.xdr_permyriad_per_icp,
            rate.transfer_fee_e8s,
        )
    });
    if amount.is_none() {
        report
            .funding
            .unresolved
            .push(ReadinessUnresolved::ConversionAmountUnavailable);
    }
    let debit = amount.and_then(|amount| {
        if amount == 0 {
            Some(0)
        } else {
            amount.checked_add(rate.transfer_fee_e8s)
        }
    });
    report.funding.conversion = Some(ReadinessConversionQuote {
        cmc: selected.cmc.to_text(),
        icp_ledger: selected.icp_ledger.to_text(),
        rate,
        estimated_mint_e8s: amount,
        estimated_total_icp_debit_e8s: debit,
    });
}
