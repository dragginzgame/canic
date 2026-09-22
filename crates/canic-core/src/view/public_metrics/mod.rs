//! Module: view::public_metrics
//!
//! Responsibility: carry one read-only history projection from its storage owner.
//! Boundary: no collection, mutation, endpoint DTOs or query authorisation.

use crate::model::public_metrics::PublicHistorySample;

/// One exact series in chronological source-slot order, without storage bindings.
#[derive(Clone, Debug)]
pub struct PublicHistorySeries {
    pub unit: String,
    pub slots: Vec<PublicHistorySample>,
}
