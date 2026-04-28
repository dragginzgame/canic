// Category C - Artifact / deployment test (embedded static config).
// This test relies on embedded config by design (test stub).

use candid::{Principal, encode_one};
use canic::protocol;
use canic_core::{
    api::rpc::RpcApi,
    dto::{
        auth::{DelegatedToken, DelegatedTokenClaims, DelegationAudience, SignedRoleAttestation},
        capability::{
            CAPABILITY_VERSION_V1, CapabilityProof, CapabilityRequestMetadata, CapabilityService,
            PROOF_VERSION_V1, RoleAttestationProof, RootCapabilityEnvelopeV1,
            RootCapabilityResponseV1,
        },
        error::Error,
        placement::directory::DirectoryEntryStatusResponse,
        rpc::{CreateCanisterParent, CreateCanisterRequest, Request, Response},
        subnet::SubnetIdentity,
        topology::SubnetRegistryResponse,
    },
    ids::{CanisterRole, cap},
};
use canic_testkit::{
    artifacts::{
        WasmBuildProfile, build_internal_test_wasm_canisters, read_wasm, test_target_dir,
        workspace_root_for,
    },
    pic::{Pic, PicBuilder, acquire_pic_serial_guard, wait_until_ready},
};
use serde::de::DeserializeOwned;
use std::{
    path::{Path, PathBuf},
    sync::Once,
};

const ROOT_INSTALL_CYCLES: u128 = 120_000_000_000_000;
const ROOT_PACKAGE: [&str; 1] = ["delegation_root_stub"];
static BUILD_ONCE: Once = Once::new();

#[test]
fn directory_resolves_one_key_to_one_instance_and_reuses_it() {
    let workspace_root = workspace_root();
    let target_dir = test_target_dir(&workspace_root, "pic-wasm");
    build_canisters_once(&workspace_root);

    let root_wasm = read_wasm(&target_dir, "delegation_root_stub", WasmBuildProfile::Fast);

    let _serial_guard = acquire_pic_serial_guard();
    let pic = PicBuilder::new()
        .with_ii_subnet()
        .with_application_subnet()
        .build();

    let root_id = pic.create_canister();
    pic.add_cycles(root_id, ROOT_INSTALL_CYCLES);
    pic.install_canister(
        root_id,
        root_wasm,
        encode_one(SubnetIdentity::Manual).expect("encode root init args"),
        None,
    );
    wait_until_ready(&pic, root_id, 240);

    let project_hub_id = create_project_hub(&pic, root_id);
    let (token_subject, token) = issue_project_instance_token(&pic, root_id);

    let alpha_first: Result<DirectoryEntryStatusResponse, Error> = update_call(
        &pic,
        project_hub_id,
        "resolve_project",
        ("alpha".to_string(),),
    );
    let alpha_first = alpha_first.expect("first alpha resolve should succeed");
    let alpha_pid = expect_bound(alpha_first);

    let looked_up_alpha: Result<Option<Principal>, Error> = query_call(
        &pic,
        project_hub_id,
        "lookup_project",
        ("alpha".to_string(),),
    );
    assert_eq!(
        looked_up_alpha.expect("lookup alpha should succeed"),
        Some(alpha_pid)
    );

    let instance_id: Result<Principal, Error> = query_call(&pic, alpha_pid, "instance_id", ());
    assert_eq!(
        instance_id.expect("project instance should be callable"),
        alpha_pid
    );

    let verified_on_new_instance: Result<(), Error> = update_call_as(
        &pic,
        alpha_pid,
        token_subject,
        "instance_verify_token",
        (token,),
    );
    verified_on_new_instance
        .expect("new project instance should receive the old active proof during creation");

    let alpha_second: Result<DirectoryEntryStatusResponse, Error> = update_call(
        &pic,
        project_hub_id,
        "resolve_project",
        ("alpha".to_string(),),
    );
    let alpha_second = alpha_second.expect("second alpha resolve should succeed");
    assert_eq!(expect_bound(alpha_second), alpha_pid);

    let beta: Result<DirectoryEntryStatusResponse, Error> = update_call(
        &pic,
        project_hub_id,
        "resolve_project",
        ("beta".to_string(),),
    );
    let beta = beta.expect("beta resolve should succeed");
    let beta_pid = expect_bound(beta);

    assert_ne!(alpha_pid, beta_pid);
}

// Issue one project-instance-scoped token before any project instance verifier exists.
fn issue_project_instance_token(pic: &Pic, root_id: Principal) -> (Principal, DelegatedToken) {
    let signer_id = signer_pid(pic, root_id);
    let now: Result<u64, Error> = query_call(pic, root_id, "root_now_secs", ());
    let now = now.expect("query root now_secs failed");
    let token_subject = Principal::from_slice(&[91; 29]);
    let claims = DelegatedTokenClaims {
        sub: token_subject,
        shard_pid: signer_id,
        scopes: vec![cap::VERIFY.to_string()],
        aud: DelegationAudience::Roles(vec![CanisterRole::new("project_instance")]),
        iat: now,
        exp: now + 600,
        ext: None,
    };
    let token: Result<DelegatedToken, Error> = update_call_as(
        pic,
        signer_id,
        token_subject,
        "signer_issue_token",
        (claims,),
    );
    let token = token.expect("issue project-instance token failed");

    (token_subject, token)
}

