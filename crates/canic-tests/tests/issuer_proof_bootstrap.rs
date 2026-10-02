//! Fresh issuer proof fetching and idempotent issuer setup through canonical wire commands.

use candid::{CandidType, Deserialize, Principal};
use canic::{
    Error,
    diagnostics::codes,
    dto::auth::{
        ActiveDelegationProofStatus, ActiveDelegationProofStatusResponse, AuthRequestMetadata,
        DelegatedRoleGrant, DelegatedToken, DelegatedTokenGetRequest, DelegatedTokenPrepareRequest,
        DelegatedTokenPrepareResponse, DelegationAudience, RootIssuerConfigureRequest,
        RootIssuerConfigureResponse, RootIssuerRenewalBatchStatus, RootIssuerRenewalStatusRequest,
        RootIssuerRenewalStatusResponse,
    },
    ids::cap,
    protocol,
};
use canic_testing_internal::pic::{
    ActiveComponentRegistryFixture, managed_test_init_identity, role_grant,
    setup_fresh_active_component_registry,
};
use ic_testkit::pic::CandidCallExt;
use std::time::Duration;

#[derive(CandidType)]
enum RootCommand {
    ConfigureIssuer(RootIssuerConfigureRequest),
}
#[derive(CandidType, Deserialize)]
enum RootCommandResponse {
    ConfigureIssuer(RootIssuerConfigureResponse),
}
#[derive(CandidType)]
enum RootStatusRequest {
    IssuerRenewal(RootIssuerRenewalStatusRequest),
}
#[derive(CandidType, Deserialize)]
enum RootStatusResponse {
    IssuerRenewal(RootIssuerRenewalStatusResponse),
}
#[derive(CandidType)]
enum IssuerCommand {
    PrepareDelegatedToken(DelegatedTokenPrepareRequest),
}
#[derive(CandidType, Deserialize)]
enum IssuerCommandResponse {
    PrepareDelegatedToken(DelegatedTokenPrepareResponse),
}
#[derive(CandidType)]
enum IssuerStatusRequest {
    ActiveDelegationProof,
    DelegatedToken(DelegatedTokenGetRequest),
}
#[derive(CandidType, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "mirrors the canonical issuer status wire variants"
)]
enum IssuerStatusResponse {
    ActiveDelegationProof(ActiveDelegationProofStatusResponse),
    DelegatedToken(DelegatedToken),
}

#[test]
fn fresh_issuer_fetches_proof_and_configuration_retries_preserve_it() {
    let fixture = setup_fresh_active_component_registry();
    assert_eq!(
        active_proof_status(&fixture).status,
        ActiveDelegationProofStatus::Missing
    );
    let missing = prepare(&fixture, 50).unwrap_err();
    assert_eq!(missing.code(), codes::CONFIGURATION_INCOMPLETE.raw_code());
    assert!(renewal_status(&fixture).template.is_none());
    assert!(renewal_status(&fixture).latest_batch.is_none());

    configure_issuer(&fixture);
    assert_eq!(
        active_proof_status(&fixture).status,
        ActiveDelegationProofStatus::Missing
    );
    let token = prepare_and_get(&fixture, 51);
    assert_eq!(token.claims.subject, subject());
    assert_eq!(token.claims.issuer_pid, fixture.issuer.canister_id);
    let installed = active_proof_status(&fixture);
    assert_eq!(installed.status, ActiveDelegationProofStatus::Valid);
    assert_eq!(installed.cert_hash, Some(token.claims.cert_hash));

    configure_issuer(&fixture);
    assert_eq!(active_proof_status(&fixture), installed);
    let replay = prepare_and_get(&fixture, 51);
    assert_eq!(replay.claims, token.claims);
    let next = prepare_and_get(&fixture, 52);
    assert_eq!(next.proof, token.proof);
    fixture
        .pic()
        .update_candid_as_or_panic::<Result<(), Error>, _>(
            fixture.verifier.canister_id,
            subject(),
            "issuer_verify_token",
            (next,),
        )
        .expect("verifier must accept the automatically fetched proof");
    drop(fixture);
}

fn subject() -> Principal {
    Principal::self_authenticating([51; 32])
}

fn audience() -> DelegationAudience {
    DelegationAudience::Fleet(managed_test_init_identity().fleet.fleet)
}

fn grants(fixture: &ActiveComponentRegistryFixture) -> Vec<DelegatedRoleGrant> {
    vec![role_grant(
        fixture.verifier.role.clone(),
        vec![cap::SESSION.to_string(), cap::VERIFY.to_string()],
    )]
}

