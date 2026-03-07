use crate::{
    InternalError,
    dto::auth::{DelegationRequest, RoleAttestationRequest},
    dto::rpc::{
        CreateCanisterRequest, CyclesRequest, RootCapabilityRequest, RootRequestMetadata,
        UpgradeCanisterRequest,
    },
    ops::runtime::metrics::root_capability::RootCapabilityMetricKey,
};

#[derive(Clone, Debug)]
pub(super) enum RootCapability {
    Provision(CreateCanisterRequest),
    Upgrade(UpgradeCanisterRequest),
    MintCycles(CyclesRequest),
    IssueDelegation(DelegationRequest),
    IssueRoleAttestation(RoleAttestationRequest),
}

impl RootCapability {
    pub(super) const fn capability_name(&self) -> &'static str {
        match self {
            Self::Provision(_) => "Provision",
            Self::Upgrade(_) => "Upgrade",
            Self::MintCycles(_) => "MintCycles",
            Self::IssueDelegation(_) => "IssueDelegation",
            Self::IssueRoleAttestation(_) => "IssueRoleAttestation",
        }
    }

    pub(super) const fn metadata(&self) -> Option<RootRequestMetadata> {
        match self {
            Self::Provision(req) => req.metadata,
            Self::Upgrade(req) => req.metadata,
            Self::MintCycles(req) => req.metadata,
            Self::IssueDelegation(req) => req.metadata,
            Self::IssueRoleAttestation(req) => req.metadata,
        }
    }

    pub(super) const fn metric_key(&self) -> RootCapabilityMetricKey {
        match self {
            Self::Provision(_) => RootCapabilityMetricKey::Provision,
            Self::Upgrade(_) => RootCapabilityMetricKey::Upgrade,
            Self::MintCycles(_) => RootCapabilityMetricKey::MintCycles,
            Self::IssueDelegation(_) => RootCapabilityMetricKey::IssueDelegation,
            Self::IssueRoleAttestation(_) => RootCapabilityMetricKey::IssueRoleAttestation,
        }
    }

    pub(super) fn payload_hash(&self) -> Result<[u8; 32], InternalError> {
        let canonical = match self {
            Self::Provision(req) => {
                let mut canonical = req.clone();
                canonical.metadata = None;
                RootCapabilityRequest::ProvisionCanister(canonical)
            }
            Self::Upgrade(req) => {
                let mut canonical = req.clone();
                canonical.metadata = None;
                RootCapabilityRequest::UpgradeCanister(canonical)
            }
            Self::MintCycles(req) => {
                let mut canonical = req.clone();
                canonical.metadata = None;
                RootCapabilityRequest::MintCycles(canonical)
            }
            Self::IssueDelegation(req) => {
                let mut canonical = req.clone();
                canonical.metadata = None;
                RootCapabilityRequest::IssueDelegation(canonical)
            }
            Self::IssueRoleAttestation(req) => {
                let mut canonical = req.clone();
                canonical.metadata = None;
                RootCapabilityRequest::IssueRoleAttestation(canonical)
            }
        };

        super::replay::hash_capability_payload(&canonical)
    }
}

pub(super) fn map_request(req: RootCapabilityRequest) -> RootCapability {
    match req {
        RootCapabilityRequest::ProvisionCanister(req) => RootCapability::Provision(req),
        RootCapabilityRequest::UpgradeCanister(req) => RootCapability::Upgrade(req),
        RootCapabilityRequest::MintCycles(req) => RootCapability::MintCycles(req),
        RootCapabilityRequest::IssueDelegation(req) => RootCapability::IssueDelegation(req),
        RootCapabilityRequest::IssueRoleAttestation(req) => {
            RootCapability::IssueRoleAttestation(req)
        }
    }
}
