//! Exact lifecycle arguments for Store-backed applications. No service policy or engine.
use crate::{children::ChildId, stored};
use serde::{Deserialize, Serialize};

/// Maximum explicit application argument, including its method-bound frame.
pub const MAX_ARGUMENT_BYTES: usize = 4096;
/// Bounded argument retained before dispatch; empty legacy arguments use old contracts.
pub type Arguments = ic_wire::Bytes<MAX_ARGUMENT_BYTES>;
/// Admit a configured Store-backed child.
pub const STAGE: u64 = 61;
/// Reconcile only that complete configured creation.
pub const RECONCILE: u64 = 62;
/// Child-only deployment evidence. Never an input to Root maintenance.
pub const CHILD_EVIDENCE: u64 = ic_wire::method_id("canic.root.select_child_deployments.v1");
/// Complete Root custody including lifecycle arguments.
pub const CUSTODY: u64 = ic_wire::method_id("canic.root.select_root_custody.v1");
/// Admit an exact Blob maintenance upgrade.
pub const STAGE_UPGRADE: u64 = 64;
/// Reconcile that upgrade without issuing a replacement effect.
pub const RECONCILE_UPGRADE: u64 = 65;
/// Separately authorize activation after verified reopening.
pub const ACTIVATE: u64 = 66;
/// Complete custody within the retained artifact byte budget.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: crate::custody::ROOT_HISTORY_LIMITS.bytes + 1_048_576,
    allocation: crate::custody::ROOT_HISTORY_LIMITS.allocation + 1_048_576 * 9,
    nodes: crate::custody::ROOT_HISTORY_LIMITS.nodes + 2048,
    depth: crate::custody::ROOT_HISTORY_LIMITS.depth + 2,
};
/// Stage or recover the exact Store artifact, including application lifecycle inputs when present.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Deployment<S, A> {
    /// The approved Store release and operation.
    Stored(S),
    /// The same authority bound to exact application lifecycle bytes.
    Application(A),
}
/// Inputs for creating a child from the fleet Store.
pub type Creation = Deployment<stored::Stage<crate::Approval>, Stage<crate::Approval>>;
/// Inputs for upgrading a child from the fleet Store.
pub type Replacement = Deployment<stored::Stage<crate::upgrade::Approval>, Upgrade>;
/// Immutable effect authority, source and exact application bytes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stage<A> {
    /// Existing artifact and effect authority.
    pub source: stored::Stage<A>,
    /// Bytes supplied unchanged to the IC installation call.
    pub arguments: Arguments,
}
impl<A: Serialize> Stage<A> {
    /// Exact retention identity includes the arguments and stable child slot.
    /// # Panics
    /// Traps if the input exceeds the admitted metadata envelope.
    #[must_use]
    pub fn pin(&self, method: u64, slot: ChildId) -> ic_wasm_store_protocol::Pin {
        let bytes = ic_wire::encode(&(method, slot, self), ic_wire::Encoding::Payload, LIMITS)
            .expect("bounded application approval");
        ic_wasm_store_protocol::Pin {
            operation: crate::digest(&bytes),
            manifest: self.source.reference.manifest.clone(),
        }
    }
}
/// Retained application bytes, independently of artifact delivery progress.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lifecycle {
    /// Original initialization bytes.
    pub creation: Arguments,
    /// Each admitted upgrade's bytes in permanent journal order.
    pub upgrades: Vec<Maintenance>,
}
/// Exact upgrade inputs; Blob owns admission of the monotonic attempt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Upgrade {
    /// Approved artifact, target, interval and upgrade bytes.
    pub source: Stage<crate::upgrade::Approval>,
    /// Exact supplier maintenance attempt, never inferred from latest state.
    pub attempt: u64,
}
impl Upgrade {
    /// Pin identity includes the exact supplier attempt.
    /// # Panics
    /// Traps if the bounded metadata cannot encode.
    #[must_use]
    pub fn pin(&self, slot: ChildId) -> ic_wasm_store_protocol::Pin {
        let bytes = ic_wire::encode(
            &(STAGE_UPGRADE, slot, self),
            ic_wire::Encoding::Payload,
            LIMITS,
        )
        .expect("bounded maintenance approval");
        ic_wasm_store_protocol::Pin {
            operation: crate::digest(&bytes),
            manifest: self.source.source.reference.manifest.clone(),
        }
    }
}
/// Application lifecycle evidence alongside the existing upgrade journal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Maintenance {
    /// Original controller that admitted this operation.
    pub operator: String,
    /// Exact upgrade arguments.
    pub arguments: Arguments,
    /// Exact supplier attempt.
    pub attempt: u64,
    /// Committed progress; Issued phases never authorize resubmission.
    pub phase: MaintenancePhase,
    /// Drain, stop, start and activation request commitments.
    pub requests: [Option<[u8; 32]>; 4],
    /// Replicated closed management state retained before stop.
    pub before: Option<Arguments>,
    /// Identical replicated closed management state after reopening.
    pub after: Option<Arguments>,
    /// Controller that separately authorized activation, retained before dispatch.
    pub activator: Option<String>,
}
/// One bounded Blob maintenance sequence, with no provider or recovery engine.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum MaintenancePhase {
    /// No maintenance effect has been issued.
    Scheduled,
    /// Original drain request was issued.
    DrainIssued,
    /// Closed admission was observed and retained.
    Drained,
    /// Original stop request was issued.
    StopIssued,
    /// IC reported the child stopped.
    Stopped,
    /// Original start request was issued after upgrade verification.
    StartIssued,
    /// IC reported the child running.
    Started,
    /// Reopened service matches retained closed custody; awaits explicit activation.
    Inspected,
    /// Original activation was issued under separate controller authorization.
    ActivateIssued,
    /// Replicated management inspection proves that exact attempt active.
    Active,
}
impl Maintenance {
    /// Check bounded local custody independently of supplier business admission.
    #[must_use]
    pub fn valid(&self) -> bool {
        use MaintenancePhase as P;
        let (issued, closed, inspected, activated) = match self.phase {
            P::Scheduled => (0, false, false, false),
            P::DrainIssued => (1, false, false, false),
            P::Drained => (1, true, false, false),
            P::StopIssued | P::Stopped => (2, true, false, false),
            P::StartIssued | P::Started => (3, true, false, false),
            P::Inspected => (3, true, true, false),
            P::ActivateIssued | P::Active => (4, true, true, true),
        };
        self.attempt > 0
            && !self.arguments.as_slice().is_empty()
            && self.operator.len() <= 63
            && !self.operator.is_empty()
            && self
                .requests
                .iter()
                .enumerate()
                .all(|(i, r)| r.is_some() == (i < issued))
            && self.before.is_some() == closed
            && self.after.is_some() == inspected
            && (!inspected || self.before == self.after)
            && self.activator.is_some() == activated
            && self.activator.as_ref().is_none_or(|a| a == &self.operator)
    }
}
/// Full custody; old observers must refuse rather than omit these bindings.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Custody {
    /// Permanent service slots, including empty slots, in service order.
    pub children: Vec<ChildCustody>,
    /// Latest Store operation and earlier attempts in admission order.
    pub store_operation: Option<crate::store_maintenance::Status>,
    /// Completed predecessors of the current Store operation.
    pub store_history: Vec<crate::store_maintenance::Status>,
}

