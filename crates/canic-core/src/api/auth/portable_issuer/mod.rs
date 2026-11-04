//! Root-authenticated issuer preparation API; no credential-issuance endpoint is mounted.

use crate::{
    ops::{auth::portable_issuer as ops, ic::IcOps},
    workflow::runtime::auth::portable_issuer as workflow,
};
use canic_contracts::dto::portable_enrollment::{
    PortableIssuerChallenge, PortableIssuerEnrollment, PortableIssuerPreparedKey,
    PortableIssuerRegistrationLookup, PortableIssuerRegistrationRefusal as Refusal,
    PortableIssuerRegistrationStatus, PreparePortableIssuerKey,
};
use canic_contracts::dto::portable_issuance::{
    IssuePortableCredential, IssuedPortableCredential, PortableCredentialIssuanceRefusal,
    PortableCredentialIssuanceStatus, PortableIssuanceContext, PortableIssuanceMaintenance,
    ReconcilePortableCredential,
};

pub use crate::model::auth::portable_issuer::PortableIssuancePolicy;

fn require_root() -> Result<(), crate::ops::auth::enrollment::PortableEnrollmentError> {
    let domain = ops::current_domain()?;
    if ic_platform::Principal::from(IcOps::msg_caller()) != domain.authority {
        return Err(crate::ops::auth::enrollment::PortableEnrollmentError::Binding);
    }
    Ok(())
}

/// Prepare one trusted-local heap key after authenticating the installed Root.
/// Returned public bytes do not establish custody assurance or Root approval.
pub async fn prepare(
    request: PreparePortableIssuerKey,
) -> Result<PortableIssuerPreparedKey, Refusal> {
    require_root()?;
    workflow::prepare(request).await
}

/// Prove possession to the installed Root using only its exact domain-bound challenge.
pub async fn register(
    challenge: PortableIssuerChallenge,
) -> Result<PortableIssuerEnrollment, Refusal> {
    require_root()?;
    workflow::register(challenge).await
}

/// Return the original Root registration state without generating new work.
pub async fn lookup(
    request: PortableIssuerRegistrationLookup,
) -> Result<PortableIssuerRegistrationStatus, Refusal> {
    require_root()?;
    workflow::lookup(request).await
}

/// Issue for the actual IC-authenticated account and the separately bound browser key.
///
/// The application callback must authorize every requested role/scope for this account.
/// It runs before intent and again after awaits; Root enrollment is only the issuer's
/// ceiling. This controlled-local path refuses production while its clock is unqualified.
pub async fn issue(
    request: IssuePortableCredential,
    authorize: impl Fn(
        ic_platform::Principal,
        &IssuePortableCredential,
    ) -> Result<PortableIssuancePolicy, PortableCredentialIssuanceRefusal>,
) -> Result<IssuedPortableCredential, PortableCredentialIssuanceRefusal> {
    ops::issuance::require_local()?;
    let caller = IcOps::msg_caller().into();
    if caller == ic_platform::Principal::anonymous() {
        return Err(PortableCredentialIssuanceRefusal::Denied);
    }
    let policy = authorize(caller, &request)?;
    workflow::issue(caller, request, policy, authorize).await
}

/// Lookup only the authenticated account's original credential.
///
/// This also works after expiry or key loss. Missing evidence is unresolved;
/// lookup never calls the authorization hook,
/// creates a nonce, signs, refreshes Root or writes a clock observation.
pub fn reconcile(
    request: ReconcilePortableCredential,
) -> Result<PortableCredentialIssuanceStatus, PortableCredentialIssuanceRefusal> {
    ops::issuance::reconcile(
        &ops::current_domain()?,
        IcOps::msg_caller().into(),
        request.epoch,
        request.operation_id,
    )
}

/// Read the current epoch without reserving or renewing any account operation.
pub fn context() -> Result<PortableIssuanceContext, PortableCredentialIssuanceRefusal> {
    ops::issuance::context(&ops::protected_domain()?)
}

/// Let the installed Root reclaim expired history under the durable trusted clock.
/// Missing/pruned results remain unresolved; old epochs never admit new work.
pub fn reclaim() -> Result<PortableIssuanceMaintenance, PortableCredentialIssuanceRefusal> {
    require_root()?;
    ops::issuance::require_local()?;
    ops::issuance::reclaim(&ops::current_domain()?, IcOps::now_nanos())
}
