//! Delegation operation helpers layered atop `state::delegation`.
//!
//! The ops layer adds policy, logging, and cleanup around session caches and
//! registries while re-exporting the state-level helpers.

mod cache;
mod registry;

pub use cache::*;
pub use registry::*;

use crate::{
    Error,
    cdk::call::Call,
    memory::topology::SubnetDirectory,
    state::delegation::{DelegationSessionView, RegisterSessionArgs},
    types::{CanisterType, Principal},
    utils::time::now_secs,
};

///
/// Synchronize a session from the source (for instance, an auth canister)
/// into the local registry.
///
/// This function is typically called by the delegation session itself to
/// retrieve and register its wallet principal and expiration state.
///
/// # Returns
/// - `Ok(())` if the session was successfully synchronized.
/// - `Err(Error)` if the call or registration failed.
///
pub async fn sync_session_with_source(
    session_pid: Principal,
    ty: CanisterType,
) -> Result<(), Error> {
    let auth_canister_pid = SubnetDirectory::try_get(&ty)?.pid;

    let session: DelegationSessionView =
        Call::unbounded_wait(auth_canister_pid, "canic_delegation_track")
            .with_args(&(session_pid,))
            .await?
            .candid::<Result<DelegationSessionView, Error>>()
            .map_err(|e| Error::custom(e.to_string()))??;

    DelegationRegistry::register_session(
        session.wallet_pid,
        RegisterSessionArgs {
            session_pid: session.session_pid,
            duration_secs: session.expires_at.saturating_sub(now_secs()),
        },
    )?;

    Ok(())
}