fn prepare(
    fixture: &ActiveComponentRegistryFixture,
    request_id: u8,
) -> Result<DelegatedTokenPrepareResponse, Error> {
    let response: Result<IssuerCommandResponse, Error> = fixture.pic().update_candid_as_or_panic(
        fixture.issuer.canister_id,
        subject(),
        protocol::CANIC_COMMAND,
        (IssuerCommand::PrepareDelegatedToken(
            DelegatedTokenPrepareRequest {
                metadata: Some(AuthRequestMetadata {
                    request_id: [request_id; 32],
                    ttl_ns: 60_000_000_000,
                }),
                aud: audience(),
                grants: grants(fixture),
                ttl_ns: 10_000_000_000,
                ext: None,
            },
        ),),
    );
    let IssuerCommandResponse::PrepareDelegatedToken(response) = response?;
    Ok(response)
}

fn prepare_and_get(fixture: &ActiveComponentRegistryFixture, request_id: u8) -> DelegatedToken {
    for _ in 0..5 {
        match prepare(fixture, request_id) {
            Ok(prepared) => {
                let response: Result<IssuerStatusResponse, Error> =
                    fixture.pic().query_candid_as_or_panic(
                        fixture.issuer.canister_id,
                        subject(),
                        protocol::CANIC_AUTH_STATUS,
                        (IssuerStatusRequest::DelegatedToken(
                            DelegatedTokenGetRequest {
                                claims_hash: prepared.claims_hash,
                            },
                        ),),
                    );
                let IssuerStatusResponse::DelegatedToken(token) = response.unwrap() else {
                    panic!("unexpected issuer token response");
                };
                return token;
            }
            Err(error) if error.code() == codes::SECURITY_UNAVAILABLE.raw_code() => {
                let batch = renewal_status(fixture)
                    .latest_batch
                    .expect("pending fetch must have a Root batch");
                assert!(
                    matches!(
                        batch.status,
                        RootIssuerRenewalBatchStatus::Prepared
                            | RootIssuerRenewalBatchStatus::Signing
                            | RootIssuerRenewalBatchStatus::Signed
                            | RootIssuerRenewalBatchStatus::Installing
                            | RootIssuerRenewalBatchStatus::Installed
                    ),
                    "unexpected batch state: {batch:?}"
                );
                fixture.pic().advance_time(Duration::from_secs(2));
                for _ in 0..6 {
                    fixture.pic().tick();
                }
            }
            Err(error) => panic!("token preparation failed: {error:?}"),
        }
    }
    panic!("Root signing did not complete within the bounded preparation retries");
}

fn active_proof_status(
    fixture: &ActiveComponentRegistryFixture,
) -> ActiveDelegationProofStatusResponse {
    let response: Result<IssuerStatusResponse, Error> = fixture.pic().query_candid_as_or_panic(
        fixture.issuer.canister_id,
        fixture.root,
        protocol::CANIC_AUTH_STATUS,
        (IssuerStatusRequest::ActiveDelegationProof,),
    );
    let IssuerStatusResponse::ActiveDelegationProof(status) = response.unwrap() else {
        panic!("unexpected issuer proof response");
    };
    status
}

fn renewal_status(fixture: &ActiveComponentRegistryFixture) -> RootIssuerRenewalStatusResponse {
    let response: Result<RootStatusResponse, Error> = fixture.pic().query_candid_or_panic(
        fixture.root,
        protocol::CANIC_ROOT_STATUS,
        (RootStatusRequest::IssuerRenewal(
            RootIssuerRenewalStatusRequest {
                issuer_pid: fixture.issuer.canister_id,
            },
        ),),
    );
    let RootStatusResponse::IssuerRenewal(status) = response.unwrap();
    status
}

fn configure_issuer(fixture: &ActiveComponentRegistryFixture) {
    let response: Result<RootCommandResponse, Error> = fixture.pic().update_candid_or_panic(
        fixture.root,
        protocol::CANIC_ROOT_COMMAND,
        (RootCommand::ConfigureIssuer(RootIssuerConfigureRequest {
            issuer_pid: fixture.issuer.canister_id,
            enabled: true,
            aud: audience(),
            grants: grants(fixture),
            cert_ttl_ns: 60_000_000_000,
            refresh_after_ratio_bps: 8_000,
        }),),
    );
    let RootCommandResponse::ConfigureIssuer(configuration) = response.unwrap();
    assert_eq!(configuration.issuer.issuer_pid, fixture.issuer.canister_id);
    assert_eq!(
        configuration.template.issuer_pid,
        fixture.issuer.canister_id
    );
}
