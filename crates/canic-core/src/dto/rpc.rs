use crate::dto::{
    auth::{
        DelegationProvisionResponse, DelegationRequest, RoleAttestationRequest,
        SignedRoleAttestation,
    },
    prelude::*,
};

//
// Request
//
// Root orchestration request.
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum Request {
    CreateCanister(CreateCanisterRequest),
    UpgradeCanister(UpgradeCanisterRequest),
    RecycleCanister(RecycleCanisterRequest),
    Cycles(CyclesRequest),
    IssueDelegation(DelegationRequest),
    IssueRoleAttestation(RoleAttestationRequest),
}

impl Request {
    // create_canister
    //
    // Build a root request for canister provisioning.
    #[must_use]
    pub const fn create_canister(request: CreateCanisterRequest) -> Self {
        Self::CreateCanister(request)
    }

    // upgrade_canister
    //
    // Build a root request for upgrading an existing canister.
    #[must_use]
    pub const fn upgrade_canister(request: UpgradeCanisterRequest) -> Self {
        Self::UpgradeCanister(request)
    }

    // recycle_canister
    //
    // Build a root request for recycling one child canister back into the pool.
    #[must_use]
    pub const fn recycle_canister(request: RecycleCanisterRequest) -> Self {
        Self::RecycleCanister(request)
    }

    // cycles
    //
    // Build a root request for requesting/transferring cycles.
    #[must_use]
    pub const fn cycles(request: CyclesRequest) -> Self {
        Self::Cycles(request)
    }

    // issue_delegation
    //
    // Build a root request for delegated token issuance.
    #[must_use]
    pub const fn issue_delegation(request: DelegationRequest) -> Self {
        Self::IssueDelegation(request)
    }

    // issue_role_attestation
    //
    // Build a root request for role attestation issuance.
    #[must_use]
    pub const fn issue_role_attestation(request: RoleAttestationRequest) -> Self {
        Self::IssueRoleAttestation(request)
    }

    // family
    //
    // Resolve the request capability family without exposing variant matches at call sites.
    #[must_use]
    pub const fn family(&self) -> RequestFamily {
        match self {
            Self::CreateCanister(_) => RequestFamily::Provision,
            Self::UpgradeCanister(_) => RequestFamily::Upgrade,
            Self::RecycleCanister(_) => RequestFamily::RecycleCanister,
            Self::Cycles(_) => RequestFamily::RequestCycles,
            Self::IssueDelegation(_) => RequestFamily::IssueDelegation,
            Self::IssueRoleAttestation(_) => RequestFamily::IssueRoleAttestation,
        }
    }

    // metadata
    //
    // Return replay metadata carried by the request variant.
    #[must_use]
    pub const fn metadata(&self) -> Option<RootRequestMetadata> {
        match self {
            Self::CreateCanister(req) => req.metadata,
            Self::UpgradeCanister(req) => req.metadata,
            Self::RecycleCanister(req) => req.metadata,
            Self::Cycles(req) => req.metadata,
            Self::IssueDelegation(req) => req.metadata,
            Self::IssueRoleAttestation(req) => req.metadata,
        }
    }

    // with_metadata
    //
    // Attach root replay metadata to the request payload.
    #[must_use]
    pub const fn with_metadata(mut self, metadata: RootRequestMetadata) -> Self {
        match &mut self {
            Self::CreateCanister(req) => req.metadata = Some(metadata),
            Self::UpgradeCanister(req) => req.metadata = Some(metadata),
            Self::RecycleCanister(req) => req.metadata = Some(metadata),
            Self::Cycles(req) => req.metadata = Some(metadata),
            Self::IssueDelegation(req) => req.metadata = Some(metadata),
            Self::IssueRoleAttestation(req) => req.metadata = Some(metadata),
        }
        self
    }

    // without_metadata
    //
    // Remove root replay metadata for canonical hashing and signature binding.
    #[must_use]
    pub const fn without_metadata(mut self) -> Self {
        match &mut self {
            Self::CreateCanister(req) => req.metadata = None,
            Self::UpgradeCanister(req) => req.metadata = None,
            Self::RecycleCanister(req) => req.metadata = None,
            Self::Cycles(req) => req.metadata = None,
            Self::IssueDelegation(req) => req.metadata = None,
            Self::IssueRoleAttestation(req) => req.metadata = None,
        }
        self
    }

    // upgrade_request
    //
    // Return the upgrade payload when this request belongs to the upgrade family.
    #[must_use]
    pub const fn upgrade_request(&self) -> Option<&UpgradeCanisterRequest> {
        match self {
            Self::UpgradeCanister(request) => Some(request),
            _ => None,
        }
    }
}

//
// RequestFamily
//
// Request family label.
//

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestFamily {
    Provision,
    Upgrade,
    RecycleCanister,
    RequestCycles,
    IssueDelegation,
    IssueRoleAttestation,
}

impl RequestFamily {
    // label
    //
    // Return the canonical family label used across capability checks and logs.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Provision => "Provision",
            Self::Upgrade => "Upgrade",
            Self::RecycleCanister => "RecycleCanister",
            Self::RequestCycles => "RequestCycles",
            Self::IssueDelegation => "IssueDelegation",
            Self::IssueRoleAttestation => "IssueRoleAttestation",
        }
    }
}

//
// RootCapabilityCommand
//
// Internal root command.
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum RootCapabilityCommand {
    ProvisionCanister(CreateCanisterRequest),
    UpgradeCanister(UpgradeCanisterRequest),
    RecycleCanister(RecycleCanisterRequest),
    RequestCycles(CyclesRequest),
    IssueDelegation(DelegationRequest),
    IssueRoleAttestation(RoleAttestationRequest),
}

