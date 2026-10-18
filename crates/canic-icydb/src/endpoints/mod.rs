//! Explicit native endpoint mounting; database semantics remain in the consumer.

/// Declare the complete fixed IcyDB endpoint surface exported by this canister.
///
/// SQL uses `icydb_sql_query(introspection = false)` or `introspection = true`
/// and exports the Wire method `icydb_query`. An optional
/// `authorization = guard(path)` argument must follow `introspection`; the guard
/// has the `icydb::ReadAuthorizationGuard` signature. SQL declarations require `sql`.
///
/// Fixture reset/load declarations require both the consuming canister's
/// `test-admin-api` feature and IcyDB's `sql` feature. Gate those declarations
/// with `#[cfg(feature = "test-admin-api")]` when production omits fixtures.
#[macro_export]
#[expect(
    clippy::crate_in_macro_def,
    reason = "endpoints! must prove crate-root placement in the consuming canister"
)]
macro_rules! endpoints {
    ($($declaration:tt)*) => {
        #[doc(hidden)]
        struct __IcydbEndpointsRootMarker;

        #[doc(hidden)]
        const fn __icydb_endpoints_root_binding(_: __IcydbEndpointsRootMarker) {}

        const _: fn(__IcydbEndpointsRootMarker) = crate::__icydb_endpoints_root_binding;

        #[used]
        static __ICYDB_ENDPOINT_DECLARATIONS: () =
            crate::__icydb_generated::__ICYDB_START_BINDING;

        #[allow(dead_code, reason = "declared endpoint families select controller checks")]
        mod __icydb_endpoint_authorization {
            fn require_controller(
                boundary: crate::__icydb_facade::diagnostic::RuntimeBoundaryCode,
            ) -> Result<(), crate::__icydb_facade::Error> {
                let caller = $crate::__private::cdk::api::msg_caller();
                if !$crate::__private::cdk::api::is_controller(&caller) {
                    return Err(crate::__icydb_facade::Error::from_runtime_boundary(
                        boundary,
                        crate::__icydb_facade::ErrorOrigin::Interface,
                    ));
                }

                Ok(())
            }

            pub(crate) fn require_sql_controller() -> Result<(), crate::__icydb_facade::Error> {
                require_controller(
                    crate::__icydb_facade::diagnostic::RuntimeBoundaryCode::SqlSurfaceControllerRequired,
                )
            }

            pub(crate) fn require_operational_controller() -> Result<(), crate::__icydb_facade::Error> {
                require_controller(
                    crate::__icydb_facade::diagnostic::RuntimeBoundaryCode::OperationalSurfaceControllerRequired,
                )
            }

            pub(crate) fn require_schema_controller() -> Result<(), crate::__icydb_facade::Error> {
                require_controller(
                    crate::__icydb_facade::diagnostic::RuntimeBoundaryCode::SchemaSurfaceControllerRequired,
                )
            }
        }
        $crate::__icydb_endpoints_internal!($($declaration)*);
    };
}

