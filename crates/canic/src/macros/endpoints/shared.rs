//! Module: macros::endpoints::shared
//!
//! Responsibility: emit endpoint macros shared by root, non-root, and wasm-store canisters.
//! Does not own: endpoint auth policy, runtime state, metrics storage, or query semantics.
//! Boundary: exposes facade macros that delegate immediately to core APIs.

/// Emit the lifecycle and runtime readiness endpoints shared by all Canic canisters.
#[macro_export]
macro_rules! canic_emit_lifecycle_core_endpoints {
    () => {
        #[$crate::canic_query(public)]
        fn canic_cycle_balance() -> Result<u128, ::canic::Error> {
            Ok($crate::__internal::cdk::api::canister_cycle_balance())
        }

        #[$crate::canic_query(internal, public)]
        fn canic_ready() -> bool {
            $crate::__internal::core::api::ready::ReadyApi::is_ready()
        }

        #[$crate::canic_query(internal, public)]
        fn canic_bootstrap_status() -> ::canic::dto::state::BootstrapStatusResponse {
            $crate::__internal::core::api::ready::ReadyApi::bootstrap_status()
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_fleet_activation_status()
        -> Result<::canic::dto::fleet_activation::FleetActivationStatusResponse, ::canic::Error> {
            $crate::__internal::core::api::fleet_activation::FleetActivationApi::status()
        }
    };
}

/// Emit guarded runtime introspection endpoints shared by all Canic canisters.
#[macro_export]
macro_rules! canic_emit_runtime_introspection_endpoints {
    () => {
        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_health() -> Result<::canic::dto::runtime::CanicHealthStatus, ::canic::Error>
        {
            Ok(
                $crate::__internal::core::api::runtime::RuntimeIntrospectionApi::health(Some(
                    $crate::__internal::cdk::api::time(),
                )),
            )
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_readiness()
        -> Result<::canic::dto::runtime::CanicReadinessStatus, ::canic::Error> {
            Ok(
                $crate::__internal::core::api::runtime::RuntimeIntrospectionApi::readiness(
                    $crate::__internal::cdk::api::time(),
                ),
            )
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_runtime_status()
        -> Result<::canic::dto::runtime::CanicRuntimeStatus, ::canic::Error> {
            Ok(
                $crate::__internal::core::api::runtime::RuntimeIntrospectionApi::runtime_status(
                    $crate::__internal::cdk::api::time(),
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION"),
                    $crate::VERSION,
                    $crate::__internal::cdk::api::canister_version(),
                ),
            )
        }
    };
}

/// Emit the ICRC standards-facing query endpoints shared by all Canic canisters.
#[macro_export]
macro_rules! canic_emit_icrc_standards_endpoints {
    () => {
        #[$crate::canic_query(internal, public)]
        pub fn icrc10_supported_standards() -> Vec<(String, String)> {
            $crate::__internal::core::api::icrc::Icrc10Query::supported_standards()
        }

        #[cfg(canic_icrc21_enabled)]
        #[$crate::canic_query(internal, public)]
        async fn icrc21_canister_call_consent_message(
            req: ::canic::dto::icrc21::ConsentMessageRequest,
        ) -> ::canic::dto::icrc21::ConsentMessageResponse {
            $crate::__internal::core::api::icrc::Icrc21Query::consent_message(req)
        }
    };
}

/// Emit the Canic metadata endpoint shared by all Canic canisters.
#[macro_export]
macro_rules! canic_emit_canic_metadata_endpoints {
    () => {
        #[$crate::canic_query(internal, public)]
        fn canic_metadata() -> ::canic::dto::metadata::CanicMetadataResponse {
            $crate::__internal::core::api::metadata::CanicMetadataApi::metadata_for(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_DESCRIPTION"),
                $crate::VERSION,
                $crate::__internal::cdk::api::canister_version(),
            )
        }
    };
}

/// Emit the default runtime discovery endpoint bundle.
#[macro_export]
macro_rules! canic_bundle_discovery_endpoints {
    () => {
        #[cfg(not(canic_disable_bundle_icrc_standards))]
        $crate::canic_emit_icrc_standards_endpoints!();
        #[cfg(not(canic_disable_bundle_metadata))]
        $crate::canic_emit_canic_metadata_endpoints!();
    };
}

/// Emit the minimal stable-memory ABI ledger recovery diagnostic endpoint.
#[macro_export]
macro_rules! canic_emit_memory_ledger_diagnostic_endpoint {
    () => {
        #[$crate::__internal::cdk::query]
        fn canic_memory_ledger()
        -> Result<::canic::dto::memory::MemoryLedgerResponse, ::canic::Error> {
            let caller = $crate::__internal::cdk::api::msg_caller();
            if !$crate::__internal::cdk::api::is_controller(&caller) {
                return Err(::canic::Error::unauthorized(format!(
                    "caller '{caller}' is not a controller of this canister"
                )));
            }

            $crate::__internal::core::api::memory::MemoryQuery::ledger()
        }
    };
}

/// Emit the environment snapshot diagnostic endpoint shared by all Canic canisters.
#[macro_export]
macro_rules! canic_emit_env_observability_endpoints {
    () => {
        #[$crate::canic_query(public)]
        fn canic_env() -> Result<::canic::dto::env::EnvSnapshotResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::env::EnvQuery::snapshot())
        }
    };
}

/// Emit runtime log diagnostic endpoints shared by all Canic canisters.
#[macro_export]
macro_rules! canic_emit_log_observability_endpoints {
    () => {
        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_log(
            crate_name: Option<String>,
            topic: Option<String>,
            min_level: Option<::canic::__internal::core::log::Level>,
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::log::LogEntry>, ::canic::Error> {
            Ok($crate::__internal::core::api::log::LogQuery::page(
                crate_name, topic, min_level, page,
            ))
        }
    };
}

/// Emit shared observability and operator-facing diagnostic endpoints.
#[macro_export]
macro_rules! canic_bundle_observability_endpoints {
    () => {
        $crate::canic_emit_runtime_introspection_endpoints!();
        #[cfg(not(canic_disable_bundle_observability_env))]
        $crate::canic_emit_env_observability_endpoints!();
        #[cfg(not(canic_disable_bundle_observability_log))]
        $crate::canic_emit_log_observability_endpoints!();
    };
}

/// Emit the metrics query surface shared by all Canic canisters.
#[macro_export]
macro_rules! canic_emit_metrics_endpoints {
    () => {
        #[$crate::canic_query(public)]
        fn canic_metrics(
            kind: ::canic::dto::metrics::MetricsKind,
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::metrics::MetricEntry>, ::canic::Error> {
            match kind {
                #[cfg(canic_metrics_core)]
                ::canic::dto::metrics::MetricsKind::Core => Ok(
                    $crate::__internal::core::api::metrics::MetricsQuery::core(page),
                ),
                #[cfg(canic_metrics_placement)]
                ::canic::dto::metrics::MetricsKind::Placement => {
                    Ok($crate::__internal::core::api::metrics::MetricsQuery::placement(page))
                }
                #[cfg(canic_metrics_platform)]
                ::canic::dto::metrics::MetricsKind::Platform => {
                    Ok($crate::__internal::core::api::metrics::MetricsQuery::platform(page))
                }
                #[cfg(canic_metrics_runtime)]
                ::canic::dto::metrics::MetricsKind::Runtime => {
                    Ok($crate::__internal::core::api::metrics::MetricsQuery::runtime(page))
                }
                #[cfg(canic_metrics_security)]
                ::canic::dto::metrics::MetricsKind::Security => {
                    Ok($crate::__internal::core::api::metrics::MetricsQuery::security(page))
                }
                #[cfg(canic_metrics_storage)]
                ::canic::dto::metrics::MetricsKind::Storage => {
                    Ok($crate::__internal::core::api::metrics::MetricsQuery::storage(page))
                }
                _ => Err(::canic::Error::invalid(
                    "metrics tier is not enabled for this canister",
                )),
            }
        }
    };
}

/// Emit the response-capability and trust-chain runtime endpoint.
#[macro_export]
macro_rules! canic_emit_auth_attestation_endpoints {
    () => {
        #[cfg(canic_is_root)]
        #[$crate::canic_update(internal, requires(caller::is_registered_to_subnet()))]
        async fn canic_response_capability_v1(
            envelope: ::canic::dto::capability::RootCapabilityEnvelopeV1,
        ) -> Result<::canic::dto::capability::RootCapabilityResponseV1, ::canic::Error> {
            $crate::__internal::core::api::rpc::RpcApi::response_capability_v1_root(envelope).await
        }

        #[cfg(not(canic_is_root))]
        #[$crate::canic_update(internal, public)]
        async fn canic_response_capability_v1(
            envelope: ::canic::dto::capability::NonrootCyclesCapabilityEnvelopeV1,
        ) -> Result<::canic::dto::capability::NonrootCyclesCapabilityResponseV1, ::canic::Error> {
            $crate::__internal::core::api::rpc::RpcApi::response_capability_v1_nonroot(envelope)
                .await
        }
    };
}
