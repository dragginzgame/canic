use crate::{
    cdk::types::Principal,
    dto::{
        error::Error,
        placement::binding::{
            PlacementBindingRecoveryResponse, PlacementBindingRegistryResponse,
            PlacementBindingStatusResponse,
        },
    },
    workflow::placement::binding::{PlacementBindingWorkflow, query::PlacementBindingQuery},
};

///
/// PlacementBindingApi
///

pub struct PlacementBindingApi;

impl PlacementBindingApi {
    #[must_use]
    pub fn lookup_key(pool: &str, key_value: &str) -> Option<Principal> {
        PlacementBindingQuery::lookup_key(pool, key_value)
    }

    #[must_use]
    pub fn lookup_entry(pool: &str, key_value: &str) -> Option<PlacementBindingStatusResponse> {
        PlacementBindingQuery::lookup_entry(pool, key_value)
    }

    pub async fn recover_entry(
        pool: &str,
        key_value: impl AsRef<str>,
    ) -> Result<PlacementBindingRecoveryResponse, Error> {
        PlacementBindingWorkflow::recover_entry(pool, key_value.as_ref())
            .await
            .map_err(Error::from)
    }

    pub async fn resolve_or_create(
        pool: &str,
        key_value: impl AsRef<str>,
    ) -> Result<PlacementBindingStatusResponse, Error> {
        PlacementBindingWorkflow::resolve_or_create(pool, key_value.as_ref())
            .await
            .map_err(Error::from)
    }

    pub fn bind_instance(
        pool: &str,
        key_value: impl AsRef<str>,
        pid: Principal,
    ) -> Result<(), Error> {
        PlacementBindingWorkflow::bind_instance(pool, key_value.as_ref(), pid).map_err(Error::from)
    }

    #[must_use]
    pub fn registry() -> PlacementBindingRegistryResponse {
        PlacementBindingQuery::registry()
    }
}
