//! Module: workflow::placement::scaling::query
//!
//! Responsibility: expose read-only scaling registry query projections.
//! Does not own: worker creation, scaling policy, or endpoint authorization.
//! Boundary: delegates storage reads and maps them into scaling DTO responses.

use crate::{
    dto::placement::scaling::ScalingRegistryResponse,
    ops::storage::placement::scaling::ScalingRegistryOps,
};

///
/// ScalingQuery
///
/// Read-only query facade for scaling registry state.
///

pub struct ScalingQuery;

impl ScalingQuery {
    pub fn registry() -> ScalingRegistryResponse {
        ScalingRegistryOps::entries_response()
    }
}
