use crate::{
    cdk::types::Principal,
    dto::{
        error::Error,
        placement::directory::{DirectoryEntryStatusResponse, DirectoryRegistryResponse},
    },
    workflow::placement::directory::{DirectoryWorkflow, query::DirectoryQuery},
};

///
/// DirectoryApi
///

pub struct DirectoryApi;

impl DirectoryApi {
    #[must_use]
    pub fn lookup_key(pool: &str, key_value: &str) -> Option<Principal> {
        DirectoryQuery::lookup_key(pool, key_value)
    }

    #[must_use]
    pub fn lookup_entry(pool: &str, key_value: &str) -> Option<DirectoryEntryStatusResponse> {
        DirectoryQuery::lookup_entry(pool, key_value)
    }

    pub async fn resolve_or_create(
        pool: &str,
        key_value: impl AsRef<str>,
    ) -> Result<DirectoryEntryStatusResponse, Error> {
        DirectoryWorkflow::resolve_or_create(pool, key_value.as_ref())
            .await
            .map_err(Error::from)
    }

    pub fn bind_instance(
        pool: &str,
        key_value: impl AsRef<str>,
        pid: Principal,
    ) -> Result<(), Error> {
        DirectoryWorkflow::bind_instance(pool, key_value.as_ref(), pid).map_err(Error::from)
    }

    #[must_use]
    pub fn registry() -> DirectoryRegistryResponse {
        DirectoryQuery::registry()
    }
}
