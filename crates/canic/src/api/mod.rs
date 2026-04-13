///
/// Structured public runtime API surface.
///
/// This module groups Canic’s runtime capabilities by intent (auth, calls,
/// canister topology, observability, scheduling) rather than mirroring internal
/// core layout.
///

/// Delegation workflow helpers
pub mod auth {
    pub use crate::__internal::core::api::auth::DelegationApi;
}

/// Environment queries
pub mod env {
    pub use crate::__internal::core::api::env::EnvQuery;
}

/// IC primitives (calls, HTTP, crypto, network, system APIs)
pub mod ic {
    pub use crate::__internal::core::api::ic::call::{
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
        pub use crate::__internal::core::api::placement::scaling::ScalingApi;

        #[cfg(feature = "sharding")]
        pub use crate::__internal::core::api::placement::sharding::ShardingApi;
    }

    #[cfg(feature = "control-plane")]
    pub mod template {
        pub use canic_control_plane::api::template::WasmStoreApi as EmbeddedTemplateApi;
        pub use canic_control_plane::api::template::{
            WasmStoreApi, WasmStoreBootstrapApi, WasmStoreCanisterApi, WasmStorePublicationApi,
        };
    }
}

/// RPC abstractions (non-IC-specific)
pub mod rpc {
    pub use crate::__internal::core::api::rpc::RpcApi;
}

/// Protocol runtime helpers
pub mod protocol {
    pub mod icrc21 {
        pub use crate::__internal::core::dispatch::icrc21::Icrc21Dispatcher;
    }
}

/// Observability and operational helpers
pub mod ops {
    pub use crate::__internal::core::{log, perf};
}

/// Timers and scheduling helpers
pub mod timer {
    pub use crate::{timer, timer_interval};
}
