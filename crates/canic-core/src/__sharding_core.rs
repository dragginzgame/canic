pub mod config {
    pub mod schema {
        pub use crate::config::schema::{ShardPool, ShardPoolPolicy, ShardingConfig};
    }
}

pub mod ops {
    pub mod config {
        use crate::{config::schema::ShardingConfig, error::InternalError};

        pub fn current_sharding_config() -> Result<Option<ShardingConfig>, InternalError> {
            Ok(crate::ops::config::ConfigOps::current_canister()?.sharding)
        }
    }

    pub mod ic {
        pub use crate::ops::ic::IcOps;
    }

    pub mod rpc {
        pub mod request {
            pub use crate::ops::rpc::request::{CreateCanisterParent, RequestOps};
        }
    }

    pub mod storage {
        pub mod children {
            pub use crate::ops::storage::children::CanisterChildrenOps;
        }

        pub mod placement {
            pub mod sharding {
                pub use crate::ops::storage::placement::sharding::ShardingRegistryOps;
            }

            pub mod sharding_lifecycle {
                pub use crate::ops::storage::placement::sharding_lifecycle::ShardingLifecycleOps;
            }
        }
    }
}

pub mod storage {
    pub mod stable {
        pub mod sharding {
            pub use crate::storage::stable::sharding::{ShardEntryRecord, ShardKey};
        }
    }
}
