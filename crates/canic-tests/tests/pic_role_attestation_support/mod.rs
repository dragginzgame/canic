pub use candid::Principal;
pub use canic_core::api::rpc::RpcApi;
pub use canic_core::dto::{
    auth::{AttestationKeyStatus, RoleAttestationRequest, SignedRoleAttestation},
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
pub use canic_core::ids::CanisterRole;
pub use canic_testing_internal::pic::install_test_root_cached;
pub use ic_testkit::pic::Pic;
pub use std::time::Duration;

mod attestation;
mod calls;
mod metrics;

pub use attestation::{
    NS_PER_SEC, TEST_ROLE_ATTESTATION_TTL_NS, TEST_SHORT_ROLE_ATTESTATION_TTL_NS,
    capability_metadata, cycles_role_attestation_envelope, encode_delegated_grant_capability_proof,
    encode_role_attestation_capability_proof, issue_self_attestation, issue_self_attestation_as,
    root_capability_hash,
};
pub use calls::test_progress;
pub use metrics::{metric_count_for_labels, query_metric_entries};
