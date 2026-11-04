#![expect(clippy::unused_async)]
//! Minimal non-root canister for delegation proof tests.

mod portable_issuance;
mod protocol;

use canic::{
    Error,
    access::auth::{
        LocalApplicationAuthorizationDecision, LocalApplicationAuthorizationDenial,
        LocalApplicationAuthorizationRequest, authorize_local_application,
    },
    api::auth::AuthApi,
    api::metrics::MetricsQuery,
    dto::{metrics::QueryPerfSample, role_attestation::SignedRoleAttestation},
    prelude::*,
};
use ic_platform::principal::ExternalPrincipal as Principal;
use serde::Deserialize;

const VERIFY_APPLICATION_SCOPE: canic::access::auth::ApplicationScopeRef<'static> =
    canic::application_scope!("app00:xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");

#[derive(Clone, Copy, Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
enum LocalAuthorizationDenialProbe {
    Anonymous,
    AuthorityUnavailable,
    CallerMismatch,
}

impl LocalAuthorizationDenialProbe {
    const fn denial(self) -> LocalApplicationAuthorizationDenial {
        match self {
            Self::Anonymous => LocalApplicationAuthorizationDenial::Anonymous,
            Self::AuthorityUnavailable => LocalApplicationAuthorizationDenial::AuthorityUnavailable,
            Self::CallerMismatch => LocalApplicationAuthorizationDenial::CallerMismatch,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Anonymous => "anonymous",
            Self::AuthorityUnavailable => "authority_unavailable",
            Self::CallerMismatch => "caller_mismatch",
        }
    }
}

canic::start!(lifecycle_participant(
    init = compose_application_certification,
    post_upgrade = compose_application_certification
),);

// A trusted application branch shares the actual IC certification root with /sig.
// Both install and same-release restoration reapply it before deferred work.
fn compose_application_certification() {
    AuthApi::set_canister_signature_siblings(vec![canic::api::auth::SignatureSibling::Left(
        [0x3c; 32],
    )])
    .expect("compose application certification with Canic signatures");
}

/// Run no-op setup for the delegation issuer stub.
async fn canic_setup() {}

/// Accept no install payload for the delegation issuer stub.
async fn canic_install(_args: Option<Vec<u8>>) {}

/// Run no-op upgrade handling for the delegation issuer stub.
async fn canic_upgrade() {}

/// Prepare a genuine canister-generated key through the selected issuer owner.
#[update(requires(caller::is_root()), on_access_denied = "reject", wire(id = protocol::VOLATILE_KEY, request = canic::protocol::portable_enrollment::registration::REQUEST_LIMITS, response = canic::protocol::portable_enrollment::registration::RESPONSE_LIMITS))]
async fn test_prepare_volatile_portable_issuer(
    request: canic::dto::portable_enrollment::PreparePortableIssuerKey,
) -> Result<
    canic::dto::portable_enrollment::PortableIssuerPreparedKey,
    canic::dto::portable_enrollment::PortableIssuerRegistrationRefusal,
> {
    canic::api::auth::portable_issuer::prepare(request).await
}

/// Register a genuine heap key without exposing a generic signer or its seed.
#[update(requires(caller::is_root()), on_access_denied = "reject", wire(id = protocol::VOLATILE_REGISTER, request = canic::protocol::portable_enrollment::registration::REQUEST_LIMITS, response = canic::protocol::portable_enrollment::registration::RESPONSE_LIMITS))]
async fn test_register_volatile_portable_issuer(
    challenge: canic::dto::portable_enrollment::PortableIssuerChallenge,
) -> Result<
    canic::dto::portable_enrollment::PortableIssuerEnrollment,
    canic::dto::portable_enrollment::PortableIssuerRegistrationRefusal,
> {
    canic::api::auth::portable_issuer::register(challenge).await
}

/// Read original public Root work; a missing heap key cannot be reconstructed here.
#[update(requires(caller::is_root()), on_access_denied = "reject", wire(id = protocol::VOLATILE_LOOKUP, request = canic::protocol::portable_enrollment::registration::REQUEST_LIMITS, response = canic::protocol::portable_enrollment::registration::RESPONSE_LIMITS))]
async fn test_lookup_volatile_portable_issuer(
    request: canic::dto::portable_enrollment::PortableIssuerRegistrationLookup,
) -> Result<
    canic::dto::portable_enrollment::PortableIssuerRegistrationStatus,
    canic::dto::portable_enrollment::PortableIssuerRegistrationRefusal,
> {
    canic::api::auth::portable_issuer::lookup(request).await
}

/// Authenticate the account independently of the browser and issue through the real owner.
#[update(public, on_access_denied = "reject", wire(id = protocol::ISSUE, request = canic::protocol::portable_issuance::REQUEST_LIMITS, response = canic::protocol::portable_issuance::RESPONSE_LIMITS))]
async fn test_issue_portable_credential(
    request: canic::dto::portable_issuance::IssuePortableCredential,
) -> Result<
    canic::dto::portable_issuance::IssuedPortableCredential,
    canic::dto::portable_issuance::PortableCredentialIssuanceRefusal,
> {
    portable_issuance::issue(request).await
}

