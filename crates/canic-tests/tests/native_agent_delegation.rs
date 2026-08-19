use candid::{CandidType, Decode, Deserialize, Encode};
use canic::{
    Error,
    dto::auth::{
        AuthRequestMetadata, DelegatedToken, DelegatedTokenGetRequest,
        DelegatedTokenPrepareRequest, DelegatedTokenPrepareResponse, DelegationAudience,
        RootIssuerPolicyResponse, RootIssuerPolicyUpsertRequest, RootIssuerRenewalTemplateResponse,
        RootIssuerRenewalTemplateUpsertRequest,
    },
    ids::cap,
    protocol,
};
use canic_testing_internal::pic::{
    managed_test_init_identity, report_canister_diagnostics, role_grant,
    setup_fresh_active_component_registry,
};
use ic_agent::{Agent, Identity, identity::Secp256k1Identity};
use ic_testkit::pic::CandidCallExt;

// Deterministic test-only identity; it has no authority outside this fresh PocketIC runtime.
const TEST_IDENTITY_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIGEAgEAMBAGByqGSM49AgEGBSuBBAAKBG0wawIBAQQgCDLudkRxUeRDhnUp2pvL
xLDICLIoNCa1sQdMgz5Y14GhRANCAASA7zusnWjPN0y8nJlD4YAEOpTEYu+CcCdO
VwidXc26G4+/g7dUbMwbN4E3d3bpxHEP31M+2by6jY67MqFKKroR
-----END PRIVATE KEY-----";

#[derive(CandidType)]
enum RootCommand {
    UpsertIssuerPolicy(RootIssuerPolicyUpsertRequest),
    UpsertIssuerRenewalTemplate(RootIssuerRenewalTemplateUpsertRequest),
}

#[derive(CandidType, Deserialize)]
enum RootCommandResponse {
    UpsertIssuerPolicy(RootIssuerPolicyResponse),
    UpsertIssuerRenewalTemplate(RootIssuerRenewalTemplateResponse),
}

#[derive(CandidType)]
enum CanisterCommand {
    PrepareDelegatedToken(DelegatedTokenPrepareRequest),
}

#[derive(CandidType, Deserialize)]
enum CanisterCommandResponse {
    PrepareDelegatedToken(DelegatedTokenPrepareResponse),
}

#[derive(CandidType)]
enum CanisterStatusRequest {
    DelegatedToken(DelegatedTokenGetRequest),
}

#[derive(CandidType, Deserialize)]
enum CanisterStatusResponse {
    DelegatedToken(DelegatedToken),
}

#[test]
fn pem_backed_native_agent_prepares_retrieves_and_presents_delegated_token() {
    let mut fixture = setup_fresh_active_component_registry();
    let gateway_url = fixture.start_http_gateway();
    configure_issuer(&fixture);
    provision_delegation_proof(&fixture);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("native-agent runtime must build");
    runtime.block_on(run_native_agent_journey(&fixture, gateway_url));
    drop(runtime);
    drop(fixture);
}

