pub use candid::{Principal, utils::ArgumentEncoder};
pub use canic_core::api::rpc::RpcApi;
pub use canic_core::dto::{
    auth::{AttestationKeyStatus, RoleAttestationRequest, SignedRoleAttestation},
    capability::{
        CAPABILITY_VERSION_V1, CapabilityProof, CapabilityRequestMetadata, CapabilityService,
        DelegatedGrant, DelegatedGrantProof, DelegatedGrantScope, PROOF_VERSION_V1,
        RoleAttestationProof, RootCapabilityEnvelopeV1, RootCapabilityResponseV1,
    },
    error::{Error, ErrorCode},
    rpc::{CyclesRequest, Request, Response},
};
pub use canic_core::ids::CanisterRole;
pub use canic_testing_internal::pic::install_test_root_cached;
pub use canic_testkit::pic::Pic;
pub use serde::de::DeserializeOwned;
pub use std::time::Duration;

mod attestation;
mod calls;

pub use attestation::{
    capability_metadata, cycles_role_attestation_envelope, encode_delegated_grant_capability_proof,
    encode_role_attestation_capability_proof, issue_self_attestation, issue_self_attestation_as,
    root_capability_hash,
};
pub use calls::{PicBorrow, test_progress, update_call_as};
