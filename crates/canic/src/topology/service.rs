//! Resolve outbound methods through the actor's trusted Root and local directory.
use crate::topology::{
    attestation::{RoleCallError, RoleCalls, RoleMethod},
    directory::retained::DIRECTORY,
};
use ic_platform_types::Principal;
use std::{cell::OnceCell, rc::Rc};

thread_local! {
    static CALLS: OnceCell<Rc<RoleCalls>> = const { OnceCell::new() };
}

/// Attach outbound role calls during install or reopen using the actor’s trusted routing Root.
/// Keep the returned handle when explicit cached calls need the same proof cache.
///
/// # Panics
/// Traps on a duplicate attachment or an invalid routing Root.
#[allow(
    clippy::must_use_candidate,
    reason = "Lifecycle callers can use the attached named service client without retaining another handle"
)]
pub fn attach_role_calls(root: Principal) -> Rc<RoleCalls> {
    assert_eq!(root.as_slice().last(), Some(&1), "invalid routing Root");
    let client = Rc::new(RoleCalls::new(root));
    CALLS.with(|calls| assert!(calls.set(Rc::clone(&client)).is_ok(), "already attached"));
    client
}

/// Submit one role-bound request to a named service in the published directory.
/// The application rechecks its user's permission after any proof renewal.
/// # Errors
/// Refuses missing topology, failed authorization, invalid proof or a failed application call.
pub async fn call_service<M: RoleMethod>(
    name: &'static str,
    seconds: u32,
    request: M::Input,
    authorize: impl Fn() -> Result<(), String>,
) -> Result<M::Output, RoleCallError> {
    let target = DIRECTORY
        .with_borrow(|index| index.as_ref()?.current()?.by_name(name))
        .ok_or_else(|| RoleCallError::Proof("service unpublished".into()))?;
    let calls = CALLS
        .with(|slot| slot.get().cloned())
        .ok_or_else(|| RoleCallError::Proof("missing installing Root".into()))?;
    calls.call::<M>(target, seconds, request, authorize).await
}
