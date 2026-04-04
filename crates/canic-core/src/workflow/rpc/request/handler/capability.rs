use crate::{
    dto::auth::{DelegationRequest, RoleAttestationRequest},
    dto::rpc::{
        CreateCanisterParent, CreateCanisterRequest, CyclesRequest, RootCapabilityCommand,
        RootRequestMetadata, UpgradeCanisterRequest,
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

    pub(super) fn payload_hash(&self) -> [u8; 32] {
        let mut hasher = super::replay::payload_hasher();

        match self {
            Self::Provision(req) => {
                super::replay::hash_str(&mut hasher, "ProvisionCanister");
                super::replay::hash_role(&mut hasher, &req.canister_role);
                hash_create_canister_parent(&mut hasher, &req.parent);
                super::replay::hash_optional_bytes(&mut hasher, req.extra_arg.as_deref());
            }
            Self::Upgrade(req) => {
                super::replay::hash_str(&mut hasher, "UpgradeCanister");
                super::replay::hash_principal(&mut hasher, &req.canister_pid);
            }
            Self::RequestCycles(req) => {
                super::replay::hash_str(&mut hasher, "RequestCycles");
                super::replay::hash_u128(&mut hasher, req.cycles);
            }
            Self::IssueDelegation(req) => {
                super::replay::hash_str(&mut hasher, "IssueDelegation");
                super::replay::hash_principal(&mut hasher, &req.shard_pid);
                super::replay::hash_strings(&mut hasher, &req.scopes);
                super::replay::hash_principals(&mut hasher, &req.aud);
                super::replay::hash_u64(&mut hasher, req.ttl_secs);
                super::replay::hash_principals(&mut hasher, &req.verifier_targets);
                super::replay::hash_bool(&mut hasher, req.include_root_verifier);
            }
            Self::IssueRoleAttestation(req) => {
                super::replay::hash_str(&mut hasher, "IssueRoleAttestation");
                super::replay::hash_principal(&mut hasher, &req.subject);
                super::replay::hash_role(&mut hasher, &req.role);
                super::replay::hash_optional_principal(&mut hasher, req.subnet_id);
                super::replay::hash_optional_principal(&mut hasher, req.audience);
                super::replay::hash_u64(&mut hasher, req.ttl_secs);
                super::replay::hash_u64(&mut hasher, req.epoch);
            }
        }

        super::replay::finish_payload_hash(hasher)
    }
}

// hash_create_canister_parent
//
// Append the canonical create-parent selector into the replay payload hash.
fn hash_create_canister_parent(hasher: &mut sha2::Sha256, parent: &CreateCanisterParent) {
    match parent {
        CreateCanisterParent::Root => super::replay::hash_str(hasher, "Root"),
        CreateCanisterParent::ThisCanister => super::replay::hash_str(hasher, "ThisCanister"),
        CreateCanisterParent::Parent => super::replay::hash_str(hasher, "Parent"),
        CreateCanisterParent::Canister(pid) => {
            super::replay::hash_str(hasher, "Canister");
            super::replay::hash_principal(hasher, pid);
        }
        CreateCanisterParent::Directory(role) => {
            super::replay::hash_str(hasher, "Directory");
            super::replay::hash_role(hasher, role);
        }
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
