//! Observe Coordinator Root membership without constructing executable Registry authority.
//!
//! Project only exact maintained identity, revision and Root-row fields. Controller
//! declarations remain unprojected; authenticated physical custody owns those facts.

#[cfg(test)]
mod tests;

use crate::{
    fleet_ensure::{
        CompletedCoordinatorMembershipView,
        ops::reinstall::terminal::inventory::membership::{
            RESPONSE_BYTES, bounded_query, contract,
        },
    },
    protocol_binding::ResolvedProtocolBinding,
};
use candid::{CandidType, Principal, types::FuncMode};
use canic_core::{
    dto::fleet_registry::FleetSubnetRootEntry,
    ids::{FleetBinding, SubnetId},
    protocol,
};
use ic_agent::Agent;
use serde::Deserialize;
use std::time::Duration;
use thiserror::Error;

/// The source Coordinator cannot prove the original complete Root membership.
#[derive(Debug, Error)]
pub enum CompletedCoordinatorError {
    #[error(
        "Coordinator {coordinator} does not expose the exact required read-only Registry fields"
    )]
    Contract { coordinator: Principal },
    #[error("Coordinator Registry query failed: {0}")]
    Query(#[source] Box<ic_agent::AgentError>),
    #[error("Coordinator Registry query exceeded its observation deadline")]
    Deadline,
    #[error("Coordinator Registry query encoding or bounded decoding failed: {0}")]
    Decode(#[source] candid::Error),
    #[error("Coordinator rejected Registry observation: {0:?}")]
    Rejected(Box<canic_core::dto::error::Error>),
    #[error(
        "Coordinator identity, revision or complete Root membership differs from completed source evidence"
    )]
    Membership,
}

#[derive(CandidType)]
enum Request {
    Registry,
}

#[derive(CandidType, Deserialize)]
enum Response {
    Registry(RegistryFields),
}

#[derive(CandidType, Deserialize)]
struct RegistryFields {
    authority: AuthorityFields,
    revision: u64,
    fleet_subnet_roots: Vec<FleetSubnetRootEntry>,
}

#[derive(CandidType, Deserialize)]
struct AuthorityFields {
    binding: BindingFields,
    epoch: u64,
}

#[derive(CandidType, Deserialize)]
struct BindingFields {
    fleet: FleetBinding,
    coordinator: Principal,
    coordinator_subnet: SubnetId,
}

pub(super) fn verify_contract(
    coordinator: Principal,
    binding: &ResolvedProtocolBinding,
) -> Result<(), CompletedCoordinatorError> {
    let invalid = || CompletedCoordinatorError::Contract { coordinator };
    let text = contract::read(binding).ok_or_else(invalid)?;
    matches(&text).ok_or_else(invalid)
}

fn matches(text: &str) -> Option<()> {
    let (mut env, actor) = candid_parser::utils::CandidSource::Text(text).load().ok()?;
    let method = env
        .get_method(actor.as_ref()?, protocol::CANIC_COORDINATOR_REGISTRY)
        .ok()?
        .clone();
    if method.modes != [FuncMode::Query] || method.args.len() != 1 || method.rets.len() != 1 {
        return None;
    }
    contract::equal::<Request>(&mut env, &method.args[0])?;
    let ok = contract::variant(&env, &method.rets[0], "Ok")?;
    let registry = contract::variant(&env, &ok, "Registry")?;
    let authority = contract::record(&env, &registry, "authority")?;
    let binding = contract::record(&env, &authority, "binding")?;
    let epoch = contract::record(&env, &authority, "epoch")?;
    let revision = contract::record(&env, &registry, "revision")?;
    let roots = contract::record(&env, &registry, "fleet_subnet_roots")?;
    let fleet = contract::record(&env, &binding, "fleet")?;
    let coordinator = contract::record(&env, &binding, "coordinator")?;
    let subnet = contract::record(&env, &binding, "coordinator_subnet")?;
    let error = contract::variant(&env, &method.rets[0], "Err")?;
    contract::equal::<u64>(&mut env, &epoch)?;
    contract::equal::<u64>(&mut env, &revision)?;
    contract::equal::<Vec<FleetSubnetRootEntry>>(&mut env, &roots)?;
    contract::equal::<FleetBinding>(&mut env, &fleet)?;
    contract::equal::<Principal>(&mut env, &coordinator)?;
    contract::equal::<SubnetId>(&mut env, &subnet)?;
    contract::equal::<canic_core::dto::error::Error>(&mut env, &error)
}

pub(super) async fn observe(
    agent: &Agent,
    expected: &CompletedCoordinatorMembershipView,
) -> Result<CompletedCoordinatorMembershipView, CompletedCoordinatorError> {
    let Response::Registry(registry) = bounded_query::query(
        agent,
        expected.coordinator,
        protocol::CANIC_COORDINATOR_REGISTRY,
        Request::Registry,
        bounded_query::QueryLimits {
            timeout: Duration::from_secs(10),
            response_bytes: RESPONSE_BYTES,
        },
    )
    .await
    .map_err(|error| match error {
        bounded_query::QueryError::Decode(source) => CompletedCoordinatorError::Decode(source),
        bounded_query::QueryError::Expired => CompletedCoordinatorError::Deadline,
        bounded_query::QueryError::Query(source) => CompletedCoordinatorError::Query(source),
        bounded_query::QueryError::Rejected(reason) => CompletedCoordinatorError::Rejected(reason),
    })?;
    project(registry, expected)
}

fn project(
    registry: RegistryFields,
    expected: &CompletedCoordinatorMembershipView,
) -> Result<CompletedCoordinatorMembershipView, CompletedCoordinatorError> {
    let observed = CompletedCoordinatorMembershipView {
        fleet: registry.authority.binding.fleet,
        coordinator: registry.authority.binding.coordinator,
        coordinator_subnet: registry.authority.binding.coordinator_subnet,
        epoch: registry.authority.epoch,
        revision: registry.revision,
        roots: registry.fleet_subnet_roots,
    };
    if observed != *expected {
        return Err(CompletedCoordinatorError::Membership);
    }
    Ok(observed)
}
