use crate::{
    ops::prelude::*,
    storage::stable::registry::app::{AppRegistry, AppRegistryData},
};

///
/// AppRegistrySnapshot
/// Internal, operational snapshot of the app registry.
///

#[derive(Clone, Debug)]
pub struct AppRegistrySnapshot {
    pub entries: Vec<(Principal, Principal)>,
}

impl From<AppRegistryData> for AppRegistrySnapshot {
    fn from(data: AppRegistryData) -> Self {
        Self {
            entries: data.entries,
        }
    }
}

impl From<AppRegistrySnapshot> for AppRegistryData {
    fn from(snapshot: AppRegistrySnapshot) -> Self {
        Self {
            entries: snapshot.entries,
        }
    }
}

///
/// AppRegistryOps
///

pub struct AppRegistryOps;

impl AppRegistryOps {
    #[must_use]
    pub fn snapshot() -> AppRegistrySnapshot {
        AppRegistry::export().into()
    }
}
