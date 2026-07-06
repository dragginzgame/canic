///
/// Structured public runtime API surface.
///
/// This module groups Canic’s runtime capabilities by intent (auth, calls,
/// canister topology, observability, scheduling) rather than mirroring internal
/// core layout.
///

/// Authentication workflow helpers
pub mod auth {
    pub use crate::__internal::core::api::auth::AuthApi;
}

/// Blob-storage protocol helpers.
#[cfg(feature = "blob-storage")]
pub mod blob_storage {
    pub use crate::__internal::core::api::blob_storage::BlobStorageApi;
}

/// ICP-to-cycles refill endpoint helpers.
#[cfg(feature = "icp-refill")]
pub mod icp_refill {
    pub use crate::__internal::core::api::icp_refill::IcpRefillApi;
}

/// Environment queries
pub mod env {
    pub use crate::__internal::core::api::env::EnvQuery;
}

/// IC primitives (calls, HTTP, crypto, network, system APIs)
pub mod ic {
    pub use crate::__internal::core::api::ic::{
        Call, CallBuilder, CallResult, IntentKey, IntentReservation,
    };

    pub mod http {
        pub use crate::__internal::core::api::ic::http::HttpApi;
    }

    pub mod network {
        pub use crate::__internal::core::api::ic::network::NetworkApi;
    }
}

/// Canister lifecycle, placement, and topology
pub mod canister {
    pub use crate::__internal::core::ids::CanisterRole;

    pub mod children {
        pub use crate::__internal::core::api::topology::children::CanisterChildrenApi;
    }

    pub mod index {
        pub use crate::__internal::core::api::topology::index::{AppIndexApi, SubnetIndexApi};
    }

    pub mod registry {
        pub use crate::__internal::core::api::topology::registry::{
            AppRegistryApi, SubnetRegistryApi,
        };
    }

    pub mod placement {
        pub use crate::__internal::core::api::placement::directory::DirectoryApi;
        pub use crate::__internal::core::api::placement::scaling::ScalingApi;

        #[cfg(feature = "sharding")]
        pub use crate::__internal::core::api::placement::sharding::ShardingApi;
    }

    #[cfg(any(feature = "control-plane", feature = "wasm-store-canister"))]
    pub mod template {
        #[cfg(feature = "wasm-store-canister")]
        pub use canic_control_plane::api::template::WasmStoreCanisterApi;
        #[cfg(feature = "control-plane")]
        pub use canic_control_plane::api::template::{
            WasmStoreBootstrapApi, WasmStorePublicationApi,
        };
    }
}

/// RPC abstractions (non-IC-specific)
pub mod rpc {
    pub use crate::__internal::core::api::rpc::RpcApi;
}

/// Runtime bootstrap helpers
pub mod runtime {
    pub use crate::__internal::core::api::runtime::MemoryRuntimeApi;
}

/// Protocol runtime helpers
pub mod protocol {
    pub mod icrc21 {
        pub use crate::__internal::core::dispatch::icrc21::Icrc21Dispatcher;
    }
}

/// Observability and operational helpers
pub mod metrics {
    pub use crate::__internal::core::api::metrics::MetricsQuery;
}

/// Low-level operational helpers
pub mod ops {
    pub use crate::__internal::core::{log, perf};
}

/// Timers and scheduling helpers
pub mod timer {
    pub use crate::{timer, timer_interval};
}
