//! Typed local evidence. JSON is an output format, never an internal transport.
use crate::{Cancellation, FleetPlan, Plan, PublicationProgress, Receipt};
use canic_management_protocol::{Status, store_maintenance, stored, upgrade};
use serde::Serialize;
use std::collections::BTreeMap;

/// Local evidence for one retained approval; not a live observation.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum OperationStatus {
    /// A whole-fleet approval and its independently retained steps.
    Fleet(Box<FleetStatus>),
    /// One deployment, maintenance or publication approval.
    Canister(Box<CanisterStatus>),
}

/// Retained evidence for a single approval.
#[derive(Debug, Serialize)]
pub struct CanisterStatus {
    /// Exact approval digest.
    pub operation: String,
    /// Validated original plan.
    pub plan: Plan,
    /// Local request custody; a reply alone does not prove success.
    pub effects: BTreeMap<String, String>,
    /// Historical deployment completion, when retained.
    pub receipt: Option<Receipt>,
    /// Authenticated child progress, when retained.
    pub management: Option<ManagementStatus>,
    /// Operation-specific evidence, preserving the existing JSON fields.
    #[serde(flatten)]
    pub detail: StatusDetail,
}

/// Child creation and upgrade have different evidence contracts.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ManagementStatus {
    /// Retained creation evidence.
    Creation(Status),
    /// Retained upgrade evidence.
    Upgrade(upgrade::Status),
}

/// Additional evidence owned by artifact delivery, publication or Store maintenance.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum StatusDetail {
    /// Exact historical Auth reply; no readiness or enrollment claim.
    Auth {
        /// Validated original response, if retained.
        auth: Option<crate::AuthReceipt>,
    },
    /// Removal has no deployment receipt.
    Removal {
        /// Exact successful response, if retained.
        removal: Option<crate::RemovalReceipt>,
    },
    /// Deployment has no additional fields.
    Deployment {},
    /// Store-backed child delivery retained from authenticated Root observations.
    StoredChild {
        /// No observation means unknown, including after an admission-only reply.
        delivery: Option<Box<stored::Delivery>>,
    },
    /// Publication completion is distinct from deployment completion.
    Publication {
        /// Validated retained publication, or an unconfirmed outcome.
        publication: Option<Box<PublicationProgress>>,
    },
    /// Store maintenance retains its own exact attempt.
    StoreMaintenance {
        /// Authenticated progress, or no retained observation.
        store_maintenance: Option<Box<store_maintenance::Status>>,
    },
}

/// One named fleet step; absence never proves no remote effect.
#[derive(Debug, Serialize)]
pub struct FleetStepStatus {
    /// Application definition name.
    pub name: String,
    /// Materialized step evidence, if present.
    pub state: Option<CanisterStatus>,
}

/// Application step and its separate artifact publication.
#[derive(Debug, Serialize)]
pub struct FleetChildStatus {
    /// Application definition name.
    pub name: String,
    /// Materialized deployment evidence, if present.
    pub state: Option<CanisterStatus>,
    /// Materialized publication evidence, if present.
    pub publication: Option<CanisterStatus>,
}

/// Whole-fleet evidence under the original approval.
#[derive(Debug, Serialize)]
pub struct FleetStatus {
    /// Exact fleet approval digest.
    pub operation: String,
    /// Validated original fleet plan.
    pub fleet_plan: FleetPlan,
    /// Root step evidence.
    pub root: CanisterStatus,
    /// Retained revocation before effect preparation, if present.
    pub cancellation: Option<Cancellation>,
    /// Optional infrastructure, outside application slots.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<FleetStepStatus>,
    /// Application steps in permanent slot order.
    pub children: Vec<FleetChildStatus>,
}
