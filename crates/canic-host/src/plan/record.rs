//! Retained encoding: v1/v2 deployment bytes stay fixed; v3 adds publication authority.
use super::{ManagedOwner, Plan, PlanAction, UpgradeTarget};
use crate::{Artifact, Error, Network, Result};
use serde::{Deserialize, Serialize};
// Field order and omitted options are part of retained approval digests.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PlanRecord {
    version: u32,
    network: Network,
    controller: String,
    artifact: Artifact,
    target: Option<String>,
    previous_module: Option<String>,
    creation_cycles: u64,
    created_at_ns: u64,
    expires_at_ns: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    deployment: Option<crate::DeploymentName>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    management: Option<ManagedOwner>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    maintenance: Option<canic_management_protocol::custody::Custody>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    maintenance_sibling: Option<canic_management_protocol::custody::Custody>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    publication: Option<crate::Publication>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    store_root: Option<UpgradeTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    maintenance_store: Option<super::StoreMaintenance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    store_upgrade_root: Option<UpgradeTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    store_upgrade_sequence: Option<u64>,
}

fn invalid() -> Error {
    Error::Invalid("incomplete or conflicting retained operation authority".into())
}

impl TryFrom<PlanRecord> for Plan {
    type Error = Error;
    fn try_from(record: PlanRecord) -> Result<Self> {
        let PlanRecord {
            version,
            network,
            controller,
            artifact,
            target,
            previous_module,
            creation_cycles,
            created_at_ns,
            expires_at_ns,
            deployment,
            management,
            maintenance,
            maintenance_sibling,
            publication,
            store_root,
            maintenance_store,
            store_upgrade_root,
            store_upgrade_sequence,
        } = record;
        if store_upgrade_sequence.is_some() && store_upgrade_root.is_none() {
            return Err(invalid());
        }
        if maintenance_store.is_some() && maintenance.is_none() {
            return Err(invalid());
        }
        let action = if let Some(root) = store_upgrade_root {
            if version != 2
                || creation_cycles != 0
                || store_root.is_some()
                || management.is_some()
                || maintenance.is_some()
                || maintenance_sibling.is_some()
                || publication.is_some()
                || maintenance_store.is_some()
            {
                return Err(invalid());
            }
            PlanAction::UpgradeStore {
                target: UpgradeTarget {
                    canister: target.ok_or_else(invalid)?,
                    module: previous_module.ok_or_else(invalid)?,
                },
                root,
                sequence: store_upgrade_sequence,
            }
        } else if let Some(root) = store_root {
            if version != 2
                || target.is_some()
                || previous_module.is_some()
                || management.is_some()
                || maintenance.is_some()
                || maintenance_sibling.is_some()
                || publication.is_some()
            {
                return Err(invalid());
            }
            PlanAction::CreateStore {
                cycles: creation_cycles,
                root,
            }
        } else if let Some(value) = publication {
            if target.is_some()
                || previous_module.is_some()
                || management.is_some()
                || maintenance.is_some()
                || maintenance_sibling.is_some()
                || creation_cycles != 0
            {
                return Err(invalid());
            }
            PlanAction::Publish(value)
        } else {
            match (
                target,
                previous_module,
                management,
                maintenance,
                maintenance_sibling,
            ) {
                (None, None, None, None, None) => PlanAction::Create {
                    cycles: creation_cycles,
                },
                (None, None, Some(owner), None, None)
                    if creation_cycles == canic_management_protocol::CREATION_CYCLES =>
                {
                    PlanAction::CreateChild(owner)
                }
                (Some(canister), Some(module), None, None, None) if creation_cycles == 0 => {
                    PlanAction::Upgrade(UpgradeTarget { canister, module })
                }
                (Some(canister), Some(module), Some(owner), None, None) if creation_cycles == 0 => {
                    PlanAction::UpgradeChild {
                        target: UpgradeTarget { canister, module },
                        owner,
                    }
                }
                (Some(canister), Some(module), None, Some(first), sibling)
                    if creation_cycles == 0 =>
                {
                    PlanAction::MaintainRoot {
                        target: UpgradeTarget { canister, module },
                        children: std::iter::once(first).chain(sibling).collect(),
                        store: maintenance_store.map(Box::new),
                    }
                }
                _ => return Err(invalid()),
            }
        };
        Ok(Self {
            version,
            network,
            controller,
            artifact,
            action,
            created_at_ns,
            expires_at_ns,
            deployment,
        })
    }
}
impl TryFrom<Plan> for PlanRecord {
    type Error = Error;
    fn try_from(plan: Plan) -> Result<Self> {
        let target = plan.target().cloned();
        let previous_module = plan.previous_module().cloned();
        let creation_cycles = plan.creation_cycles();
        let management = plan.management().cloned();
        let publication = plan.publication().cloned();
        let store_root = plan.store_root().cloned();
        let store_upgrade_root = plan.store_upgrade_root().cloned();
        let store_upgrade_sequence = plan.store_upgrade_sequence();
        let maintenance_store = plan.maintenance_store().cloned();
        let (maintenance, maintenance_sibling) = match plan.maintenance() {
            None => (None, None),
            Some([first]) => (Some(first.clone()), None),
            Some([first, second]) => (Some(first.clone()), Some(second.clone())),
            _ => return Err(invalid()),
        };
        Ok(Self {
            version: plan.version,
            network: plan.network,
            controller: plan.controller,
            artifact: plan.artifact,
            target,
            previous_module,
            creation_cycles,
            created_at_ns: plan.created_at_ns,
            expires_at_ns: plan.expires_at_ns,
            deployment: plan.deployment,
            management,
            maintenance,
            maintenance_sibling,
            publication,
            store_root,
            maintenance_store,
            store_upgrade_root,
            store_upgrade_sequence,
        })
    }
}
impl Serialize for Plan {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        PlanRecord::try_from(self.clone())
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}
