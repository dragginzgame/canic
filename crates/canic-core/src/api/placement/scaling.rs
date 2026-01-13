use crate::{
    Error,
    cdk::types::Principal,
    dto::placement::scaling::ScalingRegistryView,
    workflow::placement::scaling::{ScalingWorkflow, query::ScalingQuery},
};

///
/// ScalingApi
///

pub struct ScalingApi;

impl ScalingApi {
    /// API wrapper that exposes worker creation by delegating to the scaling workflow.
    pub async fn create_worker(pool: &str) -> Result<Principal, Error> {
        ScalingWorkflow::create_worker(pool)
            .await
            .map_err(Error::from)
    }

    /// API wrapper that exposes the scaling decision (dry-run) via the workflow.
    pub fn plan_create_worker(pool: &str) -> Result<bool, Error> {
        ScalingWorkflow::plan_create_worker(pool).map_err(Error::from)
    }

    #[must_use]
    pub fn registry_view() -> ScalingRegistryView {
        ScalingQuery::registry_view()
    }
}
