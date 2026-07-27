use crate::{
    cdk::types::Principal,
    dto::{
        error::Error,
        placement::index::{
            PlacementIndexRecoveryResponse, PlacementIndexRegistryResponse,
            PlacementIndexStatusResponse,
        },
    },
    workflow::placement::index::{PlacementIndexWorkflow, query::PlacementIndexQuery},
};

///
/// PlacementIndexApi
///

pub struct PlacementIndexApi;

impl PlacementIndexApi {
    #[must_use]
    pub fn lookup_key(pool: &str, key_value: &str) -> Option<Principal> {
        PlacementIndexQuery::lookup_key(pool, key_value)
    }

    #[must_use]
    pub fn lookup_entry(pool: &str, key_value: &str) -> Option<PlacementIndexStatusResponse> {
        PlacementIndexQuery::lookup_entry(pool, key_value)
    }

    pub async fn recover_entry(
        pool: &str,
        key_value: impl AsRef<str>,
    ) -> Result<PlacementIndexRecoveryResponse, Error> {
        PlacementIndexWorkflow::recover_entry(pool, key_value.as_ref())
            .await
            .map_err(Error::from)
    }

    pub async fn resolve_or_create(
        pool: &str,
        key_value: impl AsRef<str>,
    ) -> Result<PlacementIndexStatusResponse, Error> {
        PlacementIndexWorkflow::resolve_or_create(pool, key_value.as_ref())
            .await
            .map_err(Error::from)
    }

    pub fn bind_instance(
        pool: &str,
        key_value: impl AsRef<str>,
        pid: Principal,
    ) -> Result<(), Error> {
        PlacementIndexWorkflow::bind_instance(pool, key_value.as_ref(), pid).map_err(Error::from)
    }

    #[must_use]
    pub fn registry() -> PlacementIndexRegistryResponse {
        PlacementIndexQuery::registry()
    }
}