/// All evidence for one identified service slot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChildCustody {
    /// Permanent service identity.
    pub slot: ChildId,
    /// Original creation and upgrade journals.
    pub journal: crate::custody::Custody,
    /// Store deliveries for the admitted child.
    pub sources: Option<crate::stored::Sources>,
    /// Application-owned lifecycle arguments when configured.
    pub lifecycle: Option<Lifecycle>,
}

/// Method identity for `stage_child_creation`.
pub const STAGE_CHILD_CREATION: u64 = ic_wire::method_id("canic.root.stage_child_creation.v1");

/// Method identity for `reconcile_child_creation`.
pub const RECONCILE_CHILD_CREATION: u64 =
    ic_wire::method_id("canic.root.reconcile_child_creation.v1");

/// Method identity for `stage_child_upgrade`.
pub const STAGE_CHILD_UPGRADE: u64 = ic_wire::method_id("canic.root.stage_child_upgrade.v1");

/// Method identity for `reconcile_child_upgrade`.
pub const RECONCILE_CHILD_UPGRADE: u64 =
    ic_wire::method_id("canic.root.reconcile_child_upgrade.v1");

/// Controller observation of a single indexed piece of Root maintenance custody.
pub const CUSTODY_PAGE: u64 = ic_wire::method_id("canic.root.maintenance_custody_page.v1");

