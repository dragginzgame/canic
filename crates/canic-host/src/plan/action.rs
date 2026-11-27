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
    pub sources: Vec<Option<canic_management_protocol::stored::Sources>>,
    /// Exact surviving Store operation; selects complete Root custody when present.
    /// Absence selects child-only observation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<canic_management_protocol::store_maintenance::Status>,
    /// Completed Store predecessors, oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<canic_management_protocol::store_maintenance::Status>,
}

/// One operation with all and only the authority it needs.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "contract", content = "approval", deny_unknown_fields)]
pub enum PlanAction {
    /// One direct user application update; grants no management authority.
    #[serde(rename = "canic:application-call.v1")]
    Call(crate::application::Approval),
    /// Configure or revoke one exact Auth grant; grants no installation authority.
    #[serde(rename = "canic:auth-access.v1")]
    Auth(Box<crate::AuthApproval>),
    /// Publish immutable artifact bytes through Root; grants no installation authority.
    #[serde(rename = "canic:artifact-publication.v1")]
    Publish(crate::Publication),
    /// Remove one exact release through Root; no installation or replacement authority.
    #[serde(rename = "canic:artifact-removal.v1")]
    Remove(crate::PublicationRemoval),
    /// Standalone creation of the selected fleet Root.
    #[serde(rename = "canic:root-bootstrap.v1")]
    BootstrapRoot {
        /// Exact creation debit ceiling.
        cycles: u64,
    },
    /// Direct creation and installation, funded by the operator.
    #[serde(rename = "canic:deployment-create.v1")]
    Create {
        /// Bound install and funding policy for a fresh fleet Root.
        bootstrap: Option<canic_management_protocol::stored::RootBootstrap>,
        /// Exact creation debit ceiling.
        cycles: u64,
    },
    /// Create/install a Store, then hand sole control to the approved Root.
    #[serde(rename = "canic:store-bootstrap.v1")]
    CreateStore {
        /// Exact creation debit ceiling.
        cycles: u64,
        /// Existing Root identity and reviewed code; consumes no application slot.
        root: UpgradeTarget,
    },
    /// Upgrade a directly controlled canister.
    #[serde(rename = "canic:deployment-upgrade.v1")]
    Upgrade(UpgradeTarget),
    /// Root replaces its separate Store under one retained maintenance attempt.
    #[serde(rename = "canic:store-maintenance.v1")]
    UpgradeStore {
        /// Existing Store and its predecessor executable.
        target: UpgradeTarget,
        /// Sole Store controller and enduring effect owner.
        root: UpgradeTarget,
        /// Sequence of this exact Store maintenance attempt.
        sequence: u64,
    },
    /// Root creates its permanently assigned child with the protocol's fixed grant.
    #[serde(rename = "canic:child-creation.v1")]
    CreateChild(ManagedOwner),
    /// Root upgrades its existing child.
    #[serde(rename = "canic:child-upgrade.v1")]
    UpgradeChild {
        /// Existing child and predecessor code.
        target: UpgradeTarget,
        /// Enduring effect owner and exact slot.
        owner: ManagedOwner,
    },
    /// Upgrade Root while preserving every admitted child's complete custody.
    #[serde(rename = "canic:root-maintenance.v1")]
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
    /// Exact application call, independent of controller authority.
    #[must_use]
    pub const fn application_call(&self) -> Option<&crate::application::Approval> {
        match &self.action {
            PlanAction::Call(value) => Some(value),
            _ => None,
        }
    }

    /// Exact Auth approval, separate from deployment and readiness.
    #[must_use]
    pub fn auth(&self) -> Option<&crate::AuthApproval> {
        match &self.action {
            PlanAction::Auth(value) => Some(value),
            _ => None,
        }
    }

    /// Exact removal authority, separate from publication and deployment.
    #[must_use]
    pub fn removal(&self) -> Option<&crate::PublicationRemoval> {
        match &self.action {
            PlanAction::Remove(value) => Some(value),
            _ => None,
        }
    }

    /// Sequence for a Store upgrade. Other actions have no maintenance sequence.
    #[must_use]
    pub fn store_upgrade_sequence(&self) -> Option<u64> {
        match self.action {
            PlanAction::UpgradeStore { sequence, .. } => Some(sequence),
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
            PlanAction::BootstrapRoot { .. }
            | PlanAction::Create { .. }
            | PlanAction::CreateStore { .. }
            | PlanAction::CreateChild(_)
            | PlanAction::Remove(_)
            | PlanAction::Auth(_)
            | PlanAction::Call(_)
            | PlanAction::Publish(_) => None,
        }
    }

    /// Creation debit/attachment; upgrades have no creation authority.
    #[must_use]
    pub fn creation_cycles(&self) -> u64 {
        match self.action {
            PlanAction::BootstrapRoot { cycles }
            | PlanAction::Create { cycles, .. }
            | PlanAction::CreateStore { cycles, .. } => cycles,
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

pub(super) const ROOT_BOOTSTRAP: &str = "canic:root-bootstrap.v1";

impl PlanAction {
    pub(super) const fn contract(&self) -> &'static str {
        match self {
            Self::Call(_) => "canic:application-call.v1",
            Self::Auth(_) => "canic:auth-access.v1",
            Self::Publish(_) => "canic:artifact-publication.v1",
            Self::Remove(_) => "canic:artifact-removal.v1",
            Self::Create { .. } => "canic:deployment-create.v1",
            Self::BootstrapRoot { .. } => ROOT_BOOTSTRAP,
            Self::Upgrade(_) => "canic:deployment-upgrade.v1",
            Self::CreateStore { .. } => "canic:store-bootstrap.v1",
            Self::UpgradeStore { .. } => "canic:store-maintenance.v1",
            Self::CreateChild(_) => "canic:child-creation.v1",
            Self::UpgradeChild { .. } => "canic:child-upgrade.v1",
            Self::MaintainRoot { .. } => "canic:root-maintenance.v1",
        }
    }
}
