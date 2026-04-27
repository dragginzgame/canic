use crate::{
    dto::auth::{DelegationRequest, RoleAttestationRequest},
    dto::rpc::{
        CreateCanisterParent, CreateCanisterRequest, CyclesRequest, RecycleCanisterRequest,
        Request, RootRequestMetadata, UpgradeCanisterRequest,
    },
    ops::runtime::metrics::root_capability::RootCapabilityMetricKey,
};

#[derive(Clone, Debug)]
pub(super) enum RootCapability {
    Provision(CreateCanisterRequest),
    Upgrade(UpgradeCanisterRequest),
    RecycleCanister(RecycleCanisterRequest),
    RequestCycles(CyclesRequest),
    IssueDelegation(DelegationRequest),
    IssueRoleAttestation(RoleAttestationRequest),
}

#[derive(Clone, Copy)]
pub(super) struct RootCapabilityDescriptor {
    pub(super) name: &'static str,
    pub(super) key: RootCapabilityMetricKey,
}

#[derive(Clone, Copy)]
pub(super) struct RootReplayInput {
    pub(super) descriptor: RootCapabilityDescriptor,
    pub(super) metadata: RootRequestMetadata,
    pub(super) payload_hash: [u8; 32],
}

impl RootCapability {
    pub(super) const fn descriptor(&self) -> RootCapabilityDescriptor {
        match self {
            Self::Provision(_) => RootCapabilityDescriptor {
                name: "Provision",
                key: RootCapabilityMetricKey::Provision,
            },
            Self::Upgrade(_) => RootCapabilityDescriptor {
                name: "Upgrade",
                key: RootCapabilityMetricKey::Upgrade,
            },
            Self::RecycleCanister(_) => RootCapabilityDescriptor {
                name: "RecycleCanister",
                key: RootCapabilityMetricKey::RecycleCanister,
            },
            Self::RequestCycles(_) => RootCapabilityDescriptor {
                name: "RequestCycles",
                key: RootCapabilityMetricKey::RequestCycles,
            },
            Self::IssueDelegation(_) => RootCapabilityDescriptor {
                name: "IssueDelegation",
                key: RootCapabilityMetricKey::IssueDelegation,
            },
            Self::IssueRoleAttestation(_) => RootCapabilityDescriptor {
                name: "IssueRoleAttestation",
                key: RootCapabilityMetricKey::IssueRoleAttestation,
            },
        }
    }

    pub(super) const fn capability_name(&self) -> &'static str {
        self.descriptor().name
    }

    pub(super) fn replay_input(&self) -> Option<RootReplayInput> {
        match self {
            Self::Provision(req) => req.metadata.map(|metadata| RootReplayInput {
                descriptor: self.descriptor(),
                metadata,
                payload_hash: hash_provision_payload(req),
            }),
            Self::Upgrade(req) => req.metadata.map(|metadata| RootReplayInput {
                descriptor: self.descriptor(),
                metadata,
                payload_hash: hash_upgrade_payload(req),
            }),
            Self::RecycleCanister(req) => req.metadata.map(|metadata| RootReplayInput {
                descriptor: self.descriptor(),
                metadata,
                payload_hash: hash_recycle_payload(req),
            }),
            Self::RequestCycles(req) => req.metadata.map(|metadata| RootReplayInput {
                descriptor: self.descriptor(),
                metadata,
                payload_hash: hash_request_cycles_payload(req),
            }),
            Self::IssueDelegation(req) => req.metadata.map(|metadata| RootReplayInput {
                descriptor: self.descriptor(),
                metadata,
                payload_hash: hash_issue_delegation_payload(req),
            }),
            Self::IssueRoleAttestation(req) => req.metadata.map(|metadata| RootReplayInput {
                descriptor: self.descriptor(),
                metadata,
                payload_hash: hash_issue_role_attestation_payload(req),
            }),
        }
    }

    #[cfg(test)]
    pub(super) fn payload_hash(&self) -> [u8; 32] {
        match self {
            Self::Provision(req) => hash_provision_payload(req),
            Self::Upgrade(req) => hash_upgrade_payload(req),
            Self::RecycleCanister(req) => hash_recycle_payload(req),
            Self::RequestCycles(req) => hash_request_cycles_payload(req),
            Self::IssueDelegation(req) => hash_issue_delegation_payload(req),
            Self::IssueRoleAttestation(req) => hash_issue_role_attestation_payload(req),
        }
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
        CreateCanisterParent::Index(role) => {
            super::replay::hash_str(hasher, "Index");
            super::replay::hash_role(hasher, role);
        }
    }
}

