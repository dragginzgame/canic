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
    /// Upgrade a directly controlled canister.
    Upgrade(UpgradeTarget),
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
    },
}

impl super::Plan {
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
            | PlanAction::UpgradeChild { target, .. }
            | PlanAction::MaintainRoot { target, .. } => Some(target),
            PlanAction::Create { .. } | PlanAction::CreateChild(_) | PlanAction::Publish(_) => None,
        }
    }
    /// Creation debit/attachment; upgrades have no creation authority.
    #[must_use]
    pub fn creation_cycles(&self) -> u64 {
        match self.action {
            PlanAction::Create { cycles } => cycles,
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
}
