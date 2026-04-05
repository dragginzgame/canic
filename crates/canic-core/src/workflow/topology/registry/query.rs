use crate::{
    dto::topology::{AppRegistryResponse, SubnetRegistryResponse},
    ops::storage::registry::{
        app::AppRegistryOps, mapper::AppRegistryResponseMapper, subnet::SubnetRegistryOps,
    },
};

///
/// AppRegistryQuery
///

pub struct AppRegistryQuery;

impl AppRegistryQuery {
    pub fn registry() -> AppRegistryResponse {
        let data = AppRegistryOps::data();

        AppRegistryResponseMapper::record_to_view(data)
    }
}

///
/// SubnetRegistryQuery
///

pub struct SubnetRegistryQuery;

impl SubnetRegistryQuery {
    pub fn registry() -> SubnetRegistryResponse {
        SubnetRegistryOps::response()
    }
}