impl From<Request> for RootCapabilityCommand {
    fn from(value: Request) -> Self {
        match value {
            Request::CreateCanister(req) => Self::ProvisionCanister(req),
            Request::UpgradeCanister(req) => Self::UpgradeCanister(req),
            Request::RecycleCanister(req) => Self::RecycleCanister(req),
            Request::Cycles(req) => Self::RequestCycles(req),
            Request::IssueDelegation(req) => Self::IssueDelegation(req),
            Request::IssueRoleAttestation(req) => Self::IssueRoleAttestation(req),
        }
    }
}

//
// RootRequestMetadata
//
// Replay metadata.
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct RootRequestMetadata {
    pub request_id: [u8; 32],
    pub ttl_seconds: u64,
}

//
// CreateCanisterRequest
//
// Create-canister payload.
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CreateCanisterRequest {
    pub canister_role: CanisterRole,
    pub parent: CreateCanisterParent,
    pub extra_arg: Option<Vec<u8>>,
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
}

//
// CreateCanisterParent
//
// Parent selection.
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum CreateCanisterParent {
    Root,
    // Use the requesting canister.
    ThisCanister,
    // Use the caller's parent.
    Parent,
    Canister(Principal),
    Index(CanisterRole),
}

//
// UpgradeCanisterRequest
//
// Upgrade-canister payload.
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct UpgradeCanisterRequest {
    pub canister_pid: Principal,
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
}

//
// RecycleCanisterRequest
//
// Recycle-one-child payload.
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct RecycleCanisterRequest {
    pub canister_pid: Principal,
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
}

//
// CyclesRequest
//
// Cycles payload.
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CyclesRequest {
    pub cycles: u128,
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
}

//
// Response
//
// Root response payload.
//

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum Response {
    CreateCanister(CreateCanisterResponse),
    UpgradeCanister(UpgradeCanisterResponse),
    RecycleCanister(RecycleCanisterResponse),
    Cycles(CyclesResponse),
    DelegationIssued(DelegationProvisionResponse),
    RoleAttestationIssued(SignedRoleAttestation),
}

//
// CreateCanisterResponse
// Result of creating and installing a new canister.
//

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CreateCanisterResponse {
    pub new_canister_pid: Principal,
}

//
// UpgradeCanisterResponse
// Result of an upgrade request (currently empty, reserved for metadata)
//

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct UpgradeCanisterResponse {}

//
// RecycleCanisterResponse
// Result of recycling one canister back into the pool.
//

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct RecycleCanisterResponse {}

//
// CyclesResponse
// Result of transferring cycles to a child canister
//

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CyclesResponse {
    pub cycles_transferred: u128,
}

//
// TESTS
//

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::auth::DelegationAudience;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn metadata(id: u8) -> RootRequestMetadata {
        RootRequestMetadata {
            request_id: [id; 32],
            ttl_seconds: 60,
        }
    }

    fn requests_with_no_metadata() -> Vec<Request> {
        vec![
            Request::create_canister(CreateCanisterRequest {
                canister_role: CanisterRole::new("app"),
                parent: CreateCanisterParent::Root,
                extra_arg: None,
                metadata: None,
            }),
            Request::upgrade_canister(UpgradeCanisterRequest {
                canister_pid: p(2),
                metadata: None,
            }),
            Request::recycle_canister(RecycleCanisterRequest {
                canister_pid: p(7),
                metadata: None,
            }),
            Request::cycles(CyclesRequest {
                cycles: 100,
                metadata: None,
            }),
            Request::issue_delegation(DelegationRequest {
                shard_pid: p(3),
                scopes: vec!["rpc:verify".to_string()],
                aud: DelegationAudience::Roles(vec![CanisterRole::new("app")]),
                ttl_secs: 60,
                shard_public_key_sec1: vec![1, 2, 3],
                metadata: None,
            }),
            Request::issue_role_attestation(RoleAttestationRequest {
                subject: p(5),
                role: CanisterRole::new("test"),
                subnet_id: None,
                audience: Some(p(6)),
                ttl_secs: 60,
                epoch: 0,
                metadata: None,
            }),
        ]
    }

    #[test]
    fn request_family_matches_all_variants() {
        let families: Vec<RequestFamily> = requests_with_no_metadata()
            .iter()
            .map(Request::family)
            .collect();
        assert_eq!(
            families,
            vec![
                RequestFamily::Provision,
                RequestFamily::Upgrade,
                RequestFamily::RecycleCanister,
                RequestFamily::RequestCycles,
                RequestFamily::IssueDelegation,
                RequestFamily::IssueRoleAttestation,
            ]
        );
    }

    #[test]
    fn with_metadata_and_without_metadata_cover_all_variants() {
        let replay_meta = metadata(7);

        for request in requests_with_no_metadata() {
            let with_meta = request.clone().with_metadata(replay_meta);
            assert_eq!(
                with_meta.metadata(),
                Some(replay_meta),
                "with_metadata must set metadata for every request variant"
            );

            let without_meta = with_meta.without_metadata();
            assert_eq!(
                without_meta.metadata(),
                None,
                "without_metadata must strip metadata for every request variant"
            );
        }
    }

    #[test]
    fn upgrade_request_is_only_available_for_upgrade_variant() {
        let upgrade = Request::upgrade_canister(UpgradeCanisterRequest {
            canister_pid: p(9),
            metadata: Some(metadata(9)),
        });
        assert!(upgrade.upgrade_request().is_some());

        for request in requests_with_no_metadata() {
            if !matches!(request, Request::UpgradeCanister(_)) {
                assert!(request.upgrade_request().is_none());
            }
        }
    }
}
