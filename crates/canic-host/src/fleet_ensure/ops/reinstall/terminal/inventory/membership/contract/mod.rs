//! Exact source-side Pool query contract verification before any remote read.
//!
//! Only the maintained request, response and error shapes are admitted. Other
//! source methods and historical generated authority are not decoded or executed.

pub(super) use crate::canister_protocol::contract::{equal, record, variant};
use crate::{
    durable_io::read_regular_bytes,
    fleet_ensure::ops::reinstall::terminal::inventory::membership::CompletedMembershipError,
    protocol_binding::ResolvedProtocolBinding,
};
use candid::{Principal, types::FuncMode};
use canic_core::{
    cdk::utils::hash::sha256_bytes,
    dto::{
        error::Error,
        pool::{CanisterPoolResponse, CanisterPoolStatusRequest},
    },
    protocol,
};

pub(super) fn verify(
    root: Principal,
    binding: &ResolvedProtocolBinding,
) -> Result<(), CompletedMembershipError> {
    let invalid = || CompletedMembershipError::Contract { root };
    let text = read(binding).ok_or_else(invalid)?;
    matches(&text).ok_or_else(invalid)
}

pub(super) fn read(binding: &ResolvedProtocolBinding) -> Option<String> {
    let bytes = read_regular_bytes(binding.candid_path(), 1024 * 1024).ok()?;
    if sha256_bytes(&bytes).as_slice() != binding.binding().candid_sha256 {
        return None;
    }
    String::from_utf8(bytes).ok()
}

pub(super) fn matches(text: &str) -> Option<()> {
    let (mut env, actor) = candid_parser::utils::CandidSource::Text(text).load().ok()?;
    let method = env
        .get_method(actor.as_ref()?, protocol::CANIC_ROOT_STATUS)
        .ok()?
        .clone();
    if method.modes != [FuncMode::Query] || method.args.len() != 1 || method.rets.len() != 1 {
        return None;
    }
    let request = variant(&env, &method.args[0], "Pool")?;
    let response = variant(&env, &method.rets[0], "Ok")?;
    let response = variant(&env, &response, "Pool")?;
    let error = variant(&env, &method.rets[0], "Err")?;
    equal::<CanisterPoolStatusRequest>(&mut env, &request)?;
    equal::<CanisterPoolResponse>(&mut env, &response)?;
    equal::<Error>(&mut env, &error)
}