#[doc(hidden)]
#[macro_export]
#[expect(
    clippy::crate_in_macro_def,
    reason = "endpoint wrappers call the consuming canister's private generated module"
)]
macro_rules! __icydb_endpoints_internal {
    () => {};

    ($(#[cfg($($cfg:tt)*)])* icydb_sql_query(
        introspection = false,
        authorization = guard($guard:path) $(,)?
    ); $($rest:tt)*) => {
        $(#[cfg($($cfg)*)])*
        #[used]
        static __ICYDB_ENDPOINT_DECLARATION_QUERY: () = ();
        $(#[cfg($($cfg)*)])*
        crate::__icydb_facade::__icydb_with_sql_endpoint! {
            "icydb_sql_query";
            #[$crate::__private::native_entry(query, name = "icydb_query", sdk = crate::__icydb_host_sdk)]
        fn __icydb_export_icydb_query() {
            #[used]
            #[expect(unsafe_code, reason = "generated method identity metadata belongs to its raw export")]
            #[unsafe(link_section = "icydb:wire")]
            static CONTRACT: [u8; 8] = crate::__icydb_facade::wire::Endpoint::Query.id().to_le_bytes();
            $crate::__icydb_serve!(crate::__icydb_facade::wire::Endpoint::Query, |sql: String| -> Result<__icydb_facade::db::sql::SqlQueryResult, __icydb_facade::Error> {

                let guard: crate::__icydb_facade::ReadAuthorizationGuard = $guard;
                crate::__icydb_facade::__macro::authorize_sql_read(
                    $crate::__private::cdk::api::msg_caller(),
                    guard,
                )?;
                crate::__icydb_facade::db::with_request_execution(|| {
                    crate::__icydb_generated::endpoint_handlers::sql_query::<false>(sql)
                })

            });
        }
        }
        $crate::__icydb_endpoints_internal!($($rest)*);
    };

    ($(#[cfg($($cfg:tt)*)])* icydb_sql_query(
        introspection = true,
        authorization = guard($guard:path) $(,)?
    ); $($rest:tt)*) => {
        $(#[cfg($($cfg)*)])*
        #[used]
        static __ICYDB_ENDPOINT_DECLARATION_QUERY: () = ();
        $(#[cfg($($cfg)*)])*
        crate::__icydb_facade::__icydb_with_sql_endpoint! {
            "icydb_sql_query";
            #[$crate::__private::native_entry(query, name = "icydb_query", sdk = crate::__icydb_host_sdk)]
        fn __icydb_export_icydb_query() {
            #[used]
            #[expect(unsafe_code, reason = "generated method identity metadata belongs to its raw export")]
            #[unsafe(link_section = "icydb:wire")]
            static CONTRACT: [u8; 8] = crate::__icydb_facade::wire::Endpoint::Query.id().to_le_bytes();
            $crate::__icydb_serve!(crate::__icydb_facade::wire::Endpoint::Query, |sql: String| -> Result<__icydb_facade::db::sql::SqlQueryResult, __icydb_facade::Error> {

                let guard: crate::__icydb_facade::ReadAuthorizationGuard = $guard;
                crate::__icydb_facade::__macro::authorize_sql_read(
                    $crate::__private::cdk::api::msg_caller(),
                    guard,
                )?;
                crate::__icydb_facade::db::with_request_execution(|| {
                    crate::__icydb_generated::endpoint_handlers::sql_query::<true>(sql)
                })

            });
        }
        }
        $crate::__icydb_endpoints_internal!($($rest)*);
    };

    ($(#[cfg($($cfg:tt)*)])* icydb_sql_query(introspection = false); $($rest:tt)*) => {
        $(#[cfg($($cfg)*)])*
        #[used]
        static __ICYDB_ENDPOINT_DECLARATION_QUERY: () = ();
        $(#[cfg($($cfg)*)])*
        crate::__icydb_facade::__icydb_with_sql_endpoint! {
            "icydb_sql_query";
            #[$crate::__private::native_entry(query, name = "icydb_query", sdk = crate::__icydb_host_sdk)]
        fn __icydb_export_icydb_query() {
            #[used]
            #[expect(unsafe_code, reason = "generated method identity metadata belongs to its raw export")]
            #[unsafe(link_section = "icydb:wire")]
            static CONTRACT: [u8; 8] = crate::__icydb_facade::wire::Endpoint::Query.id().to_le_bytes();
            $crate::__icydb_serve!(crate::__icydb_facade::wire::Endpoint::Query, |sql: String| -> Result<__icydb_facade::db::sql::SqlQueryResult, __icydb_facade::Error> {

                crate::__icydb_endpoint_authorization::require_sql_controller()?;
                crate::__icydb_facade::db::with_request_execution(|| {
                    crate::__icydb_generated::endpoint_handlers::sql_query::<false>(sql)
                })

            });
        }
        }
        $crate::__icydb_endpoints_internal!($($rest)*);
    };

    ($(#[cfg($($cfg:tt)*)])* icydb_sql_query(introspection = true); $($rest:tt)*) => {
        $(#[cfg($($cfg)*)])*
        #[used]
        static __ICYDB_ENDPOINT_DECLARATION_QUERY: () = ();
        $(#[cfg($($cfg)*)])*
        crate::__icydb_facade::__icydb_with_sql_endpoint! {
            "icydb_sql_query";
            #[$crate::__private::native_entry(query, name = "icydb_query", sdk = crate::__icydb_host_sdk)]
        fn __icydb_export_icydb_query() {
            #[used]
            #[expect(unsafe_code, reason = "generated method identity metadata belongs to its raw export")]
            #[unsafe(link_section = "icydb:wire")]
            static CONTRACT: [u8; 8] = crate::__icydb_facade::wire::Endpoint::Query.id().to_le_bytes();
            $crate::__icydb_serve!(crate::__icydb_facade::wire::Endpoint::Query, |sql: String| -> Result<__icydb_facade::db::sql::SqlQueryResult, __icydb_facade::Error> {

                crate::__icydb_endpoint_authorization::require_sql_controller()?;
                crate::__icydb_facade::db::with_request_execution(|| {
                    crate::__icydb_generated::endpoint_handlers::sql_query::<true>(sql)
                })

            });
        }
        }
        $crate::__icydb_endpoints_internal!($($rest)*);
    };

    ($(#[cfg($($cfg:tt)*)])* icydb_ddl; $($rest:tt)*) => {
        $(#[cfg($($cfg)*)])*
        #[used]
        static __ICYDB_ENDPOINT_DECLARATION_DDL: () = ();
        $(#[cfg($($cfg)*)])*
        crate::__icydb_facade::__icydb_with_sql_endpoint! {
            "icydb_ddl";
            #[$crate::__private::native_entry(update, name = "icydb_ddl", sdk = crate::__icydb_host_sdk)]
        fn __icydb_export_icydb_ddl() {
            #[used]
            #[expect(unsafe_code, reason = "generated method identity metadata belongs to its raw export")]
            #[unsafe(link_section = "icydb:wire")]
            static CONTRACT: [u8; 8] = crate::__icydb_facade::wire::Endpoint::Ddl.id().to_le_bytes();
            $crate::__icydb_serve!(crate::__icydb_facade::wire::Endpoint::Ddl, |sql: String| -> Result<__icydb_facade::db::sql::SqlQueryResult, __icydb_facade::Error> {

                crate::__icydb_endpoint_authorization::require_sql_controller()?;
                crate::__icydb_facade::db::with_request_execution(|| {
                    crate::__icydb_generated::endpoint_handlers::sql_ddl(sql)
                })

            });
        }
        }
        $crate::__icydb_endpoints_internal!($($rest)*);
    };

    ($(#[cfg($($cfg:tt)*)])* icydb_update(admission = primary_key_only); $($rest:tt)*) => {
        $(#[cfg($($cfg)*)])*
        #[used]
        static __ICYDB_ENDPOINT_DECLARATION_UPDATE: () = ();
        $(#[cfg($($cfg)*)])*
        crate::__icydb_facade::__icydb_with_sql_endpoint! {
            "icydb_update";
            #[$crate::__private::native_entry(update, name = "icydb_update", sdk = crate::__icydb_host_sdk)]
        fn __icydb_export_icydb_update() {
            #[used]
            #[expect(unsafe_code, reason = "generated method identity metadata belongs to its raw export")]
            #[unsafe(link_section = "icydb:wire")]
            static CONTRACT: [u8; 8] = crate::__icydb_facade::wire::Endpoint::Update.id().to_le_bytes();
            $crate::__icydb_serve!(crate::__icydb_facade::wire::Endpoint::Update, |sql: String| -> Result<__icydb_facade::db::sql::SqlQueryResult, __icydb_facade::Error> {

                crate::__icydb_endpoint_authorization::require_sql_controller()?;
                crate::__icydb_facade::db::with_request_execution(|| {
                    crate::__icydb_generated::endpoint_handlers::sql_update_primary_key(sql)
                })

            });
        }
        }
        $crate::__icydb_endpoints_internal!($($rest)*);
    };

    ($(#[cfg($($cfg:tt)*)])* icydb_update(admission = bounded_deterministic); $($rest:tt)*) => {
        $(#[cfg($($cfg)*)])*
        #[used]
        static __ICYDB_ENDPOINT_DECLARATION_UPDATE: () = ();
        $(#[cfg($($cfg)*)])*
        crate::__icydb_facade::__icydb_with_sql_endpoint! {
            "icydb_update";
            #[$crate::__private::native_entry(update, name = "icydb_update", sdk = crate::__icydb_host_sdk)]
        fn __icydb_export_icydb_update() {
            #[used]
            #[expect(unsafe_code, reason = "generated method identity metadata belongs to its raw export")]
            #[unsafe(link_section = "icydb:wire")]
            static CONTRACT: [u8; 8] = crate::__icydb_facade::wire::Endpoint::Update.id().to_le_bytes();
            $crate::__icydb_serve!(crate::__icydb_facade::wire::Endpoint::Update, |sql: String| -> Result<__icydb_facade::db::sql::SqlQueryResult, __icydb_facade::Error> {

                crate::__icydb_endpoint_authorization::require_sql_controller()?;
                crate::__icydb_facade::db::with_request_execution(|| {
                    crate::__icydb_generated::endpoint_handlers::sql_update_bounded(sql)
                })

            });
        }
        }
        $crate::__icydb_endpoints_internal!($($rest)*);
    };

    ($(#[cfg($($cfg:tt)*)])* icydb_integrity; $($rest:tt)*) => {
        $(#[cfg($($cfg)*)])*
        #[used]
        static __ICYDB_ENDPOINT_DECLARATION_INTEGRITY: () = ();
        $(#[cfg($($cfg)*)])*
        crate::__icydb_facade::__icydb_with_sql_endpoint! {
            "icydb_integrity";
            #[allow(
                clippy::result_large_err,
                reason = "generated integrity endpoints preserve the public typed error contract"
            )]
            #[$crate::__private::native_entry(update, name = "icydb_integrity", sdk = crate::__icydb_host_sdk)]
        fn __icydb_export_icydb_integrity() {
            #[used]
            #[expect(unsafe_code, reason = "generated method identity metadata belongs to its raw export")]
            #[unsafe(link_section = "icydb:wire")]
            static CONTRACT: [u8; 8] = crate::__icydb_facade::wire::Endpoint::Integrity.id().to_le_bytes();
            $crate::__icydb_serve!(crate::__icydb_facade::wire::Endpoint::Integrity, |sql: String| -> Result<__icydb_facade::db::IntegrityCheckResult, __icydb_facade::db::SqlIntegrityError> {

                crate::__icydb_endpoint_authorization::require_sql_controller()
                    .map_err(__icydb_facade::db::SqlIntegrityError::Sql)?;
                let caller = $crate::__private::cdk::api::msg_caller();
                let owner = __icydb_facade::db::IntegrityJobOwner::new(caller.to_text())
                    .map_err(|error| __icydb_facade::db::SqlIntegrityError::Integrity(
                        __icydb_facade::db::IntegrityCheckError::Job(error)))?;
                crate::__icydb_facade::db::with_request_execution(|| {
                    crate::__icydb_generated::endpoint_handlers::sql_integrity(sql, owner)
                })

            });
        }
        }
        $crate::__icydb_endpoints_internal!($($rest)*);
    };

    ($(#[cfg($($cfg:tt)*)])* icydb_fixtures_reset; $($rest:tt)*) => {
        $(#[cfg($($cfg)*)])*
        #[used]
        static __ICYDB_ENDPOINT_DECLARATION_FIXTURES_RESET: () = ();
        $(#[cfg($($cfg)*)])*
        #[cfg(not(feature = "test-admin-api"))]
        compile_error!("endpoint declaration `icydb_fixtures_reset` requires the canister `test-admin-api` Cargo feature");
        $(#[cfg($($cfg)*)])*
        #[cfg(feature = "test-admin-api")]
        crate::__icydb_facade::__icydb_with_sql_endpoint! {
            "icydb_fixtures_reset";
            #[$crate::__private::native_entry(update, name = "icydb_fixtures_reset", sdk = crate::__icydb_host_sdk)]
        fn __icydb_export_icydb_fixtures_reset() {
            #[used]
            #[expect(unsafe_code, reason = "generated method identity metadata belongs to its raw export")]
            #[unsafe(link_section = "icydb:wire")]
            static CONTRACT: [u8; 8] = crate::__icydb_facade::wire::Endpoint::FixturesReset.id().to_le_bytes();
            $crate::__icydb_serve!(crate::__icydb_facade::wire::Endpoint::FixturesReset, |_: ()| -> Result<(), __icydb_facade::Error> {

                crate::__icydb_endpoint_authorization::require_sql_controller()?;
                crate::__icydb_facade::db::with_request_execution(|| {
                    crate::__icydb_generated::endpoint_handlers::fixtures_reset()
                })

            });
        }
        }
        $crate::__icydb_endpoints_internal!($($rest)*);
    };

    ($(#[cfg($($cfg:tt)*)])* icydb_fixtures_load(handler = $handler:path); $($rest:tt)*) => {
        $(#[cfg($($cfg)*)])*
        #[used]
        static __ICYDB_ENDPOINT_DECLARATION_FIXTURES_LOAD: () = ();
        $(#[cfg($($cfg)*)])*
        #[cfg(not(feature = "test-admin-api"))]
        compile_error!("endpoint declaration `icydb_fixtures_load` requires the canister `test-admin-api` Cargo feature");
        $(#[cfg($($cfg)*)])*
        #[cfg(feature = "test-admin-api")]
        crate::__icydb_facade::__icydb_with_sql_endpoint! {
            "icydb_fixtures_load";
            #[$crate::__private::native_entry(update, name = "icydb_fixtures_load", sdk = crate::__icydb_host_sdk)]
        fn __icydb_export_icydb_fixtures_load() {
            #[used]
            #[expect(unsafe_code, reason = "generated method identity metadata belongs to its raw export")]
            #[unsafe(link_section = "icydb:wire")]
            static CONTRACT: [u8; 8] = crate::__icydb_facade::wire::Endpoint::FixturesLoad.id().to_le_bytes();
            $crate::__icydb_serve!(crate::__icydb_facade::wire::Endpoint::FixturesLoad, |_: ()| -> Result<(), __icydb_facade::Error> {

                crate::__icydb_endpoint_authorization::require_sql_controller()?;
                let handler: fn() -> Result<(), crate::__icydb_facade::Error> = $handler;
                crate::__icydb_facade::db::with_request_execution(|| {
                    crate::__icydb_generated::endpoint_handlers::fixtures_load(handler)
                })

            });
        }
        }
        $crate::__icydb_endpoints_internal!($($rest)*);
    };

    ($(#[cfg($($cfg:tt)*)])* icydb_metrics(authorization = public); $($rest:tt)*) => {
        $(#[cfg($($cfg)*)])*
        #[used]
        static __ICYDB_ENDPOINT_DECLARATION_METRICS: () = ();
        $(#[cfg($($cfg)*)])*
        crate::__icydb_facade::__icydb_with_metrics_endpoint! {
            "icydb_metrics";
            #[$crate::__private::native_entry(query, name = "icydb_metrics", sdk = crate::__icydb_host_sdk)]
        fn __icydb_export_icydb_metrics() {
            #[used]
            #[expect(unsafe_code, reason = "generated method identity metadata belongs to its raw export")]
            #[unsafe(link_section = "icydb:wire")]
            static CONTRACT: [u8; 8] = crate::__icydb_facade::wire::Endpoint::Metrics.id().to_le_bytes();
            $crate::__icydb_serve!(crate::__icydb_facade::wire::Endpoint::Metrics, |_: ()| -> Result<__icydb_facade::metrics::MetricsReport, __icydb_facade::Error> {

                crate::__icydb_generated::endpoint_handlers::metrics()

            });
        }
        }
        $crate::__icydb_endpoints_internal!($($rest)*);
    };

    ($(#[cfg($($cfg:tt)*)])* icydb_metrics(authorization = controller); $($rest:tt)*) => {
        $(#[cfg($($cfg)*)])*
        #[used]
        static __ICYDB_ENDPOINT_DECLARATION_METRICS: () = ();
        $(#[cfg($($cfg)*)])*
        crate::__icydb_facade::__icydb_with_metrics_endpoint! {
            "icydb_metrics";
            #[$crate::__private::native_entry(query, name = "icydb_metrics", sdk = crate::__icydb_host_sdk)]
        fn __icydb_export_icydb_metrics() {
            #[used]
            #[expect(unsafe_code, reason = "generated method identity metadata belongs to its raw export")]
            #[unsafe(link_section = "icydb:wire")]
            static CONTRACT: [u8; 8] = crate::__icydb_facade::wire::Endpoint::Metrics.id().to_le_bytes();
            $crate::__icydb_serve!(crate::__icydb_facade::wire::Endpoint::Metrics, |_: ()| -> Result<__icydb_facade::metrics::MetricsReport, __icydb_facade::Error> {

                crate::__icydb_endpoint_authorization::require_operational_controller()?;
                crate::__icydb_generated::endpoint_handlers::metrics()

            });
        }
        }
        $crate::__icydb_endpoints_internal!($($rest)*);
    };

    ($(#[cfg($($cfg:tt)*)])* icydb_metrics_reset; $($rest:tt)*) => {
        $(#[cfg($($cfg)*)])*
        #[used]
        static __ICYDB_ENDPOINT_DECLARATION_METRICS_RESET: () = ();
        $(#[cfg($($cfg)*)])*
        crate::__icydb_facade::__icydb_with_metrics_endpoint! {
            "icydb_metrics_reset";
            #[$crate::__private::native_entry(update, name = "icydb_metrics_reset", sdk = crate::__icydb_host_sdk)]
        fn __icydb_export_icydb_metrics_reset() {
            #[used]
            #[expect(unsafe_code, reason = "generated method identity metadata belongs to its raw export")]
            #[unsafe(link_section = "icydb:wire")]
            static CONTRACT: [u8; 8] = crate::__icydb_facade::wire::Endpoint::MetricsReset.id().to_le_bytes();
            $crate::__icydb_serve!(crate::__icydb_facade::wire::Endpoint::MetricsReset, |_: ()| -> Result<(), __icydb_facade::Error> {

                crate::__icydb_endpoint_authorization::require_operational_controller()?;
                crate::__icydb_generated::endpoint_handlers::metrics_reset()

            });
        }
        }
        $crate::__icydb_endpoints_internal!($($rest)*);
    };

    ($(#[cfg($($cfg:tt)*)])* icydb_snapshot; $($rest:tt)*) => {
        $(#[cfg($($cfg)*)])*
        #[used]
        static __ICYDB_ENDPOINT_DECLARATION_SNAPSHOT: () = ();
        $(#[cfg($($cfg)*)])*
        #[$crate::__private::native_entry(query, name = "icydb_snapshot", sdk = crate::__icydb_host_sdk)]
        fn __icydb_export_icydb_snapshot() {
            #[used]
            #[expect(unsafe_code, reason = "generated method identity metadata belongs to its raw export")]
            #[unsafe(link_section = "icydb:wire")]
            static CONTRACT: [u8; 8] = crate::__icydb_facade::wire::Endpoint::Snapshot.id().to_le_bytes();
            $crate::__icydb_serve!(crate::__icydb_facade::wire::Endpoint::Snapshot, |_: ()| -> Result<__icydb_facade::db::StorageReport, __icydb_facade::Error> {

            crate::__icydb_endpoint_authorization::require_operational_controller()?;
            crate::__icydb_facade::db::with_request_execution(|| {
                crate::__icydb_generated::endpoint_handlers::snapshot()
            })

            });
        }
        $crate::__icydb_endpoints_internal!($($rest)*);
    };

    ($(#[cfg($($cfg:tt)*)])* icydb_schema(authorization = public); $($rest:tt)*) => {
        $(#[cfg($($cfg)*)])*
        #[used]
        static __ICYDB_ENDPOINT_DECLARATION_SCHEMA: () = ();
        $(#[cfg($($cfg)*)])*
        #[$crate::__private::native_entry(query, name = "icydb_schema", sdk = crate::__icydb_host_sdk)]
        fn __icydb_export_icydb_schema() {
            #[used]
            #[expect(unsafe_code, reason = "generated method identity metadata belongs to its raw export")]
            #[unsafe(link_section = "icydb:wire")]
            static CONTRACT: [u8; 8] = crate::__icydb_facade::wire::Endpoint::Schema.id().to_le_bytes();
            $crate::__icydb_serve!(crate::__icydb_facade::wire::Endpoint::Schema, |_: ()| -> Result<Vec<__icydb_facade::db::EntitySchemaDescription>, __icydb_facade::Error> {

            crate::__icydb_facade::db::with_request_execution(|| {
                crate::__icydb_generated::endpoint_handlers::schema()
            })

            });
        }
        $crate::__icydb_endpoints_internal!($($rest)*);
    };

    ($(#[cfg($($cfg:tt)*)])* icydb_schema(
        authorization = guard($guard:path) $(,)?
    ); $($rest:tt)*) => {
        $(#[cfg($($cfg)*)])*
        #[used]
        static __ICYDB_ENDPOINT_DECLARATION_SCHEMA: () = ();
        $(#[cfg($($cfg)*)])*
        #[$crate::__private::native_entry(query, name = "icydb_schema", sdk = crate::__icydb_host_sdk)]
        fn __icydb_export_icydb_schema() {
            #[used]
            #[expect(unsafe_code, reason = "generated method identity metadata belongs to its raw export")]
            #[unsafe(link_section = "icydb:wire")]
            static CONTRACT: [u8; 8] = crate::__icydb_facade::wire::Endpoint::Schema.id().to_le_bytes();
            $crate::__icydb_serve!(crate::__icydb_facade::wire::Endpoint::Schema, |_: ()| -> Result<Vec<__icydb_facade::db::EntitySchemaDescription>, __icydb_facade::Error> {

            let guard: crate::__icydb_facade::ReadAuthorizationGuard = $guard;
            crate::__icydb_facade::__macro::authorize_schema_read(
                $crate::__private::cdk::api::msg_caller(),
                guard,
            )?;
            crate::__icydb_facade::db::with_request_execution(|| {
                crate::__icydb_generated::endpoint_handlers::schema()
            })

            });
        }
        $crate::__icydb_endpoints_internal!($($rest)*);
    };

    ($(#[cfg($($cfg:tt)*)])* icydb_schema(authorization = controller); $($rest:tt)*) => {
        $(#[cfg($($cfg)*)])*
        #[used]
        static __ICYDB_ENDPOINT_DECLARATION_SCHEMA: () = ();
        $(#[cfg($($cfg)*)])*
        #[$crate::__private::native_entry(query, name = "icydb_schema", sdk = crate::__icydb_host_sdk)]
        fn __icydb_export_icydb_schema() {
            #[used]
            #[expect(unsafe_code, reason = "generated method identity metadata belongs to its raw export")]
            #[unsafe(link_section = "icydb:wire")]
            static CONTRACT: [u8; 8] = crate::__icydb_facade::wire::Endpoint::Schema.id().to_le_bytes();
            $crate::__icydb_serve!(crate::__icydb_facade::wire::Endpoint::Schema, |_: ()| -> Result<Vec<__icydb_facade::db::EntitySchemaDescription>, __icydb_facade::Error> {

            crate::__icydb_endpoint_authorization::require_schema_controller()?;
            crate::__icydb_facade::db::with_request_execution(|| {
                crate::__icydb_generated::endpoint_handlers::schema()
            })

            });
        }
        $crate::__icydb_endpoints_internal!($($rest)*);
    };

    ($(#[cfg($($cfg:tt)*)])* icydb_schema_migrate; $($rest:tt)*) => {
        $(#[cfg($($cfg)*)])*
        #[used]
        static __ICYDB_ENDPOINT_DECLARATION_SCHEMA_MIGRATE: () = ();
        $(#[cfg($($cfg)*)])*
        crate::__icydb_facade::__icydb_with_migration_endpoint! {
            "icydb_schema_migrate";
            #[$crate::__private::native_entry(update, name = "icydb_schema_migrate", sdk = crate::__icydb_host_sdk)]
        fn __icydb_export_icydb_schema_migrate() {
            #[used]
            #[expect(unsafe_code, reason = "generated method identity metadata belongs to its raw export")]
            #[unsafe(link_section = "icydb:wire")]
            static CONTRACT: [u8; 8] = crate::__icydb_facade::wire::Endpoint::SchemaMigrate.id().to_le_bytes();
            $crate::__icydb_serve!(crate::__icydb_facade::wire::Endpoint::SchemaMigrate, |command: __icydb_facade::db::SchemaMigrationCommand| -> Result<__icydb_facade::db::SchemaMigrationStatusPage, __icydb_facade::Error> {

                crate::__icydb_endpoint_authorization::require_operational_controller()?;
                crate::__icydb_facade::db::with_request_execution(|| {
                    crate::__icydb_generated::endpoint_handlers::schema_migrate(command)
                })

            });
        }
        }
        $crate::__icydb_endpoints_internal!($($rest)*);
    };

    ($(#[cfg($($cfg:tt)*)])* icydb_schema_migration; $($rest:tt)*) => {
        $(#[cfg($($cfg)*)])*
        #[used]
        static __ICYDB_ENDPOINT_DECLARATION_SCHEMA_MIGRATION: () = ();
        $(#[cfg($($cfg)*)])*
        crate::__icydb_facade::__icydb_with_migration_endpoint! {
            "icydb_schema_migration";
            #[$crate::__private::native_entry(query, name = "icydb_schema_migration", sdk = crate::__icydb_host_sdk)]
        fn __icydb_export_icydb_schema_migration() {
            #[used]
            #[expect(unsafe_code, reason = "generated method identity metadata belongs to its raw export")]
            #[unsafe(link_section = "icydb:wire")]
            static CONTRACT: [u8; 8] = crate::__icydb_facade::wire::Endpoint::SchemaMigration.id().to_le_bytes();
            $crate::__icydb_serve!(crate::__icydb_facade::wire::Endpoint::SchemaMigration, |request: __icydb_facade::db::SchemaMigrationStatusRequest| -> Result<__icydb_facade::db::SchemaMigrationStatusPage, __icydb_facade::Error> {

                crate::__icydb_endpoint_authorization::require_operational_controller()?;
                crate::__icydb_facade::db::with_request_execution(|| {
                    crate::__icydb_generated::endpoint_handlers::schema_migration(&request)
                })

            });
        }
        }
        $crate::__icydb_endpoints_internal!($($rest)*);
    };

    ($(#[cfg($($cfg:tt)*)])* $endpoint:ident $($rest:tt)*) => {
        compile_error!(concat!("unknown or invalid IcyDB endpoint declaration `", stringify!($endpoint), "`"));
    };

    (#[$attribute:meta] $($rest:tt)*) => {
        compile_error!("IcyDB endpoint declarations accept only `#[cfg(...)]` attributes");
    };

    ($($invalid:tt)+) => {
        compile_error!("invalid IcyDB endpoint declaration syntax");
    };
}

#[doc(hidden)]
#[macro_export]
#[expect(
    clippy::crate_in_macro_def,
    reason = "the host uses its consumer's native contract"
)]
macro_rules! __icydb_serve {
    ($endpoint:expr, $handler:expr $(,)?) => {{
        let endpoint = $endpoint;
        $crate::serve_raw(
            crate::__icydb_facade::wire::MAX_REQUEST_BYTES,
            |bytes| endpoint.decode_request(bytes),
            |response| endpoint.encode_response(response),
            $handler,
        );
    }};
}