// Resolve the auto-created signer canister from root's subnet registry.
fn signer_pid(pic: &Pic, root_id: Principal) -> Principal {
    let registry: Result<SubnetRegistryResponse, Error> =
        query_call(pic, root_id, protocol::CANIC_SUBNET_REGISTRY, ());
    let SubnetRegistryResponse(entries) = registry.expect("query root subnet registry failed");
    entries
        .into_iter()
        .find(|entry| entry.role == CanisterRole::new("signer"))
        .map(|entry| entry.pid)
        .expect("signer must be registered")
}

// Issue a root-backed create-canister capability request for the embedded project_hub role.
fn create_project_hub(pic: &Pic, root_id: Principal) -> Principal {
    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(root_id), 0u64),
    );
    let issued = issued.expect("attestation issuance failed");
    let issued_at = issued.payload.issued_at;

    let request = Request::CreateCanister(CreateCanisterRequest {
        canister_role: CanisterRole::new("project_hub"),
        parent: CreateCanisterParent::Root,
        extra_arg: None,
        metadata: None,
    });
    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request.clone(),
        proof: encode_role_attestation_capability_proof(RoleAttestationProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash: root_capability_hash(root_id, &request),
            attestation: issued,
        }),
        metadata: capability_metadata(issued_at, 41, 24, 60),
    };
    let response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        pic,
        root_id,
        root_id,
        "canic_response_capability_v1",
        (envelope,),
    );
    let project_hub_id = match response
        .expect("project_hub creation capability call must succeed")
        .response
    {
        Response::CreateCanister(res) => res.new_canister_pid,
        other => panic!("expected create-canister response, got: {other:?}"),
    };

    wait_until_ready(pic, project_hub_id, 240);
    project_hub_id
}

// Extract the resolved instance pid and reject any non-bound status.
fn expect_bound(status: DirectoryEntryStatusResponse) -> Principal {
    match status {
        DirectoryEntryStatusResponse::Bound { instance_pid, .. } => instance_pid,
        DirectoryEntryStatusResponse::Pending { .. } => {
            panic!("expected bound directory entry after resolve_or_create")
        }
    }
}

// Run one typed update call as the requested caller.
fn update_call_as<T, A>(
    pic: &Pic,
    canister_id: Principal,
    caller: Principal,
    method: &str,
    args: A,
) -> T
where
    T: candid::CandidType + DeserializeOwned,
    A: candid::utils::ArgumentEncoder,
{
    pic.update_call_as(canister_id, caller, method, args)
        .expect("update_call failed")
}

// Run one typed update call with PocketIC's default caller.
fn update_call<T, A>(pic: &Pic, canister_id: Principal, method: &str, args: A) -> T
where
    T: candid::CandidType + DeserializeOwned,
    A: candid::utils::ArgumentEncoder,
{
    pic.update_call(canister_id, method, args)
        .expect("update_call failed")
}

// Run one typed query call with PocketIC's default caller.
fn query_call<T, A>(pic: &Pic, canister_id: Principal, method: &str, args: A) -> T
where
    T: candid::CandidType + DeserializeOwned,
    A: candid::utils::ArgumentEncoder,
{
    pic.query_call(canister_id, method, args)
        .expect("query_call failed")
}

// Encode the role-attestation proof into the capability envelope wire shape.
fn encode_role_attestation_capability_proof(proof: RoleAttestationProof) -> CapabilityProof {
    proof
        .try_into()
        .expect("role attestation proof should encode")
}

// Compute the canonical root capability hash for one request payload.
fn root_capability_hash(target_canister: Principal, capability: &Request) -> [u8; 32] {
    RpcApi::root_capability_hash(target_canister, CAPABILITY_VERSION_V1, capability)
        .expect("compute root capability hash")
}

// Build deterministic request metadata for the root capability envelope.
const fn capability_metadata(
    issued_at: u64,
    request_id_seed: u8,
    nonce_seed: u8,
    ttl_seconds: u32,
) -> CapabilityRequestMetadata {
    CapabilityRequestMetadata {
        request_id: [request_id_seed; 16],
        nonce: [nonce_seed; 16],
        issued_at,
        ttl_seconds,
    }
}

// Build the root stub wasm once for the full test process.
fn build_canisters_once(workspace_root: &Path) {
    BUILD_ONCE.call_once(|| {
        let target_dir = test_target_dir(workspace_root, "pic-wasm");
        build_internal_test_wasm_canisters(
            workspace_root,
            &target_dir,
            &ROOT_PACKAGE,
            WasmBuildProfile::Fast,
        );
    });
}

// Resolve the repository root from this test crate.
fn workspace_root() -> PathBuf {
    workspace_root_for(env!("CARGO_MANIFEST_DIR"))
}
