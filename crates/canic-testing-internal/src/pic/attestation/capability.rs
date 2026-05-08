use candid::Principal;
use canic::Error;
use canic_core::api::rpc::RpcApi;
use canic_core::dto::{
    auth::SignedRoleAttestation,
    capability::{
        CAPABILITY_VERSION_V1, CapabilityProof, CapabilityService, PROOF_VERSION_V1,
        RoleAttestationProof, RootCapabilityEnvelopeV1, RootCapabilityResponseV1,
    },
    rpc::{CreateCanisterParent, CreateCanisterRequest, Request, Response},
};
use canic_core::ids::CanisterRole;
use canic_testkit::pic::{Pic, wait_until_ready as wait_for_ready_canister};
use serde::de::DeserializeOwned;

use super::fixture::progress;

// Create a non-root verifier canister through the root capability endpoint.
pub(super) fn create_verifier_canister(pic: &Pic, root_id: Principal) -> Principal {
    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, root_id, 0u64),
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
    let verifier_id = match response
        .expect("verifier canister creation capability call must succeed")
        .response
    {
        Response::CreateCanister(res) => res.new_canister_pid,
        other => panic!("expected create-canister response, got: {other:?}"),
    };
    progress("waiting for verifier canister readiness");
    wait_for_ready_canister(pic, verifier_id, 240);
    verifier_id
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

fn encode_role_attestation_capability_proof(proof: RoleAttestationProof) -> CapabilityProof {
    proof
        .try_into()
        .expect("role attestation proof should encode")
}

fn root_capability_hash(root_id: Principal, request: &Request) -> [u8; 32] {
    RpcApi::root_capability_hash(root_id, CAPABILITY_VERSION_V1, request)
        .expect("compute root capability hash")
}

const fn capability_metadata(
    issued_at: u64,
    request_id_seed: u8,
    nonce_seed: u8,
    ttl_seconds: u32,
) -> canic_core::dto::capability::CapabilityRequestMetadata {
    canic_core::dto::capability::CapabilityRequestMetadata {
        request_id: [request_id_seed; 16],
        nonce: [nonce_seed; 16],
        issued_at,
        ttl_seconds,
    }
}
