//! Minimal non-root canister for delegation proof tests.

#![expect(clippy::unused_async)]

use candid::{CandidType, Deserialize, Principal};
use canic::{
    Error,
    access::auth::{
        LocalApplicationAuthorizationDecision, LocalApplicationAuthorizationDenial,
        LocalApplicationAuthorizationRequest, authorize_local_application,
    },
    api::auth::AuthApi,
    api::call::Call,
    api::metrics::MetricsQuery,
    dto::{
        auth::{DelegatedToken, SignedRoleAttestation},
        component_registry::RootPeerComponentAllocationRequest,
        metrics::{MetricEntry, QueryPerfSample},
        page::{Page, PageRequest},
        role::OperationReceipt,
    },
    ids::cap,
    prelude::*,
    protocol::CANIC_ROOT_COMMAND,
};

const VERIFY_APPLICATION_SCOPE: canic::access::auth::ApplicationScopeRef<'static> =
    canic::application_scope!("app00:xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");

#[derive(CandidType)]
enum RootCommand {
    ProvisionPeer(RootPeerComponentAllocationRequest),
}

#[derive(CandidType, Deserialize)]
enum RootCommandResponse {
    OperationAccepted(OperationReceipt),
}

#[derive(CandidType, Clone, Copy, Deserialize)]
enum LocalAuthorizationDenialProbe {
    Anonymous,
    AuthorityUnavailable,
    CallerMismatch,
    Disabled,
    Expired,
    InadmissibleSubject,
    MissingScope,
    MissingSession,
    StaleAuthority,
}

impl LocalAuthorizationDenialProbe {
    const fn denial(self) -> LocalApplicationAuthorizationDenial {
        match self {
            Self::Anonymous => LocalApplicationAuthorizationDenial::Anonymous,
            Self::AuthorityUnavailable => LocalApplicationAuthorizationDenial::AuthorityUnavailable,
            Self::CallerMismatch => LocalApplicationAuthorizationDenial::CallerMismatch,
            Self::Disabled => LocalApplicationAuthorizationDenial::Disabled,
            Self::Expired => LocalApplicationAuthorizationDenial::Expired,
            Self::InadmissibleSubject => LocalApplicationAuthorizationDenial::InadmissibleSubject,
            Self::MissingScope => LocalApplicationAuthorizationDenial::MissingScope,
            Self::MissingSession => LocalApplicationAuthorizationDenial::MissingSession,
            Self::StaleAuthority => LocalApplicationAuthorizationDenial::StaleAuthority,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Anonymous => "anonymous",
            Self::AuthorityUnavailable => "authority_unavailable",
            Self::CallerMismatch => "caller_mismatch",
            Self::Disabled => "disabled",
            Self::Expired => "expired",
            Self::InadmissibleSubject => "inadmissible_subject",
            Self::MissingScope => "missing_scope",
            Self::MissingSession => "missing_session",
            Self::StaleAuthority => "stale_authority",
        }
    }
}

canic::start!();

/// Run no-op setup for the delegation issuer stub.
async fn canic_setup() {}

/// Accept no install payload for the delegation issuer stub.
async fn canic_install(_args: Option<Vec<u8>>) {}

/// Run no-op upgrade handling for the delegation issuer stub.
async fn canic_upgrade() {}

#[canic_update(requires(auth::authenticated(cap::VERIFY)))]
async fn issuer_verify_token(token: DelegatedToken) -> Result<(), Error> {
    let _ = token;
    Ok(())
}

#[canic_update(requires(auth::authenticated()))]
async fn issuer_verify_token_any(token: DelegatedToken) -> Result<(), Error> {
    let _ = token;
    Ok(())
}

/// Exercise the generic synchronous application guard without changing its application ABI.
#[canic_query(public)]
async fn issuer_application_subject() -> Result<Principal, Error> {
    application_subject()
}

/// Exercise Fleet admission against the exact local managed projection.
#[canic_query(requires(caller::is_fleet_admitted()))]
async fn issuer_fleet_admission_probe() -> Result<(), Error> {
    Ok(())
}

/// Measure the same synchronous local-application decision in its query context.
#[canic_query(public)]
async fn issuer_application_subject_perf()
-> Result<QueryPerfSample<Result<Principal, Error>>, Error> {
    let start = MetricsQuery::sample_query(()).local_instructions;
    let mut sample = MetricsQuery::sample_query(application_subject());
    sample.local_instructions = sample.local_instructions.saturating_sub(start);
    Ok(sample)
}

/// Return runtime metrics to the focused instruction-measurement fixture.
#[canic_query(public)]
async fn issuer_runtime_metrics() -> Result<Page<MetricEntry>, Error> {
    Ok(MetricsQuery::runtime(PageRequest {
        offset: 0,
        limit: 1_000,
    }))
}

/// Measure one branch of the closed local-authorization denial partition.
#[canic_query(public)]
async fn issuer_application_denial_perf(
    probe: LocalAuthorizationDenialProbe,
) -> Result<QueryPerfSample<String>, Error> {
    let start = MetricsQuery::sample_query(()).local_instructions;
    let decision =
        canic::__internal::core::access::auth::measure_local_application_authorization_denial(
            probe.denial(),
        );
    assert!(matches!(
        decision,
        LocalApplicationAuthorizationDecision::Deny(_)
    ));
    let mut sample = MetricsQuery::sample_query(probe.label().to_string());
    sample.local_instructions = sample.local_instructions.saturating_sub(start);
    Ok(sample)
}

fn application_subject() -> Result<Principal, Error> {
    let caller = ic_cdk::api::msg_caller();
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

#[canic_update(public)]
async fn issuer_verify_role_attestation(
    attestation: SignedRoleAttestation,
    min_accepted_epoch: u64,
) -> Result<(), Error> {
    AuthApi::verify_role_attestation(&attestation, min_accepted_epoch).await
}

#[canic_update(requires(auth::attested_local_subnet()))]
async fn issuer_require_attested_local_subnet(
    attestation: SignedRoleAttestation,
) -> Result<(), Error> {
    let _ = attestation;
    Ok(())
}

#[canic_update(requires(caller::is_root()))]
async fn issuer_guard_is_root() -> Result<(), Error> {
    Ok(())
}

#[canic_update(requires(caller::is_controller()))]
async fn issuer_guard_is_controller() -> Result<(), Error> {
    Ok(())
}

#[canic_update(requires(caller::is_parent()))]
async fn issuer_guard_is_parent() -> Result<(), Error> {
    Ok(())
}

/// Forward one peer allocation request so the target root observes this ordinary Component.
#[canic_update(public)]
async fn forward_peer_allocation(
    fleet_subnet_root: Principal,
    request: RootPeerComponentAllocationRequest,
) -> Result<OperationReceipt, Error> {
    let response: Result<RootCommandResponse, Error> =
        Call::bounded_wait(fleet_subnet_root, CANIC_ROOT_COMMAND)
            .with_arg(RootCommand::ProvisionPeer(request))?
            .execute_candid()
            .await?;
    match response? {
        RootCommandResponse::OperationAccepted(receipt) => Ok(receipt),
    }
}

canic::finish!();
