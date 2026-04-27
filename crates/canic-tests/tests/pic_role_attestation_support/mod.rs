pub use candid::{Principal, utils::ArgumentEncoder};
pub use canic_core::api::rpc::RpcApi;
pub use canic_core::dto::{
    auth::{
        AttestationKeyStatus, DelegatedToken, DelegatedTokenClaims, DelegationAdminCommand,
        DelegationAdminResponse, DelegationAudience, DelegationProofInstallIntent,
        DelegationProofInstallRequest, DelegationProvisionStatus,
        DelegationVerifierProofPushRequest, RoleAttestationRequest, SignedRoleAttestation,
    },
    capability::{
        CAPABILITY_VERSION_V1, CapabilityProof, CapabilityRequestMetadata, CapabilityService,
        DelegatedGrant, DelegatedGrantProof, DelegatedGrantScope, PROOF_VERSION_V1,
        RoleAttestationProof, RootCapabilityEnvelopeV1, RootCapabilityResponseV1,
    },
    error::{Error, ErrorCode},
    metrics::{MetricEntry, MetricValue, MetricsKind},
    page::{Page, PageRequest},
    rpc::{CyclesRequest, Request, Response},
};
pub use canic_core::ids::{CanisterRole, cap};
pub use canic_testing_internal::pic::{
    CachedInstalledRoot, install_test_root_cached, install_test_root_with_verifier_cached,
    install_test_root_without_test_material_cached, signer_pid,
};
pub use canic_testkit::pic::{Pic, wait_until_ready as wait_for_ready_canister};
pub use serde::de::DeserializeOwned;
pub use std::time::Duration;

mod attestation;
mod calls;
mod delegation;
mod metrics;

pub use attestation::{
    capability_metadata, cycles_role_attestation_envelope, encode_delegated_grant_capability_proof,
    encode_role_attestation_capability_proof, issue_self_attestation, issue_self_attestation_as,
    root_capability_hash,
};
pub use calls::{PicBorrow, query_call_as, test_progress, update_call_as, update_call_raw_as};
pub use delegation::{
    assert_token_verify_proof_missing, bogus_delegated_token, delegation_admin_fixture,
    install_root_test_delegation_material, install_signer_test_delegation_material,
    prewarm_verifiers, repair_verifiers,
};
pub use metrics::{access_metric_count, assert_access_metrics};
