//!
//! Minimal root stub for PocketIC sharding tests.
//!

use canic::{
    Error, cdk,
    dto::auth::{
        AttestationKey, AttestationKeySet, AttestationKeyStatus, RoleAttestation,
        RoleAttestationRequest, SignedRoleAttestation,
    },
    dto::capability::{RootCapabilityEnvelopeV1, RootCapabilityResponseV1},
    dto::rpc::{
        CreateCanisterResponse, CyclesResponse, Request, Response, UpgradeCanisterResponse,
    },
};

const CREATE_CANISTER_CYCLES: u128 = 1_000_000_000_000;
const STUB_ATTESTATION_KEY_ID: u32 = 1;

#[cdk::init]
fn init() {}

#[cdk::update]
async fn canic_response_capability_v1(
    envelope: RootCapabilityEnvelopeV1,
) -> Result<RootCapabilityResponseV1, Error> {
    let response = handle_request(envelope.capability).await?;
    Ok(RootCapabilityResponseV1 { response })
}

#[cdk::update]
async fn canic_request_role_attestation(
    request: RoleAttestationRequest,
) -> Result<SignedRoleAttestation, Error> {
    if request.ttl_secs == 0 {
        return Err(Error::invalid("ttl_secs must be greater than zero"));
    }

    let issued_at = cdk::api::time() / 1_000_000_000;
    let expires_at = issued_at.saturating_add(request.ttl_secs);

    Ok(SignedRoleAttestation {
        payload: RoleAttestation {
            subject: request.subject,
            role: request.role,
            subnet_id: request.subnet_id,
            audience: request.audience,
            issued_at,
            expires_at,
            epoch: request.epoch,
        },
        signature: Vec::new(),
        key_id: STUB_ATTESTATION_KEY_ID,
    })
}

#[cdk::update]
async fn canic_attestation_key_set() -> Result<AttestationKeySet, Error> {
    let now = cdk::api::time() / 1_000_000_000;

    Ok(AttestationKeySet {
        root_pid: cdk::api::canister_self(),
        generated_at: now,
        keys: vec![AttestationKey {
            key_id: STUB_ATTESTATION_KEY_ID,
            public_key: Vec::new(),
            status: AttestationKeyStatus::Current,
            valid_from: Some(now),
            valid_until: None,
        }],
    })
}

async fn handle_request(request: Request) -> Result<Response, Error> {
    match request {
        Request::CreateCanister(_) => {
            let pid = create_canister().await?;
            Ok(Response::CreateCanister(CreateCanisterResponse {
                new_canister_pid: pid,
            }))
        }
        Request::UpgradeCanister(_) => Ok(Response::UpgradeCanister(UpgradeCanisterResponse {})),
        Request::Cycles(req) => Ok(Response::Cycles(CyclesResponse {
            cycles_transferred: req.cycles,
        })),
        Request::IssueDelegation(_) => Err(Error::invalid(
            "issue_delegation unsupported in sharding_root_stub",
        )),
        Request::IssueRoleAttestation(_) => Err(Error::invalid(
            "issue_role_attestation unsupported in sharding_root_stub",
        )),
    }
}

async fn create_canister() -> Result<cdk::types::Principal, Error> {
    let args = cdk::mgmt::CreateCanisterArgs { settings: None };

    let res = cdk::mgmt::create_canister_with_extra_cycles(&args, CREATE_CANISTER_CYCLES)
        .await
        .map_err(|err| Error::internal(format!("create_canister failed: {err}")))?;

    Ok(res.canister_id)
}

#[cfg(debug_assertions)]
cdk::export_candid!();