fn hash_provision_payload(req: &CreateCanisterRequest) -> [u8; 32] {
    let mut hasher = super::replay::payload_hasher();
    super::replay::hash_str(&mut hasher, "ProvisionCanister");
    super::replay::hash_role(&mut hasher, &req.canister_role);
    hash_create_canister_parent(&mut hasher, &req.parent);
    super::replay::hash_optional_bytes(&mut hasher, req.extra_arg.as_deref());
    super::replay::finish_payload_hash(hasher)
}

fn hash_upgrade_payload(req: &UpgradeCanisterRequest) -> [u8; 32] {
    let mut hasher = super::replay::payload_hasher();
    super::replay::hash_str(&mut hasher, "UpgradeCanister");
    super::replay::hash_principal(&mut hasher, &req.canister_pid);
    super::replay::finish_payload_hash(hasher)
}

fn hash_recycle_payload(req: &RecycleCanisterRequest) -> [u8; 32] {
    let mut hasher = super::replay::payload_hasher();
    super::replay::hash_str(&mut hasher, "RecycleCanister");
    super::replay::hash_principal(&mut hasher, &req.canister_pid);
    super::replay::finish_payload_hash(hasher)
}

fn hash_request_cycles_payload(req: &CyclesRequest) -> [u8; 32] {
    let mut hasher = super::replay::payload_hasher();
    super::replay::hash_str(&mut hasher, "RequestCycles");
    super::replay::hash_u128(&mut hasher, req.cycles);
    super::replay::finish_payload_hash(hasher)
}

fn hash_issue_delegation_payload(req: &DelegationRequest) -> [u8; 32] {
    let mut hasher = super::replay::payload_hasher();
    super::replay::hash_str(&mut hasher, "IssueDelegation");
    super::replay::hash_principal(&mut hasher, &req.shard_pid);
    super::replay::hash_strings(&mut hasher, &req.scopes);
    super::replay::hash_audience(&mut hasher, &req.aud);
    super::replay::hash_u64(&mut hasher, req.ttl_secs);
    crate::perf!("hash_replay_delegation_cert");
    super::replay::hash_principals(&mut hasher, &req.verifier_targets);
    super::replay::hash_bool(&mut hasher, req.include_root_verifier);
    crate::perf!("hash_replay_delegation_targets");
    super::replay::finish_payload_hash(hasher)
}

fn hash_issue_role_attestation_payload(req: &RoleAttestationRequest) -> [u8; 32] {
    let mut hasher = super::replay::payload_hasher();
    super::replay::hash_str(&mut hasher, "IssueRoleAttestation");
    super::replay::hash_principal(&mut hasher, &req.subject);
    super::replay::hash_role(&mut hasher, &req.role);
    super::replay::hash_optional_principal(&mut hasher, req.subnet_id);
    super::replay::hash_optional_principal(&mut hasher, req.audience);
    super::replay::hash_u64(&mut hasher, req.ttl_secs);
    super::replay::hash_u64(&mut hasher, req.epoch);
    super::replay::finish_payload_hash(hasher)
}

pub(super) fn map_request(req: Request) -> RootCapability {
    match req {
        Request::CreateCanister(req) => RootCapability::Provision(req),
        Request::UpgradeCanister(req) => RootCapability::Upgrade(req),
        Request::RecycleCanister(req) => RootCapability::RecycleCanister(req),
        Request::Cycles(req) => RootCapability::RequestCycles(req),
        Request::IssueDelegation(req) => RootCapability::IssueDelegation(req),
        Request::IssueRoleAttestation(req) => RootCapability::IssueRoleAttestation(req),
    }
}
