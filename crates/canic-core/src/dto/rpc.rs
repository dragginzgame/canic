use crate::dto::{
    auth::{
        DelegationProvisionResponse, DelegationRequest, RoleAttestationRequest,
        SignedRoleAttestation,
    },
    prelude::*,
};

///
/// Request
/// Root-directed orchestration commands.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum Request {
    CreateCanister(CreateCanisterRequest),
    UpgradeCanister(UpgradeCanisterRequest),
    Cycles(CyclesRequest),
    IssueDelegation(DelegationRequest),
    IssueRoleAttestation(RoleAttestationRequest),
}

///
/// RequestFamily
/// Stable capability family identifier for request dispatch logic.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestFamily {
    Provision,
    Upgrade,
    MintCycles,
    IssueDelegation,
    IssueRoleAttestation,
}

impl RequestFamily {
    /// label
    ///
    /// Return the canonical family label used across capability checks and logs.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Provision => "Provision",
            Self::Upgrade => "Upgrade",
            Self::MintCycles => "MintCycles",
            Self::IssueDelegation => "IssueDelegation",
            Self::IssueRoleAttestation => "IssueRoleAttestation",
        }
    }
}

impl Request {
    /// create_canister
    ///
    /// Build a root request for canister provisioning.
    #[must_use]
    pub const fn create_canister(request: CreateCanisterRequest) -> Self {
        Self::CreateCanister(request)
    }

    /// upgrade_canister
    ///
    /// Build a root request for upgrading an existing canister.
    #[must_use]
    pub const fn upgrade_canister(request: UpgradeCanisterRequest) -> Self {
        Self::UpgradeCanister(request)
    }

    /// cycles
    ///
    /// Build a root request for minting/transferring cycles.
    #[must_use]
    pub const fn cycles(request: CyclesRequest) -> Self {
        Self::Cycles(request)
    }

    /// issue_delegation
    ///
    /// Build a root request for delegated token issuance.
    #[must_use]
    pub const fn issue_delegation(request: DelegationRequest) -> Self {
        Self::IssueDelegation(request)
    }

    /// issue_role_attestation
    ///
    /// Build a root request for role attestation issuance.
    #[must_use]
    pub const fn issue_role_attestation(request: RoleAttestationRequest) -> Self {
        Self::IssueRoleAttestation(request)
    }

    /// family
    ///
    /// Resolve the request capability family without exposing variant matches at call sites.
    #[must_use]
    pub const fn family(&self) -> RequestFamily {
        match self {
            Self::CreateCanister(_) => RequestFamily::Provision,
            Self::UpgradeCanister(_) => RequestFamily::Upgrade,
            Self::Cycles(_) => RequestFamily::MintCycles,
            Self::IssueDelegation(_) => RequestFamily::IssueDelegation,
            Self::IssueRoleAttestation(_) => RequestFamily::IssueRoleAttestation,
        }
    }

    /// metadata
    ///
    /// Return replay metadata carried by the request variant.
    #[must_use]
    pub const fn metadata(&self) -> Option<RootRequestMetadata> {
        match self {
            Self::CreateCanister(req) => req.metadata,
            Self::UpgradeCanister(req) => req.metadata,
            Self::Cycles(req) => req.metadata,
            Self::IssueDelegation(req) => req.metadata,
            Self::IssueRoleAttestation(req) => req.metadata,
        }
    }

    /// with_metadata
    ///
    /// Attach root replay metadata to the request payload.
    #[must_use]
    pub const fn with_metadata(mut self, metadata: RootRequestMetadata) -> Self {
        match &mut self {
            Self::CreateCanister(req) => req.metadata = Some(metadata),
            Self::UpgradeCanister(req) => req.metadata = Some(metadata),
            Self::Cycles(req) => req.metadata = Some(metadata),
            Self::IssueDelegation(req) => req.metadata = Some(metadata),
            Self::IssueRoleAttestation(req) => req.metadata = Some(metadata),
        }
        self
    }

    /// without_metadata
    ///
    /// Remove root replay metadata for canonical hashing and signature binding.
    #[must_use]
    pub const fn without_metadata(mut self) -> Self {
        match &mut self {
            Self::CreateCanister(req) => req.metadata = None,
            Self::UpgradeCanister(req) => req.metadata = None,
            Self::Cycles(req) => req.metadata = None,
            Self::IssueDelegation(req) => req.metadata = None,
            Self::IssueRoleAttestation(req) => req.metadata = None,
        }
        self
    }

    /// upgrade_request
    ///
    /// Return the upgrade payload when this request belongs to the upgrade family.
    #[must_use]
    pub const fn upgrade_request(&self) -> Option<&UpgradeCanisterRequest> {
        match self {
            Self::UpgradeCanister(request) => Some(request),
            _ => None,
        }
    }
}

///
/// RootCapabilityCommand
/// Internal root capability command shape used by root workflow dispatch.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum RootCapabilityCommand {
    ProvisionCanister(CreateCanisterRequest),
    UpgradeCanister(UpgradeCanisterRequest),
    MintCycles(CyclesRequest),
    IssueDelegation(DelegationRequest),
    IssueRoleAttestation(RoleAttestationRequest),
}

impl From<Request> for RootCapabilityCommand {
    fn from(value: Request) -> Self {
        match value {
            Request::CreateCanister(req) => Self::ProvisionCanister(req),
            Request::UpgradeCanister(req) => Self::UpgradeCanister(req),
            Request::Cycles(req) => Self::MintCycles(req),
            Request::IssueDelegation(req) => Self::IssueDelegation(req),
            Request::IssueRoleAttestation(req) => Self::IssueRoleAttestation(req),
        }
    }
}

///
/// RootRequestMetadata
/// Replay and idempotency metadata for mutating root requests.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootRequestMetadata {
    pub request_id: [u8; 32],
    pub ttl_seconds: u64,
}

///
/// CreateCanisterRequest
/// Payload for [`Request::CreateCanister`]
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CreateCanisterRequest {
    pub canister_role: CanisterRole,
    pub parent: CreateCanisterParent,
    pub extra_arg: Option<Vec<u8>>,
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
}

///
/// CreateCanisterParent
/// Parent-location choices for a new canister
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum CreateCanisterParent {
    Root,
    /// Use the requesting canister as parent.
    ThisCanister,
    /// Use the requesting canister's parent (creates a sibling).
    Parent,
    Canister(Principal),
    Directory(CanisterRole),
}

///
/// UpgradeCanisterRequest
/// Payload for [`Request::UpgradeCanister`]
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct UpgradeCanisterRequest {
    pub canister_pid: Principal,
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
}

///
/// CyclesRequest
/// Payload for [`Request::Cycles`]
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CyclesRequest {
    pub cycles: u128,
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
}

///
/// Response
/// Response payloads produced by root for orchestration requests.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum Response {
    CreateCanister(CreateCanisterResponse),
    UpgradeCanister(UpgradeCanisterResponse),
    Cycles(CyclesResponse),
    DelegationIssued(DelegationProvisionResponse),
    RoleAttestationIssued(SignedRoleAttestation),
}

///
/// CreateCanisterResponse
/// Result of creating and installing a new canister.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CreateCanisterResponse {
    pub new_canister_pid: Principal,
}

///
/// UpgradeCanisterResponse
/// Result of an upgrade request (currently empty, reserved for metadata)
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct UpgradeCanisterResponse {}

///
/// CyclesResponse
/// Result of transferring cycles to a child canister
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CyclesResponse {
    pub cycles_transferred: u128,
}
