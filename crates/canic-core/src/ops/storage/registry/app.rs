use crate::{
    ops::prelude::Principal,
    storage::stable::registry::app::{AppRegistry, AppRegistryRecord},
};

///
/// AppRegistryOps
///

pub struct AppRegistryOps;

impl AppRegistryOps {
    // -------------------------------------------------------------
    // Mutation
    // -------------------------------------------------------------

    /// Record the root canister serving one subnet.
    pub fn upsert(subnet_pid: Principal, root_pid: Principal) {
        AppRegistry::upsert(subnet_pid, root_pid);
    }

    // -------------------------------------------------------------
    // Canonical data access
    // -------------------------------------------------------------

    #[must_use]
    pub fn data() -> AppRegistryRecord {
        AppRegistry::export()
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    // Build a deterministic principal for stable registry assertions.
    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    // Ensure app registry bindings are persisted through the ops API.
    #[test]
    fn upsert_records_subnet_root_binding() {
        let subnet_pid = p(201);
        let root_pid = p(202);

        AppRegistryOps::upsert(subnet_pid, root_pid);

        assert!(
            AppRegistryOps::data()
                .entries
                .contains(&(subnet_pid, root_pid))
        );
    }
}
