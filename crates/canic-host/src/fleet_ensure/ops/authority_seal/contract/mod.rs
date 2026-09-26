//! Admit only the maintained authority-fence command and observation wire types.
//!
//! A retained interface supplies evidence, never permission to adapt an old command.

use crate::{
    canister_protocol::contract::{equal, variant},
    fleet_ensure::model::DesiredCanisterKind,
};
use candid::{TypeEnv, types::FuncMode};
use canic_core::{
    dto::{
        authority_restore::{AuthorityRestoreFenceStatusResponse, AuthoritySnapshotRequest},
        error::Error,
    },
    protocol,
};

pub(in crate::fleet_ensure) const fn methods(
    kind: DesiredCanisterKind,
) -> Option<(&'static str, &'static str)> {
    match kind {
        DesiredCanisterKind::Root => {
            Some((protocol::CANIC_ROOT_COMMAND, protocol::CANIC_ROOT_STATUS))
        }
        DesiredCanisterKind::Coordinator => Some((
            protocol::CANIC_COORDINATOR_COMMAND,
            protocol::CANIC_OBSERVABILITY,
        )),
        _ => None,
    }
}

pub(in crate::fleet_ensure) fn verify(text: &str, kind: DesiredCanisterKind) -> Option<()> {
    let (command, status) = methods(kind)?;
    let (mut env, actor) = candid_parser::utils::CandidSource::Text(text).load().ok()?;
    let actor = actor?;
    let command = env.get_method(&actor, command).ok()?.clone();
    let status = env.get_method(&actor, status).ok()?.clone();
    if !command.modes.is_empty() || status.modes != [FuncMode::Query] {
        return None;
    }
    check::<AuthoritySnapshotRequest>(&mut env, &command, "PrepareAuthoritySnapshot")?;
    check::<()>(&mut env, &status, "AuthorityRestore")
}

fn check<T: candid::CandidType>(
    env: &mut TypeEnv,
    function: &candid::types::Function,
    selector: &str,
) -> Option<()> {
    let [request] = function.args.as_slice() else {
        return None;
    };
    let [response] = function.rets.as_slice() else {
        return None;
    };
    let request = variant(env, request, selector)?;
    let ok = variant(env, response, "Ok")?;
    let ok = variant(env, &ok, selector)?;
    let error = variant(env, response, "Err")?;
    equal::<T>(env, &request)?;
    equal::<AuthorityRestoreFenceStatusResponse>(env, &ok)?;
    equal::<Error>(env, &error)
}