/// The inventory and per-child counts determine exactly which pages Host must verify.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CustodyKey {
    /// Fleet-wide record counts.
    Inventory,
    /// Original creation evidence for one service slot.
    Child(ChildId),
    /// Zero-based child upgrade, oldest first.
    Upgrade(ChildId, u64),
    /// Zero-based Store attempt, oldest first.
    Store(u64),
}

/// One independently verifiable record from the selected maintenance window.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CustodyItem {
    /// Counts include all service slots and all Store attempts.
    Inventory {
        /// Number of service slots.
        children: u64,
        /// Number of Store attempts including the current attempt.
        store_attempts: u64,
    },
    /// Creation approval, source, arguments and number of admitted upgrades.
    Child {
        /// Retained creation state.
        creation: crate::Status,
        /// Exact retained Store delivery when present.
        source: Option<stored::Delivery>,
        /// Original application bytes.
        arguments: Option<Arguments>,
        /// Number of admitted upgrades.
        upgrades: u64,
    },
    /// Evidence for the single addressed upgrade.
    Upgrade {
        /// Original approval and observed effects.
        status: crate::upgrade::Status,
        /// Exact retained Store delivery when present.
        source: Option<stored::Delivery>,
        /// Optional application maintenance record.
        lifecycle: Option<Maintenance>,
    },
    /// Original Store approval and its complete effect evidence.
    Store(Box<crate::store_maintenance::Status>),
}

/// All pages in one observation must have the same Root incarnation and mutation count.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CustodyPage {
    /// Root open version and in-memory custody write sequence.
    pub revision: (u64, u64),
    /// Exact page being returned.
    pub key: CustodyKey,
    /// Absent when the requested ordinal does not exist.
    pub item: Option<CustodyItem>,
}

impl Custody {
    /// Enumerate every required record including empty slots and the current Store attempt.
    pub fn keys(&self) -> impl Iterator<Item = CustodyKey> + '_ {
        std::iter::once(CustodyKey::Inventory)
            .chain(self.children.iter().flat_map(|c| {
                std::iter::once(CustodyKey::Child(c.slot)).chain(
                    (0..c.journal.upgrades.len() as u64)
                        .map(move |n| CustodyKey::Upgrade(c.slot, n)),
                )
            }))
            .chain(
                (0..(self.store_history.len() + usize::from(self.store_operation.is_some()))
                    as u64)
                    .map(CustodyKey::Store),
            )
    }

    /// Project the approved local record for an exact remote page.
    #[must_use]
    pub fn item(&self, key: CustodyKey) -> Option<CustodyItem> {
        let child = |slot: ChildId| {
            self.children
                .get(slot.service_index()?)
                .filter(|c| c.slot == slot)
        };
        match key {
            CustodyKey::Inventory => Some(CustodyItem::Inventory {
                children: self.children.len() as u64,
                store_attempts: (self.store_history.len()
                    + usize::from(self.store_operation.is_some()))
                    as u64,
            }),
            CustodyKey::Child(slot) => {
                let c = child(slot)?;
                Some(CustodyItem::Child {
                    creation: c.journal.creation.clone(),
                    source: c.sources.as_ref().map(|s| s.creation.clone()),
                    arguments: c.lifecycle.as_ref().map(|l| l.creation.clone()),
                    upgrades: c.journal.upgrades.len() as u64,
                })
            }
            CustodyKey::Upgrade(slot, n) => {
                let i = usize::try_from(n).ok()?;
                let c = child(slot)?;
                Some(CustodyItem::Upgrade {
                    status: c.journal.upgrades.get(i)?.clone(),
                    source: c.sources.as_ref().and_then(|s| s.upgrades.get(i)).cloned(),
                    lifecycle: c
                        .lifecycle
                        .as_ref()
                        .and_then(|l| l.upgrades.get(i))
                        .cloned(),
                })
            }
            CustodyKey::Store(n) => {
                let i = usize::try_from(n).ok()?;
                let s = if i == self.store_history.len() {
                    self.store_operation.as_ref()
                } else {
                    self.store_history.get(i)
                }?;
                Some(CustodyItem::Store(Box::new(s.clone())))
            }
        }
    }
}

/// Exact child and optional upgrade approval selected for deployment recovery.
pub type DeploymentSelector = crate::children::Request<Option<crate::upgrade::Approval>>;
