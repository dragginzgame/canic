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
pub const CHILD_EVIDENCE: u64 = ic_wire::method_id("canic.root.observe_child_deployments.v1");
/// Complete Root custody including lifecycle arguments.
pub const CUSTODY: u64 = ic_wire::method_id("canic.root.observe_root_custody.v1");
/// Admit an exact Blob maintenance upgrade.
pub const STAGE_UPGRADE: u64 = 64;
/// Reconcile that upgrade without issuing a replacement effect.
pub const RECONCILE_UPGRADE: u64 = 65;
/// Separately authorize activation after verified reopening.
pub const ACTIVATE: u64 = 66;
/// Complete custody with at most two bounded application argument histories.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: crate::custody::ROOT_HISTORY_LIMITS.bytes
        + crate::children::ChildId::HISTORICAL_SERVICES.len()
            * (1 + crate::upgrade::MAX_UPGRADES)
            * (3 * MAX_ARGUMENT_BYTES + 512),
    allocation: crate::custody::ROOT_HISTORY_LIMITS.allocation
        + crate::children::ChildId::HISTORICAL_SERVICES.len()
            * (1 + crate::upgrade::MAX_UPGRADES)
            * (3 * MAX_ARGUMENT_BYTES + 512)
            * 9,
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
    /// Existing complete Root evidence.
    pub root: crate::custody::RootHistory,
    /// Lifecycle arguments for each permanent slot, absent for legacy children.
    pub applications: Vec<Option<Lifecycle>>,
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