async fn run_native_agent_journey(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
    gateway_url: String,
) {
    let identity = Secp256k1Identity::from_pem(TEST_IDENTITY_PEM.as_bytes())
        .expect("test PEM identity must parse");
    let subject = identity.sender().expect("test identity must have a sender");
    let agent = Agent::builder()
        .with_url(gateway_url)
        .with_identity(identity)
        .build()
        .expect("native agent must build");
    agent
        .fetch_root_key()
        .await
        .expect("PocketIC root key must be available");

    let audience = DelegationAudience::Fleet(managed_test_init_identity().fleet.fleet);
    let grants = vec![role_grant(
        fixture.verifier.role.clone(),
        vec![cap::VERIFY.to_string()],
    )];
    let prepare_request = DelegatedTokenPrepareRequest {
        metadata: Some(AuthRequestMetadata {
            request_id: [42; 32],
            ttl_ns: 60_000_000_000,
        }),
        aud: audience,
        grants,
        ttl_ns: 10_000_000_000,
        ext: None,
    };
    let prepared_bytes = agent
        .update(&fixture.issuer.canister_id, protocol::CANIC_COMMAND)
        .with_arg(
            Encode!(&CanisterCommand::PrepareDelegatedToken(prepare_request))
                .expect("encode delegated-token prepare command"),
        )
        .call_and_wait()
        .await
        .expect("native delegated-token prepare ingress must succeed");
    let prepared: Result<CanisterCommandResponse, Error> =
        Decode!(&prepared_bytes, Result<CanisterCommandResponse, Error>)
            .expect("decode delegated-token prepare response");
    let CanisterCommandResponse::PrepareDelegatedToken(prepared) =
        prepared.expect("delegated-token preparation must succeed");

    let token_bytes = agent
        .query(&fixture.issuer.canister_id, protocol::CANIC_STATUS)
        .with_arg(
            Encode!(&CanisterStatusRequest::DelegatedToken(
                DelegatedTokenGetRequest {
                    claims_hash: prepared.claims_hash,
                }
            ))
            .expect("encode delegated-token retrieval status"),
        )
        .call()
        .await
        .expect("native delegated-token retrieval ingress must succeed");
    let token: Result<CanisterStatusResponse, Error> =
        Decode!(&token_bytes, Result<CanisterStatusResponse, Error>)
            .expect("decode delegated-token retrieval response");
    let CanisterStatusResponse::DelegatedToken(token) =
        token.expect("delegated-token retrieval must succeed");
    assert_eq!(token.claims.presenter, subject);
    assert_eq!(token.claims.subject, subject);

    let presented_bytes = agent
        .update(&fixture.verifier.canister_id, "issuer_verify_token")
        .with_arg(Encode!(&token).expect("encode delegated-token presentation"))
        .call_and_wait()
        .await
        .expect("native delegated-token presentation ingress must succeed");
    let presented: Result<(), Error> = Decode!(&presented_bytes, Result<(), Error>)
        .expect("decode delegated-token presentation response");
    presented.expect("the same authenticated native agent must pass verification");
}

fn configure_issuer(fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture) {
    let audience = DelegationAudience::Fleet(managed_test_init_identity().fleet.fleet);
    let grants = vec![role_grant(
        fixture.verifier.role.clone(),
        vec![cap::VERIFY.to_string()],
    )];
    let policy: Result<RootCommandResponse, Error> = fixture.pic().update_candid_or_panic(
        fixture.root,
        protocol::CANIC_COMMAND,
        (RootCommand::UpsertIssuerPolicy(
            RootIssuerPolicyUpsertRequest {
                issuer_pid: fixture.issuer.canister_id,
                enabled: true,
                allowed_audiences: vec![audience.clone()],
                allowed_grants: grants.clone(),
                max_cert_ttl_ns: 60_000_000_000,
                refresh_after_ratio_bps: 8_000,
            },
        ),),
    );
    let RootCommandResponse::UpsertIssuerPolicy(policy) =
        policy.expect("root issuer policy must be accepted")
    else {
        panic!("unexpected Root command response")
    };
    assert_eq!(policy.issuer.issuer_pid, fixture.issuer.canister_id);

    let template: Result<RootCommandResponse, Error> = fixture.pic().update_candid_or_panic(
        fixture.root,
        protocol::CANIC_COMMAND,
        (RootCommand::UpsertIssuerRenewalTemplate(
            RootIssuerRenewalTemplateUpsertRequest {
                issuer_pid: fixture.issuer.canister_id,
                enabled: true,
                aud: audience,
                grants,
                cert_ttl_ns: 60_000_000_000,
            },
        ),),
    );
    let RootCommandResponse::UpsertIssuerRenewalTemplate(template) =
        template.expect("root issuer renewal template must be accepted")
    else {
        panic!("unexpected Root command response")
    };
    assert_eq!(template.template.issuer_pid, fixture.issuer.canister_id);
}

fn provision_delegation_proof(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
) {
    let provisioned: Result<(), Error> = fixture.pic().update_candid_or_panic(
        fixture.root,
        "test_provision_chain_key_delegation_proof_for_issuer",
        (fixture.issuer.canister_id,),
    );
    if provisioned.is_err() {
        report_canister_diagnostics(
            fixture.pic(),
            fixture.root,
            candid::Principal::anonymous(),
            "native-agent root proof provisioning",
        );
        report_canister_diagnostics(
            fixture.pic(),
            fixture.issuer.canister_id,
            fixture.root,
            "native-agent issuer proof provisioning",
        );
    }
    provisioned.expect("root proof provisioning must succeed");
}
