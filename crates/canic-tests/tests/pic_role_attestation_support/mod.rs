pub use candid::Principal;
pub use canic_core::dto::{
    auth::{
        RoleAttestationGetRequest, RoleAttestationPrepareResponse, RoleAttestationRequest,
        SignedRoleAttestation,
    },
    capability::{
        CAPABILITY_VERSION_V1, CapabilityProof, CapabilityRequestMetadata, CapabilityService,
        RootCapabilityEnvelopeV1, RootCapabilityResponseV1,
    },
    error::{Error, ErrorCode},
    metrics::{MetricEntry, MetricValue, MetricsKind},
    page::{Page, PageRequest},
    rpc::{CyclesRequest, Request, Response, RootRequestMetadata},
};
pub use canic_core::ids::CanisterRole;
pub use canic_testing_internal::pic::install_test_root_cached;
pub use ic_testkit::pic::Pic;
pub use std::time::Duration;

mod attestation;
mod calls;
mod metrics;

pub use attestation::{
    TEST_ROLE_ATTESTATION_TTL_NS, TEST_SHORT_ROLE_ATTESTATION_TTL_NS, capability_metadata,
    issue_self_attestation,
};
pub use calls::test_progress;
pub use metrics::{metric_count_for_labels, query_metric_entries};
