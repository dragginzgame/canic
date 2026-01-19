use crate::{
    dto::placement::scaling::ScalingRegistryResponse,
    ops::storage::placement::scaling::ScalingRegistryOps,
};

///
/// ScalingQuery
///

pub struct ScalingQuery;

impl ScalingQuery {
    pub fn registry() -> ScalingRegistryResponse {
        ScalingRegistryOps::entries_response()
    }
}
