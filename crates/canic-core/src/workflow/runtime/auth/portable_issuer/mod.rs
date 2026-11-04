//! Issuer key preparation and Root registration orchestration.
//!
//! Endpoints authenticate Root; ops owns key material, records and each IC call.
//! This preparation service does not activate production credential issuance.

use crate::model::auth::portable_issuer::{PortableIssuancePolicy, PortableIssuancePreparation};
use crate::{
    model::auth::portable_issuer::PortableIssuerKeyPreparation,
    ops::auth::{enrollment::fresh_challenge, portable_issuer as ops},
};
use canic_contracts::dto::portable_enrollment::{
    PortableIssuerChallenge, PortableIssuerEnrollment, PortableIssuerPreparedKey,
    PortableIssuerRegistrationLookup, PortableIssuerRegistrationRefusal as Refusal,
    PortableIssuerRegistrationStatus, PreparePortableIssuerKey,
};
use canic_contracts::dto::portable_issuance::{
    IssuePortableCredential, IssuedPortableCredential,
    PortableCredentialIssuanceRefusal as IssueRefusal,
};

/// Retain intent before randomness; callback completion rechecks its installed domain.
pub async fn prepare(
    request: PreparePortableIssuerKey,
) -> Result<PortableIssuerPreparedKey, Refusal> {
    let domain = ops::current_domain()?;
    match ops::begin(domain.clone(), request)? {
        PortableIssuerKeyPreparation::Ready(result) => Ok(result),
        PortableIssuerKeyPreparation::Pending(ticket) => {
            let randomness = fresh_challenge().await?;
            if ops::current_domain()? != domain {
                return Err(Refusal::Binding);
            }
            ops::finish(ticket, randomness).map_err(Into::into)
        }
    }
}

/// Sign one exact retained challenge and complete registration as the actual issuer.
pub async fn register(
    challenge: PortableIssuerChallenge,
) -> Result<PortableIssuerEnrollment, Refusal> {
    let domain = ops::current_domain()?;
    let (ticket, request) = ops::registration(
        domain.clone(),
        challenge,
        crate::ops::ic::IcOps::now_nanos(),
    )?;
    let result = ops::complete_at_root(&domain, request).await??;
    if ops::current_domain()? != domain {
        return Err(Refusal::Binding);
    }
    ops::registered(&ticket, result).map_err(Into::into)
}

/// Lookup-only reconciliation neither signs nor writes local or Root intent.
pub async fn lookup(
    request: PortableIssuerRegistrationLookup,
) -> Result<PortableIssuerRegistrationStatus, Refusal> {
    let domain = ops::current_domain()?;
    let result = ops::lookup_at_root(&domain, request).await??;
    if ops::current_domain()? != domain {
        return Err(Refusal::Binding);
    }
    Ok(result)
}

/// Persist exact intent, fetch fresh Root authority and recheck application permission.
pub async fn issue(
    caller: ic_platform::Principal,
    request: IssuePortableCredential,
    policy: PortableIssuancePolicy,
    authorize: impl Fn(
        ic_platform::Principal,
        &IssuePortableCredential,
    ) -> Result<PortableIssuancePolicy, IssueRefusal>,
) -> Result<IssuedPortableCredential, IssueRefusal> {
    let domain = ops::current_domain()?;
    match ops::issuance::begin(
        domain.clone(),
        caller,
        request.clone(),
        policy,
        crate::ops::ic::IcOps::now_nanos(),
    )? {
        PortableIssuancePreparation::Ready(result) => Ok(result),
        PortableIssuancePreparation::Pending(pending) => {
            if ops::issuance::retained_challenge(&pending, crate::ops::ic::IcOps::now_nanos())?
                .is_none()
            {
                let randomness = fresh_challenge().await?;
                ops::issuance::require_local()?;
                if ops::current_domain()? != domain {
                    return Err(IssueRefusal::Binding);
                }
                ops::issuance::challenge(&pending, randomness, crate::ops::ic::IcOps::now_nanos())?;
            }
            let snapshot = ops::issuance::fetch_authority(&pending).await?;
            ops::issuance::require_local()?;
            if ops::current_domain()? != domain {
                return Err(IssueRefusal::Binding);
            }
            ops::issuance::accept_authority(
                &pending,
                snapshot,
                crate::ops::ic::IcOps::now_nanos(),
            )?;
            if authorize(caller, &request)? != policy {
                return Err(IssueRefusal::Conflict);
            }
            ops::issuance::finish(&pending, crate::ops::ic::IcOps::now_nanos())
        }
    }
}
