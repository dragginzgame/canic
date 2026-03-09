use crate::{
    InternalError,
    dto::auth::{DelegationRequest, RoleAttestationRequest},
    dto::rpc::{
        CreateCanisterRequest, CyclesRequest, RootCapabilityCommand, RootRequestMetadata,
        UpgradeCanisterRequest,
    },
    ops::runtime::metrics::root_capability::RootCapabilityMetricKey,
};

#[derive(Clone, Debug)]
pub(super) enum RootCapability {
    Provision(CreateCanisterRequest),
    Upgrade(UpgradeCanisterRequest),
    RequestCycles(CyclesRequest),
    IssueDelegation(DelegationRequest),
    IssueRoleAttestation(RoleAttestationRequest),
}

impl RootCapability {
    pub(super) const fn capability_name(&self) -> &'static str {
        match self {
            Self::Provision(_) => "Provision",
            Self::Upgrade(_) => "Upgrade",
            Self::RequestCycles(_) => "RequestCycles",
            Self::IssueDelegation(_) => "IssueDelegation",
            Self::IssueRoleAttestation(_) => "IssueRoleAttestation",
        }
    }

    pub(super) const fn metadata(&self) -> Option<RootRequestMetadata> {
        match self {
            Self::Provision(req) => req.metadata,
            Self::Upgrade(req) => req.metadata,
            Self::RequestCycles(req) => req.metadata,
            Self::IssueDelegation(req) => req.metadata,
            Self::IssueRoleAttestation(req) => req.metadata,
        }
    }

    pub(super) const fn metric_key(&self) -> RootCapabilityMetricKey {
        match self {
            Self::Provision(_) => RootCapabilityMetricKey::Provision,
            Self::Upgrade(_) => RootCapabilityMetricKey::Upgrade,
            Self::RequestCycles(_) => RootCapabilityMetricKey::RequestCycles,
            Self::IssueDelegation(_) => RootCapabilityMetricKey::IssueDelegation,
            Self::IssueRoleAttestation(_) => RootCapabilityMetricKey::IssueRoleAttestation,
        }
    }

    pub(super) fn payload_hash(&self) -> Result<[u8; 32], InternalError> {
        let canonical = match self {
            Self::Provision(req) => {
                let mut canonical = req.clone();
                canonical.metadata = None;
                RootCapabilityCommand::ProvisionCanister(canonical)
            }
            Self::Upgrade(req) => {
                let mut canonical = req.clone();
                canonical.metadata = None;
                RootCapabilityCommand::UpgradeCanister(canonical)
            }
            Self::RequestCycles(req) => {
                let mut canonical = req.clone();
                canonical.metadata = None;
                RootCapabilityCommand::RequestCycles(canonical)
            }
            Self::IssueDelegation(req) => {
                let mut canonical = req.clone();
                canonical.metadata = None;
                RootCapabilityCommand::IssueDelegation(canonical)
            }
            Self::IssueRoleAttestation(req) => {
                let mut canonical = req.clone();
                canonical.metadata = None;
                RootCapabilityCommand::IssueRoleAttestation(canonical)
            }
        };

        super::replay::hash_capability_payload(&canonical)
    }
}

pub(super) fn map_request(req: RootCapabilityCommand) -> RootCapability {
    match req {
        RootCapabilityCommand::ProvisionCanister(req) => RootCapability::Provision(req),
        RootCapabilityCommand::UpgradeCanister(req) => RootCapability::Upgrade(req),
        RootCapabilityCommand::RequestCycles(req) => RootCapability::RequestCycles(req),
        RootCapabilityCommand::IssueDelegation(req) => RootCapability::IssueDelegation(req),
        RootCapabilityCommand::IssueRoleAttestation(req) => {
            RootCapability::IssueRoleAttestation(req)
        }
    }
}
