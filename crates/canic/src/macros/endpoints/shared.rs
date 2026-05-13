// -----------------------------------------------------------------------------
// Shared endpoint emitters
// -----------------------------------------------------------------------------

// Leaf emitter for the lifecycle/runtime core shared by all Canic canisters.
#[macro_export]
macro_rules! canic_emit_lifecycle_core_endpoints {
    () => {
        #[$crate::canic_query]
        fn canic_cycle_balance() -> Result<u128, ::canic::Error> {
            Ok($crate::cdk::api::canister_cycle_balance())
        }

        #[$crate::canic_query(internal)]
        fn canic_ready() -> bool {
            $crate::__internal::core::api::ready::ReadyApi::is_ready()
        }

        #[$crate::canic_query(internal)]
        fn canic_bootstrap_status() -> ::canic::dto::state::BootstrapStatusResponse {
            $crate::__internal::core::api::ready::ReadyApi::bootstrap_status()
        }
    };
}

// Leaf emitter for the ICRC standards-facing surface shared by all Canic canisters.
#[macro_export]
macro_rules! canic_emit_icrc_standards_endpoints {
    () => {
        #[$crate::canic_query(internal)]
        pub fn icrc10_supported_standards() -> Vec<(String, String)> {
            $crate::__internal::core::api::icrc::Icrc10Query::supported_standards()
        }

        #[cfg(canic_icrc21_enabled)]
        #[$crate::canic_query(internal)]
        async fn icrc21_canister_call_consent_message(
            req: ::canic::__internal::core::cdk::spec::standards::icrc::icrc21::ConsentMessageRequest,
        ) -> ::canic::__internal::core::cdk::spec::standards::icrc::icrc21::ConsentMessageResponse {
            $crate::__internal::core::api::icrc::Icrc21Query::consent_message(req)
        }
    };
}

// Leaf emitter for Canic metadata shared by all Canic canisters.
#[macro_export]
macro_rules! canic_emit_canic_metadata_endpoints {
    () => {
        #[$crate::canic_query(internal)]
        fn canic_metadata() -> ::canic::dto::metadata::CanicMetadataResponse {
            $crate::__internal::core::api::metadata::CanicMetadataApi::metadata_for(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_DESCRIPTION"),
                $crate::VERSION,
                $crate::cdk::api::canister_version(),
            )
        }
    };
}

// Bundle composer for the discovery surface preserved by the default runtime.
#[macro_export]
macro_rules! canic_bundle_discovery_endpoints {
    () => {
        #[cfg(not(canic_disable_bundle_icrc_standards))]
        $crate::canic_emit_icrc_standards_endpoints!();
        #[cfg(not(canic_disable_bundle_metadata))]
        $crate::canic_emit_canic_metadata_endpoints!();
    };
}

// Leaf emitter for runtime memory-registry diagnostics shared by all Canic canisters.
#[macro_export]
macro_rules! canic_emit_memory_observability_endpoints {
    () => {
        #[$crate::canic_query]
        fn canic_memory_registry()
        -> Result<::canic::dto::memory::MemoryRegistryResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::memory::MemoryQuery::registry())
        }
    };
}

// Leaf emitter for environment snapshot diagnostics shared by all Canic canisters.
#[macro_export]
macro_rules! canic_emit_env_observability_endpoints {
    () => {
        #[$crate::canic_query]
        fn canic_env() -> Result<::canic::dto::env::EnvSnapshotResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::env::EnvQuery::snapshot())
        }
    };
}

// Leaf emitter for runtime log diagnostics shared by all Canic canisters.
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

// Bundle composer for shared observability and operator-facing diagnostics.
#[macro_export]
macro_rules! canic_bundle_observability_endpoints {
    () => {
        #[cfg(not(canic_disable_bundle_observability_memory))]
        $crate::canic_emit_memory_observability_endpoints!();
        #[cfg(not(canic_disable_bundle_observability_env))]
        $crate::canic_emit_env_observability_endpoints!();
        #[cfg(not(canic_disable_bundle_observability_log))]
        $crate::canic_emit_log_observability_endpoints!();
    };
}

// Leaf emitter for the metrics query surface shared by all Canic canisters.
#[macro_export]
macro_rules! canic_emit_metrics_endpoints {
    () => {
        #[$crate::canic_query]
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

// Leaf emitter for the response-capability and trust-chain runtime.
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
        #[$crate::canic_update(internal)]
        async fn canic_response_capability_v1(
            envelope: ::canic::dto::capability::NonrootCyclesCapabilityEnvelopeV1,
        ) -> Result<::canic::dto::capability::NonrootCyclesCapabilityResponseV1, ::canic::Error> {
            $crate::__internal::core::api::rpc::RpcApi::response_capability_v1_nonroot(envelope)
                .await
        }
    };
}
