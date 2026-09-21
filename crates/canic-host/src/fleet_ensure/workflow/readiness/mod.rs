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
        view::readiness::{FleetReadiness, ReadinessBlocker, RetainedReadinessOperation},
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
    validate_path_labels(request.environment, request.fleet)?;
    let paths = EnsurePaths::under(request.workspace, request.environment, request.fleet);
    let retained_operation = retained(&paths, request)?;
    let expected_network =
        resolve_canonical_network_id_from_root(request.workspace, request.environment)?;
    let transport = OperatorMintTransport::from_icp(
        &IcpCli::new(request.icp_executable, Some(request.environment.into()))
            .with_identity(request.signing_identity)
            .with_cwd(request.workspace),
    )?;
    verify_authority(
        request.operator,
        transport.operator()?,
        &expected_network,
        &transport.canonical_network_id()?,
    )?;
    let available = transport.operator_balance(request.cycles_ledger)?;
    Ok(report(
        request,
        expected_network.to_string(),
        available,
        retained_operation,
    ))
}

fn retained(
    paths: &EnsurePaths,
    request: &FleetReadinessRequest<'_>,
) -> Result<Option<RetainedReadinessOperation>, FleetReadinessError> {
    let Some(journal) = read_journal(paths)? else {
        return Ok(None);
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
