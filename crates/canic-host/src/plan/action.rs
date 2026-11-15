//! Semantic operations, independent of the retained approval encoding.
use crate::ManagedOwner;
use canic_management_protocol::custody::Custody;

/// Exact existing canister and code that an upgrade is allowed to replace.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct UpgradeTarget {
    /// Existing canister principal.
    pub canister: String,
    /// Required predecessor module hash.
    pub module: String,
}

/// Store authority and complete artifact evidence preserved during Root maintenance.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct StoreMaintenance {
    /// Exact fleet Store identity and reviewed executable.
    pub store: UpgradeTarget,
    /// Original creation and every upgrade delivery, in permanent child-slot order.
    pub sources: [Option<canic_management_protocol::stored::Sources>; 2],
    /// Exact surviving Store operation; selects complete Root custody when present.
    /// Absence preserves the original child-only observation contract and plan digest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<canic_management_protocol::store_maintenance::Status>,
    /// Completed Store predecessors, oldest first. Omission preserves historical plan bytes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<canic_management_protocol::store_maintenance::Status>,
}

/// One operation with all and only the authority it needs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanAction {
    /// Publish immutable artifact bytes through Root; grants no installation authority.
    Publish(crate::Publication),
    /// Direct creation and installation, funded by the operator.
    Create {
        /// Exact creation debit ceiling.
        cycles: u64,
    },
    /// Create/install a Store, then hand sole control to the approved Root.
    CreateStore {
        /// Exact creation debit ceiling.
        cycles: u64,
        /// Existing Root identity and reviewed code; consumes no application slot.
        root: UpgradeTarget,
    },
    /// Upgrade a directly controlled canister.
    Upgrade(UpgradeTarget),
    /// Root replaces its separate Store under one retained maintenance attempt.
    UpgradeStore {
        /// Existing Store and its predecessor executable.
        target: UpgradeTarget,
        /// Sole Store controller and enduring effect owner.
        root: UpgradeTarget,
        /// Explicit sequence selects exact-attempt observations. Absence is the historical first upgrade.
        sequence: Option<u64>,
    },
    /// Root creates its permanently assigned child with the protocol's fixed grant.
    CreateChild(ManagedOwner),
    /// Root upgrades its existing child.
    UpgradeChild {
        /// Existing child and predecessor code.
        target: UpgradeTarget,
        /// Enduring effect owner and exact slot.
        owner: ManagedOwner,
    },
    /// Upgrade Root while preserving every admitted child's complete custody.
    MaintainRoot {
        /// Existing Root and predecessor code.
        target: UpgradeTarget,
        /// Complete child journals in stable slot order.
        children: Vec<Custody>,
        /// Present when complete Store custody is required; absent preserves inline approvals.
        store: Option<Box<StoreMaintenance>>,
    },
}

impl super::Plan {
    /// Explicit Store maintenance sequence, absent in historical first-upgrade plans.
    #[must_use]
    pub fn store_upgrade_sequence(&self) -> Option<u64> {
        match self.action {
            PlanAction::UpgradeStore { sequence, .. } => sequence,
            _ => None,
        }
    }
    /// Enduring Store upgrade owner, independent of child slot authority.
    #[must_use]
    pub fn store_upgrade_root(&self) -> Option<&UpgradeTarget> {
        match &self.action {
            PlanAction::UpgradeStore { root, .. } => Some(root),
            _ => None,
        }
    }
    /// Approved Store controller, only for infrastructure bootstrap.
    #[must_use]
    pub fn store_root(&self) -> Option<&UpgradeTarget> {
        match &self.action {
            PlanAction::CreateStore { root, .. } => Some(root),
            _ => None,
        }
    }
    /// Exact publication authority, independent of deployment slots and receipts.
    #[must_use]
    pub fn publication(&self) -> Option<&crate::Publication> {
        match &self.action {
            PlanAction::Publish(value) => Some(value),
            _ => None,
        }
    }

    /// Existing target, only for an upgrade.
    #[must_use]
    pub fn target(&self) -> Option<&String> {
        self.upgrade_target().map(|t| &t.canister)
    }
    /// Predecessor code, only for an upgrade.
    #[must_use]
    pub fn previous_module(&self) -> Option<&String> {
        self.upgrade_target().map(|t| &t.module)
    }
    fn upgrade_target(&self) -> Option<&UpgradeTarget> {
        match &self.action {
            PlanAction::Upgrade(target)
            | PlanAction::UpgradeStore { target, .. }
            | PlanAction::UpgradeChild { target, .. }
            | PlanAction::MaintainRoot { target, .. } => Some(target),
            PlanAction::Create { .. }
            | PlanAction::CreateStore { .. }
            | PlanAction::CreateChild(_)
            | PlanAction::Publish(_) => None,
        }
    }
    /// Creation debit/attachment; upgrades have no creation authority.
    #[must_use]
    pub fn creation_cycles(&self) -> u64 {
        match self.action {
            PlanAction::Create { cycles } | PlanAction::CreateStore { cycles, .. } => cycles,
            PlanAction::CreateChild(_) => canic_management_protocol::CREATION_CYCLES,
            _ => 0,
        }
    }
    /// Enduring effect owner, only for child operations.
    #[must_use]
    pub fn management(&self) -> Option<&ManagedOwner> {
        match &self.action {
            PlanAction::CreateChild(owner) | PlanAction::UpgradeChild { owner, .. } => Some(owner),
            _ => None,
        }
    }
    /// Complete child custody, only for Root maintenance.
    #[must_use]
    pub fn maintenance(&self) -> Option<&[Custody]> {
        match &self.action {
            PlanAction::MaintainRoot { children, .. } => Some(children),
            _ => None,
        }
    }
    /// Complete Store artifact evidence, only for Store-aware Root maintenance.
    #[must_use]
    pub fn maintenance_store(&self) -> Option<&StoreMaintenance> {
        match &self.action {
            PlanAction::MaintainRoot { store, .. } => store.as_deref(),
            _ => None,
        }
    }
}