/// Retrieve only the account's original credential; this update performs no issuance work.
#[update(public, on_access_denied = "reject", wire(id = protocol::RECONCILE, request = canic::protocol::portable_issuance::REQUEST_LIMITS, response = canic::protocol::portable_issuance::RESPONSE_LIMITS))]
fn test_reconcile_portable_credential(
    request: canic::dto::portable_issuance::ReconcilePortableCredential,
) -> Result<
    canic::dto::portable_issuance::PortableCredentialIssuanceStatus,
    canic::dto::portable_issuance::PortableCredentialIssuanceRefusal,
> {
    canic::api::auth::portable_issuer::reconcile(request)
}

/// Discover operation identity without reserving a login or writing the journal clock.
#[query(public, on_access_denied = "reject", wire(id = protocol::ISSUANCE_CONTEXT, request = canic::protocol::portable_issuance::REQUEST_LIMITS, response = canic::protocol::portable_issuance::RESPONSE_LIMITS))]
fn test_portable_issuance_context() -> Result<
    canic::dto::portable_issuance::PortableIssuanceContext,
    canic::dto::portable_issuance::PortableCredentialIssuanceRefusal,
> {
    canic::api::auth::portable_issuer::context()
}

/// Run bounded retention work only for the actual installed Root.
#[update(requires(caller::is_root()), on_access_denied = "reject", wire(id = protocol::ISSUANCE_RECLAIM, request = canic::protocol::portable_issuance::REQUEST_LIMITS, response = canic::protocol::portable_issuance::RESPONSE_LIMITS))]
fn test_reclaim_portable_issuance() -> Result<
    canic::dto::portable_issuance::PortableIssuanceMaintenance,
    canic::dto::portable_issuance::PortableCredentialIssuanceRefusal,
> {
    canic::api::auth::portable_issuer::reclaim()
}

/// Exercise the generic synchronous application guard through the native boundary.
#[query(public, wire(id = protocol::SUBJECT, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn issuer_application_subject() -> Result<ic_platform::Principal, Error> {
    application_subject().map(Into::into)
}

/// Exercise Fleet admission against the exact local managed projection.
#[query(requires(caller::is_fleet_admitted()), wire(id = protocol::ADMISSION, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn issuer_fleet_admission_probe() -> Result<(), Error> {
    Ok(())
}

/// Measure one implemented caller-gate denial while enrollment is unavailable.
#[query(public, wire(id = protocol::DENIAL, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
async fn issuer_application_denial_perf(
    probe: LocalAuthorizationDenialProbe,
) -> Result<QueryPerfSample<String>, Error> {
    let start = MetricsQuery::sample_query(()).local_instructions;
    let actual_caller: Principal = ic_platform::cdk::api::msg_caller().into();
    let observed_transport_caller = match probe {
        LocalAuthorizationDenialProbe::CallerMismatch => Principal::anonymous(),
        _ => actual_caller,
    };
    let decision = authorize_local_application(LocalApplicationAuthorizationRequest {
        observed_transport_caller,
        required_scope: VERIFY_APPLICATION_SCOPE,
    });
    assert_eq!(
        decision,
        LocalApplicationAuthorizationDecision::Deny(probe.denial())
    );
    let mut sample = MetricsQuery::sample_query(probe.label().to_string());
    sample.local_instructions = sample.local_instructions.saturating_sub(start);
    Ok(sample)
}

/// Measure the successful local guard under the same instruction ceiling as denials.
#[query(public, wire(id = protocol::ALLOW_PERF, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
fn issuer_application_allow_perf() -> Result<QueryPerfSample<ic_platform::Principal>, Error> {
    let start = MetricsQuery::sample_query(()).local_instructions;
    let subject = application_subject()?;
    let mut sample = MetricsQuery::sample_query(subject.into());
    sample.local_instructions = sample.local_instructions.saturating_sub(start);
    Ok(sample)
}

fn application_subject() -> Result<Principal, Error> {
    let caller =
        ic_platform::principal::ExternalPrincipal::from(ic_platform::cdk::api::msg_caller());
    match authorize_local_application(LocalApplicationAuthorizationRequest {
        observed_transport_caller: caller,
        required_scope: VERIFY_APPLICATION_SCOPE,
    }) {
        LocalApplicationAuthorizationDecision::Allow(authorized) => Ok(authorized.subject),
        LocalApplicationAuthorizationDecision::Deny(_) => Err(Error::from_registered(
            canic::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        )),
    }
}

#[update(public, wire(id = protocol::VERIFY, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
fn issuer_verify_role_attestation(request: protocol::VerifyRequest) -> Result<(), Error> {
    AuthApi::verify_role_attestation(&request.attestation.into(), request.min_accepted_epoch)
}

#[update(requires(auth::attested_local_subnet()), wire(id = protocol::ATTESTED, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
fn issuer_require_attested_local_subnet(attestation: SignedRoleAttestation) -> Result<(), Error> {
    let _ = attestation;
    Ok(())
}

#[update(requires(caller::is_root()), wire(id = protocol::ROOT_GUARD, request = protocol::REQUEST_LIMITS, response = protocol::RESPONSE_LIMITS))]
fn issuer_guard_is_root() -> Result<(), Error> {
    Ok(())
}

canic::finish!();
