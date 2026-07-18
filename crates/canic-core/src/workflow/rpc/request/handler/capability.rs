//! Module: workflow::rpc::request::handler::capability
//!
//! Responsibility: own internal root-capability families, payloads, and replay identity.
//! Does not own: authorization, replay storage, or capability side effects.
//! Boundary: maps passive request DTOs once for capability and handler workflows.

use crate::{
    dto::rpc::{
        AcknowledgePlacementReceiptRequest, CreateCanisterParent, CreateCanisterRequest,
        CyclesRequest, RecycleCanisterRequest, Request, RootRequestMetadata,
        UpgradeCanisterRequest,
    },
    model::replay::{PLACEMENT_CHILD_REPLAY_COMMAND_KIND, ROOT_PROVISION_REPLAY_COMMAND_KIND},
    ops::runtime::metrics::root_capability::RootCapabilityMetricKey,
};

///
/// RootCapability
///
/// Internal workflow envelope for root-bound RPC capabilities.
///
#[derive(Clone, Debug)]
pub(in crate::workflow::rpc) enum RootCapability {
    AcknowledgePlacementReceipt(AcknowledgePlacementReceiptRequest),
    AllocatePlacementChild(CreateCanisterRequest),
    ProvisionCanister(CreateCanisterRequest),
    UpgradeCanister(UpgradeCanisterRequest),
    RecycleCanister(RecycleCanisterRequest),
    RequestCycles(CyclesRequest),
}

///
/// RootCapabilityDescriptor
///
/// Stable capability metadata used by replay, metrics, and logs.
///
#[derive(Clone, Copy)]
pub(in crate::workflow::rpc) struct RootCapabilityDescriptor {
    pub(in crate::workflow::rpc) name: &'static str,
    pub(in crate::workflow::rpc) command_kind: &'static str,
    pub(in crate::workflow::rpc) key: RootCapabilityMetricKey,
}

///
/// RootReplayInput
///
/// Canonical replay input derived from a capability and caller metadata.
///
#[derive(Clone, Copy)]
pub(super) struct RootReplayInput {
    pub(super) descriptor: RootCapabilityDescriptor,
    pub(super) metadata: RootRequestMetadata,
    pub(super) payload_hash: [u8; 32],
}

impl RootCapability {
    /// Map the passive boundary request into its canonical workflow family.
    #[must_use]
    pub(in crate::workflow::rpc) fn from_request(request: Request) -> Self {
        match request {
            Request::AcknowledgePlacementReceipt(request) => {
                Self::AcknowledgePlacementReceipt(request)
            }
            Request::AllocatePlacementChild(request) => Self::AllocatePlacementChild(request),
            Request::CreateCanister(request) => Self::ProvisionCanister(request),
            Request::UpgradeCanister(request) => Self::UpgradeCanister(request),
            Request::RecycleCanister(request) => Self::RecycleCanister(request),
            Request::Cycles(request) => Self::RequestCycles(request),
        }
    }

    /// Attach admitted replay metadata before request execution.
    #[must_use]
    pub(in crate::workflow::rpc) const fn with_metadata(
        mut self,
        metadata: RootRequestMetadata,
    ) -> Self {
        match &mut self {
            Self::AcknowledgePlacementReceipt(request) => request.metadata = Some(metadata),
            Self::AllocatePlacementChild(request) | Self::ProvisionCanister(request) => {
                request.metadata = Some(metadata);
            }
            Self::UpgradeCanister(request) => request.metadata = Some(metadata),
            Self::RecycleCanister(request) => request.metadata = Some(metadata),
            Self::RequestCycles(request) => request.metadata = Some(metadata),
        }
        self
    }

    pub(in crate::workflow::rpc) const fn descriptor(&self) -> RootCapabilityDescriptor {
        match self {
            Self::AcknowledgePlacementReceipt(_) => RootCapabilityDescriptor {
                name: "AcknowledgePlacementReceipt",
                command_kind: "root.acknowledge_placement_receipt",
                key: RootCapabilityMetricKey::AcknowledgePlacementReceipt,
            },
            Self::AllocatePlacementChild(_) => RootCapabilityDescriptor {
                name: "AllocatePlacementChild",
                command_kind: PLACEMENT_CHILD_REPLAY_COMMAND_KIND,
                key: RootCapabilityMetricKey::AllocatePlacementChild,
            },
            Self::ProvisionCanister(_) => RootCapabilityDescriptor {
                name: "Provision",
                command_kind: ROOT_PROVISION_REPLAY_COMMAND_KIND,
                key: RootCapabilityMetricKey::Provision,
            },
            Self::UpgradeCanister(_) => RootCapabilityDescriptor {
                name: "Upgrade",
                command_kind: "root.upgrade.v1",
                key: RootCapabilityMetricKey::Upgrade,
            },
            Self::RecycleCanister(_) => RootCapabilityDescriptor {
                name: "RecycleCanister",
                command_kind: "root.recycle_canister.v1",
                key: RootCapabilityMetricKey::RecycleCanister,
            },
            Self::RequestCycles(_) => RootCapabilityDescriptor {
                name: "RequestCycles",
                command_kind: "root.request_cycles.v1",
                key: RootCapabilityMetricKey::RequestCycles,
            },
        }
    }

    pub(super) fn replay_input(&self) -> Option<RootReplayInput> {
        match self {
            Self::AcknowledgePlacementReceipt(_) => None,
            Self::AllocatePlacementChild(req) => req.metadata.map(|metadata| RootReplayInput {
                descriptor: self.descriptor(),
                metadata,
                payload_hash: hash_create_canister_payload(req, "AllocatePlacementChild"),
            }),
            Self::ProvisionCanister(req) => req.metadata.map(|metadata| RootReplayInput {
                descriptor: self.descriptor(),
                metadata,
                payload_hash: hash_create_canister_payload(req, "ProvisionCanister"),
            }),
            Self::UpgradeCanister(req) => req.metadata.map(|metadata| RootReplayInput {
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
        }
    }

    #[cfg(test)]
    pub(super) fn payload_hash(&self) -> [u8; 32] {
        match self {
            Self::AcknowledgePlacementReceipt(_) => {
                unreachable!("receipt acknowledgement is not replay protected")
            }
            Self::AllocatePlacementChild(req) => {
                hash_create_canister_payload(req, "AllocatePlacementChild")
            }
            Self::ProvisionCanister(req) => hash_create_canister_payload(req, "ProvisionCanister"),
            Self::UpgradeCanister(req) => hash_upgrade_payload(req),
            Self::RecycleCanister(req) => hash_recycle_payload(req),
            Self::RequestCycles(req) => hash_request_cycles_payload(req),
        }
    }
}

// Keep create-parent selectors stable; replay payload hashes are protocol-visible.
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

fn hash_create_canister_payload(req: &CreateCanisterRequest, family: &str) -> [u8; 32] {
    let mut hasher = super::replay::payload_hasher();
    super::replay::hash_str(&mut hasher, family);
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
