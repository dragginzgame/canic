//! Reusable caller checks for Canic endpoints. Each check reads the receiving canister's authority.
#[cfg(feature = "role-verification")]
use crate::topology::attestation::{Proof, RoleMethod};
#[cfg(feature = "topology")]
use ic_platform_types::Principal;

/// Require the actual caller to control this canister.
/// # Errors
/// Refuses a caller outside the current IC controller set.
#[cfg(feature = "method")]
pub fn require_controller() -> Result<(), &'static str> {
    use ic_platform_runtime::system;
    system::is_controller(&system::caller())
        .then_some(())
        .ok_or("controller required")
}

/// Check the actual caller against a locally retained topology authority.
/// # Errors
/// Refuses an invalid authority or a call from another principal.
#[cfg(feature = "topology")]
pub fn require_sender(expected: Principal) -> Result<(), &'static str> {
    if expected.as_slice().last() == Some(&1) && expected == ic_platform_runtime::system::caller() {
        Ok(())
    } else {
        Err("unexpected topology sender")
    }
}

/// Require the actual IC caller to be the named canister in this actor's directory.
/// # Errors
/// Refuses an unopened directory, an unpublished name, or a different caller.
#[cfg(all(feature = "topology", feature = "storage"))]
pub fn require_canister(name: &str) -> Result<(), &'static str> {
    let principal = crate::topology::directory::retained::DIRECTORY
        .with_borrow(|slot| slot.as_ref()?.current()?.by_name(name))
        .ok_or("service unpublished")?;
    require_sender(principal)
}

/// Pre-decode guard for a parent-only endpoint.
/// # Errors
/// Refuses a missing binding or a caller other than this actor's parent.
#[cfg(all(feature = "topology", feature = "storage"))]
pub fn require_parent() -> Result<(), &'static str> {
    (crate::topology::parent() == Some(ic_platform_runtime::system::caller()))
        .then_some(())
        .ok_or("unauthorized hub")
}

/// Admit only the issuer canister bound by the receiver's protected installation.
///
/// # Errors
/// Refuses when the receiver is closed or the caller is not its issuer.
#[cfg(feature = "auth-enrollment")]
pub fn require_issuer() -> Result<(), &'static str> {
    let caller = ic_platform_runtime::system::caller();
    let issuer = crate::auth::session::configured_issuer().ok_or("receiver unavailable")?;
    if caller.as_slice().last() == Some(&1) && caller == issuer {
        Ok(())
    } else {
        Err("configured issuer required")
    }
}

/// Admission shared by query and update mounts with a role contract.
/// # Errors
/// Refuses missing trust or a proof that fails this method's role check.
#[cfg(feature = "role-verification")]
pub fn require_role<M: RoleMethod>(proof: &Proof) -> Result<(), &'static str> {
    admit_roles(M::ROLES, proof)
}

#[cfg(feature = "role-verification")]
#[doc(hidden)]
pub fn admit_roles(roles: &[&str], proof: &Proof) -> Result<(), &'static str> {
    crate::topology::attestation::verify_roles(roles, proof)
}
