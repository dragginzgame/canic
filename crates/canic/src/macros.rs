//! Facade macros for downstream canister crates.

// -----------------------------------------------------------------------------
// Build macros
// -----------------------------------------------------------------------------

/// Embed the shared Canic configuration into a canister crate's build script.
///
/// Reads the provided TOML file (relative to the crate manifest dir), validates it
/// using [`Config`](crate::core::config::Config), and sets
/// `CANIC_CONFIG_PATH` for later use by `include_str!`. Canister crates typically
/// invoke this from `build.rs`.
#[macro_export]
macro_rules! build {
    ($file:expr) => {{
        $crate::__canic_build_internal! {
            $file,
            |cfg_str, cfg_path, cfg| {
                let _ = (&cfg_str, &cfg_path, &cfg);
            }
        }
    }};
}

/// Embed the shared configuration for the root orchestrator canister.
///
/// Performs the same validation as [`macro@build`].
#[macro_export]
macro_rules! build_root {
    ($file:expr) => {{
        $crate::__canic_build_internal! {
            $file,
            |_cfg_str, _cfg_path, _cfg| {}
        }
    }};
}

/// Internal helper shared by [`macro@build`] and [`macro@build_root`].
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_build_internal {
    ($file:expr, |$cfg_str:ident, $cfg_path:ident, $cfg:ident| $body:block) => {{
        let manifest_dir =
            std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
        let $cfg_path = std::path::PathBuf::from(manifest_dir).join($file);
        println!("cargo:rerun-if-changed={}", $cfg_path.display());
        if let Some(parent) = $cfg_path.parent() {
            println!("cargo:rerun-if-changed={}", parent.display());
        }

        let $cfg_str = match std::fs::read_to_string(&$cfg_path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                panic!("Missing Canic config at {}", $cfg_path.display())
            }
            Err(e) => panic!("Failed to read {}: {}", $cfg_path.display(), e),
        };

        // Init Config
        let $cfg = $crate::core::init_config(&$cfg_str).expect("invalid canic config");

        // Run the extra body (per-canister or nothing)
        $body

        let abs = $cfg_path.canonicalize().expect("canonicalize canic config path");
        println!("cargo:rustc-env=CANIC_CONFIG_PATH={}", abs.display());
        println!("cargo:rerun-if-changed={}", abs.display());
    }};
}

// -----------------------------------------------------------------------------
// Start macros
// -----------------------------------------------------------------------------

/// Configure lifecycle hooks for **non-root** Canic canisters.
///
/// This macro defines the IC-required `init` and `post_upgrade` entry points
/// at the crate root and *immediately delegates* all real work to runtime
/// bootstrap code.
///
/// IMPORTANT:
/// - This macro must remain **thin**
/// - It must not perform orchestration
/// - It must not contain async logic
/// - It must not encode policy
/// - It may schedule async hooks via timers, but must never await them
///
/// Its sole responsibility is to bridge IC lifecycle hooks to runtime code.
#[macro_export]
macro_rules! start {
    ($canister_role:expr) => {
        #[::canic::cdk::init]
        fn init(payload: ::canic::core::dto::abi::v1::CanisterInitPayload, args: Option<Vec<u8>>) {
            // Load embedded configuration early.
            $crate::__canic_load_config!();

            // Delegate to lifecycle adapter (NOT workflow).
            $crate::core::api::lifecycle::LifecycleApi::init_nonroot_canister(
                $canister_role,
                payload,
                args.clone(),
            );

            // ---- userland lifecycle hooks (scheduled last) ----
            $crate::core::api::timer::TimerApi::set_lifecycle_timer(
                ::std::time::Duration::ZERO,
                "canic:user:init",
                async move {
                    canic_setup().await;
                    canic_install(args).await;
                },
            );
        }

        #[::canic::cdk::post_upgrade]
        fn post_upgrade() {
            // Reload embedded configuration on upgrade.
            $crate::__canic_load_config!();

            // Delegate to lifecycle adapter.
            $crate::core::api::lifecycle::LifecycleApi::post_upgrade_nonroot_canister(
                $canister_role,
            );

            // ---- userland lifecycle hooks (scheduled last) ----
            $crate::core::api::timer::TimerApi::set_lifecycle_timer(
                ::core::time::Duration::ZERO,
                "canic:user:init",
                async move {
                    canic_setup().await;
                    canic_upgrade().await;
                },
            );
        }

        $crate::canic_endpoints!();
        $crate::canic_endpoints_nonroot!();
    };
}

/// Configure lifecycle hooks for the **root orchestrator** canister.
///
/// This macro behaves like [`start!`], but delegates to root-specific
/// bootstrap logic.
///
/// IMPORTANT:
/// - The macro does NOT perform root orchestration
/// - The macro does NOT import WASMs
/// - The macro does NOT create canisters
/// - The macro may schedule async hooks via timers, but must never await them
///
/// All root-specific behavior lives in `workflow::bootstrap`.
#[macro_export]
macro_rules! start_root {
    () => {
        #[::canic::cdk::init]
        fn init(identity: ::canic::core::dto::subnet::SubnetIdentity) {
            // Load embedded configuration early.
            $crate::__canic_load_config!();

            // Delegate to lifecycle adapter.
            $crate::core::api::lifecycle::LifecycleApi::init_root_canister(identity);

            // ---- userland lifecycle hooks (scheduled last) ----
            $crate::core::api::timer::TimerApi::set_lifecycle_timer(
                ::core::time::Duration::ZERO,
                "canic:user:init",
                async move {
                    canic_setup().await;
                    canic_install().await;
                },
            );
        }

        #[::canic::cdk::post_upgrade]
        fn post_upgrade() {
            // Reload embedded configuration on upgrade.
            $crate::__canic_load_config!();

            // Delegate to lifecycle adapter.
            $crate::core::api::lifecycle::LifecycleApi::post_upgrade_root_canister();

            // ---- userland lifecycle hooks (scheduled last) ----
            $crate::core::api::timer::TimerApi::set_lifecycle_timer(
                ::core::time::Duration::ZERO,
                "canic:user:init",
                async move {
                    canic_setup().await;
                    canic_upgrade().await;
                },
            );
        }

        $crate::canic_endpoints!();
        $crate::canic_endpoints_root!();
    };
}

//
// Private helpers
//

///
/// Load the embedded configuration during init and upgrade hooks.
///
/// This macro exists solely to embed and load the TOML configuration file
/// at compile time (`CANIC_CONFIG_PATH`). It is used internally by
/// [`macro@canic::start`] and [`macro@canic::start_root`].

#[doc(hidden)]
#[macro_export]
macro_rules! __canic_load_config {
    () => {{
        let config_str = include_str!(env!("CANIC_CONFIG_PATH"));
        if let Err(err) = $crate::core::init_config(config_str) {
            $crate::cdk::println!(
                "[canic] FATAL: config init failed (CANIC_CONFIG_PATH={}): {err}",
                env!("CANIC_CONFIG_PATH")
            );

            let msg = format!(
                "canic init failed: config init failed (CANIC_CONFIG_PATH={}): {err}",
                env!("CANIC_CONFIG_PATH")
            );

            $crate::cdk::api::trap(&msg);
        }
    }};
}

// -----------------------------------------------------------------------------
// Endpoint bundle macros
// -----------------------------------------------------------------------------

// Macros that generate public IC endpoints for Canic canisters.

// Expose the shared query and update handlers used by all Canic canisters.
#[macro_export]
macro_rules! canic_endpoints {
    () => {
        // NOTE: Avoid `$crate` in endpoint signatures (args/returns); Candid rejects it.
        //
        // IC API ENDPOINTS (IMPORTANT!!)
        // these are specific endpoints defined by the IC spec
        //

        // ic_cycles_accept
        #[canic_update]
        fn ic_cycles_accept(max_amount: u128) -> u128 {
            $crate::cdk::api::msg_cycles_accept(max_amount)
        }

        //
        // ICRC ENDPOINTS
        //

        #[canic_query]
        pub fn icrc10_supported_standards() -> Vec<(String, String)> {
            $crate::core::api::icrc::Icrc10Query::supported_standards()
        }

        #[canic_query]
        async fn icrc21_canister_call_consent_message(
            req: ::canic::core::cdk::spec::standards::icrc::icrc21::ConsentMessageRequest,
        ) -> ::canic::core::cdk::spec::standards::icrc::icrc21::ConsentMessageResponse {
            $crate::core::api::icrc::Icrc21Query::consent_message(req)
        }

        //
        // CANISTER HELPERS
        //

        #[canic_query]
        fn canic_canister_cycle_balance() -> u128 {
            $crate::cdk::api::canister_cycle_balance()
        }

        #[canic_query]
        fn canic_canister_version() -> u64 {
            $crate::cdk::api::canister_version()
        }

        #[canic_query]
        fn canic_time() -> u64 {
            $crate::cdk::api::time()
        }

        //
        // MEMORY
        //

        #[canic_query]
        fn canic_memory_registry() -> ::canic::core::dto::memory::MemoryRegistryView {
            $crate::core::api::memory::MemoryQuery::registry_view()
        }

        #[canic_query]
        fn canic_env() -> ::canic::core::dto::env::EnvView {
            $crate::core::api::env::EnvQuery::view()
        }

        #[canic_query]
        fn canic_log(
            crate_name: Option<String>,
            topic: Option<String>,
            min_level: Option<::canic::core::log::Level>,
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::log::LogEntryView> {
            $crate::core::api::log::LogQuery::page(crate_name, topic, min_level, page)
        }

        //
        // METRICS
        //

        #[canic_query]
        fn canic_metrics_system() -> Vec<::canic::core::dto::metrics::SystemMetricEntry> {
            $crate::core::api::metrics::MetricsQuery::system_snapshot()
        }

        #[canic_query]
        fn canic_metrics_icc(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::metrics::IccMetricEntry> {
            $crate::core::api::metrics::MetricsQuery::icc_page(page)
        }

        #[canic_query]
        fn canic_metrics_http(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::metrics::HttpMetricEntry> {
            $crate::core::api::metrics::MetricsQuery::http_page(page)
        }

        #[canic_query]
        fn canic_metrics_timer(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::metrics::TimerMetricEntry> {
            $crate::core::api::metrics::MetricsQuery::timer_page(page)
        }

        #[canic_query]
        fn canic_metrics_access(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::metrics::AccessMetricEntry> {
            $crate::core::api::metrics::MetricsQuery::access_page(page)
        }

        // metrics, but lives in the perf module
        #[canic_query]
        fn canic_metrics_perf(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::perf::PerfEntry> {
            $crate::core::api::metrics::MetricsQuery::perf_page(page)
        }

        // derived_view
        #[canic_query]
        fn canic_metrics_endpoint_health(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::metrics::EndpointHealthView> {
            $crate::core::api::metrics::MetricsQuery::endpoint_health_page(
                page,
                Some($crate::core::protocol::CANIC_METRICS_ENDPOINT_HEALTH),
            )
        }

        //
        // STATE
        //

        #[canic_query]
        fn canic_app_state() -> ::canic::core::dto::state::AppStateView {
            $crate::core::api::state::AppStateQuery::view()
        }

        #[canic_query]
        fn canic_subnet_state() -> ::canic::core::dto::state::SubnetStateView {
            $crate::core::api::state::SubnetStateQuery::view()
        }

        //
        // DIRECTORY VIEWS
        //

        #[canic_query]
        fn canic_app_directory(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::topology::DirectoryEntryView> {
            $crate::core::api::topology::directory::AppDirectoryApi::page(page)
        }

        #[canic_query]
        fn canic_subnet_directory(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::topology::DirectoryEntryView> {
            $crate::core::api::topology::directory::SubnetDirectoryApi::page(page)
        }

        //
        // TOPOLOGY
        //

        #[canic_query]
        fn canic_canister_children(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::canister::CanisterRecordView> {
            $crate::core::api::topology::children::CanisterChildrenApi::page(page)
        }

        //
        // CYCLES
        //

        #[canic_query]
        fn canic_cycle_tracker(
            page: ::canic::core::dto::page::PageRequest,
        ) -> ::canic::core::dto::page::Page<::canic::core::dto::cycles::CycleTrackerEntryView> {
            $crate::core::api::cycles::CycleTrackerQuery::page(page)
        }

        //
        // SCALING
        //

        #[canic_query(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_scaling_registry()
        -> Result<::canic::core::dto::placement::scaling::ScalingRegistryView, ::canic::Error> {
            Ok($crate::core::api::placement::scaling::ScalingApi::registry_view())
        }

        //
        // SHARDING
        //

        #[canic_query(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_sharding_registry()
        -> Result<::canic::core::dto::placement::sharding::ShardingRegistryView, ::canic::Error> {
            Ok($crate::core::api::placement::sharding::ShardingApi::registry_view())
        }

        #[canic_query(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_sharding_tenants(
            pool: String,
            shard_pid: ::canic::core::cdk::types::Principal,
        ) -> Result<::canic::core::dto::placement::sharding::ShardingTenantsView, ::canic::Error> {
            Ok($crate::core::api::placement::sharding::ShardingApi::tenants_view(&pool, shard_pid))
        }

        //
        // ICTS
        // extra endpoints for each canister as per rem.codes
        //
        // NOTE: ICTS return types are fixed by a third-party standard; do not change them.

        #[canic_query]
        fn icts_name() -> String {
            $crate::core::api::icts::IctsApi::name()
        }

        #[canic_query]
        fn icts_version() -> String {
            $crate::core::api::icts::IctsApi::version()
        }

        #[canic_query]
        fn icts_description() -> String {
            $crate::core::api::icts::IctsApi::description()
        }

        #[canic_query]
        fn icts_metadata() -> ::canic::core::dto::icts::CanisterMetadataView {
            $crate::core::api::icts::IctsApi::metadata()
        }

        /// ICTS add-on endpoint: returns string errors by design.
        #[canic_update]
        async fn icts_canister_status()
        -> Result<::canic::core::dto::canister::CanisterStatusView, String> {
            use $crate::cdk::api::msg_caller;

            static ICTS_CALLER: ::std::sync::LazyLock<::candid::Principal> =
                ::std::sync::LazyLock::new(|| {
                    ::candid::Principal::from_text("ylse7-raaaa-aaaal-qsrsa-cai")
                        .expect("ICTS caller principal must be valid")
                });

            if msg_caller() != *ICTS_CALLER {
                return Err("unauthorized".to_string());
            }

            $crate::core::api::icts::IctsApi::canister_status()
                .await
                .map_err(|err| err.to_string())
        }
    };
}

// Generate the endpoint surface for the root orchestrator canister.
#[macro_export]
macro_rules! canic_endpoints_root {
    () => {
        // canic_app
        // root-only app-level state mutation endpoint
        #[canic_update(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_app(
            cmd: ::canic::core::dto::state::AppCommand,
        ) -> Result<(), ::canic::Error> {
            $crate::core::api::state::AppStateApi::execute_command(cmd).await
        }

        // canic_canister_upgrade
        #[canic_update(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_canister_upgrade(
            canister_pid: ::candid::Principal,
        ) -> Result<::canic::core::dto::rpc::UpgradeCanisterResponse, ::canic::Error> {
            let res =
                $crate::core::api::rpc::RpcApi::upgrade_canister_request(canister_pid).await?;

            Ok(res)
        }

        // canic_response
        // root's way to respond to a generic request from another canister
        // has to come from a direct child canister
        #[canic_update(auth_any(::canic::core::access::auth::is_registered_to_subnet))]
        async fn canic_response(
            request: ::canic::core::dto::rpc::Request,
        ) -> Result<::canic::core::dto::rpc::Response, ::canic::Error> {
            let response = $crate::core::api::rpc::RpcApi::response(request).await?;

            Ok(response)
        }

        // canic_canister_status
        // this can be called via root as root is the master controller
        #[canic_update(auth_any(
            ::canic::core::access::auth::is_root,
            ::canic::core::access::auth::is_controller
        ))]
        async fn canic_canister_status(
            pid: ::canic::cdk::candid::Principal,
        ) -> Result<::canic::core::dto::canister::CanisterStatusView, ::canic::Error> {
            $crate::core::api::ic::mgmt::MgmtApi::canister_status(pid).await
        }

        //
        // CONFIG
        //

        #[canic_query(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_config() -> Result<String, ::canic::Error> {
            $crate::core::api::config::ConfigApi::export_toml()
        }

        //
        // REGISTRIES
        //

        #[canic_query]
        fn canic_app_registry() -> ::canic::core::dto::topology::AppRegistryView {
            $crate::core::api::topology::registry::AppRegistryApi::view()
        }

        #[canic_query]
        fn canic_subnet_registry() -> ::canic::core::dto::topology::SubnetRegistryView {
            $crate::core::api::topology::registry::SubnetRegistryApi::view()
        }

        //
        // CANISTER POOL
        //

        #[canic_query]
        async fn canic_pool_list() -> ::canic::core::dto::pool::CanisterPoolView {
            $crate::core::api::pool::CanisterPoolApi::list_view()
        }

        #[canic_update(auth_any(::canic::core::access::auth::is_controller))]
        async fn canic_pool_admin(
            cmd: ::canic::core::dto::pool::PoolAdminCommand,
        ) -> Result<::canic::core::dto::pool::PoolAdminResponse, ::canic::Error> {
            $crate::core::api::pool::CanisterPoolApi::admin(cmd).await
        }
    };
}

// Generate the endpoint surface for non-root canisters.
#[macro_export]
macro_rules! canic_endpoints_nonroot {
    () => {
        //
        // SYNC
        //

        #[canic_update(auth_any(::canic::core::access::auth::is_parent))]
        async fn canic_sync_state(
            snapshot: ::canic::core::dto::cascade::StateSnapshotView,
        ) -> Result<(), ::canic::Error> {
            $crate::core::api::cascade::CascadeApi::sync_state(snapshot).await
        }

        #[canic_update(auth_any(::canic::core::access::auth::is_parent))]
        async fn canic_sync_topology(
            snapshot: ::canic::core::dto::cascade::TopologySnapshotView,
        ) -> Result<(), ::canic::Error> {
            $crate::core::api::cascade::CascadeApi::sync_topology(snapshot).await
        }
    };
}

// -----------------------------------------------------------------------------
// Timer macros
// -----------------------------------------------------------------------------

// Perf-instrumented timer helpers that auto-label with module + function name.
//
// These macros wrap `TimerApi` so callers can schedule work without manually
// threading labels. Labels are constructed as `module_path!()::function_name`.

///
/// timer
/// Schedule a one-shot timer with an auto-generated label.
///
/// # Examples
/// - `timer!(Duration::from_secs(5), do_cleanup);`
/// - `timer!(Duration::ZERO, my_task, arg1, arg2);`
///
#[macro_export]
macro_rules! timer {
    ($delay:expr, $func:path $(, $($args:tt)*)? ) => {{
        let label = concat!(module_path!(), "::", stringify!($func));
        $crate::core::api::timer::TimerApi::set_lifecycle_timer(
            $delay,
            label,
            $func($($($args)*)?),
        )
    }};
}

///
/// timer_guarded
/// Schedule a one-shot timer if none is already scheduled for the slot.
/// Returns true when a new timer was scheduled.
///
/// # Examples
/// - `timer_guarded!(MY_TIMER, Duration::from_secs(5), do_cleanup);`
/// - `timer_guarded!(MY_TIMER, Duration::ZERO, my_task, arg1, arg2);`
///
#[macro_export]
macro_rules! timer_guarded {
    ($slot:path, $delay:expr, $func:path $(, $($args:tt)*)? ) => {{
        let label = concat!(module_path!(), "::", stringify!($func));
        $crate::core::api::timer::TimerApi::set_guarded(
            &$slot,
            $delay,
            label,
            $func($($($args)*)?),
        )
    }};
}

///
/// timer_interval
/// Schedule a repeating timer with an auto-generated label.
///
/// # Examples
/// - `timer_interval!(Duration::from_secs(60), heartbeat);`
/// - `timer_interval!(Duration::from_secs(10), tick, state.clone());`
///
#[macro_export]
macro_rules! timer_interval {
    ($interval:expr, $func:path $(, $($args:tt)*)? ) => {{
        let label = concat!(module_path!(), "::", stringify!($func));
        $crate::core::api::timer::TimerApi::set_interval(
            $interval,
            label,
            move || $func($($($args)*)?),
        )
    }};
}

///
/// timer_interval_guarded
/// Schedule an init timer that installs a repeating timer for the slot.
/// Returns true when a new timer was scheduled.
///
/// # Examples
/// - `timer_interval_guarded!(MY_TIMER, Duration::ZERO, init_task; Duration::from_secs(60), tick);`
/// - `timer_interval_guarded!(MY_TIMER, Duration::from_secs(2), init; Duration::from_secs(10), tick, state.clone());`
///
#[macro_export]
macro_rules! timer_interval_guarded {
    (
        $slot:path,
        $init_delay:expr,
        $init_func:path $(, $($init_args:tt)*)?
        ;
        $interval:expr,
        $tick_func:path $(, $($tick_args:tt)*)?
    ) => {{
        let init_label = concat!(module_path!(), "::", stringify!($init_func));
        let tick_label = concat!(module_path!(), "::", stringify!($tick_func));

        $crate::core::api::timer::TimerApi::set_guarded_interval(
            &$slot,
            $init_delay,
            init_label,
            move || $init_func($($($init_args)*)?),
            $interval,
            tick_label,
            move || $tick_func($($($tick_args)*)?),
        )
    }};
}

// -----------------------------------------------------------------------------
// Log macro
// -----------------------------------------------------------------------------

/// Log a runtime entry using Canic's structured logger.
#[macro_export]
macro_rules! log {
    ($($tt:tt)*) => {{
        $crate::core::log!($($tt)*);
    }};
}

// -----------------------------------------------------------------------------
// Perf macro
// -----------------------------------------------------------------------------

/// Log elapsed instruction counts since the last `perf!` invocation in this thread.
///
/// - Uses a thread-local `PERF_LAST` snapshot.
/// - Computes `delta = now - last`.
/// - Prints a human-readable line for debugging.
///
/// Intended usage:
/// - Long-running maintenance tasks where you want *checkpoints* in a single call.
///
/// Note: `perf!` is independent of endpoint perf scopes and does not touch the
/// endpoint stack used by dispatch.
///
/// Notes:
/// - On non-wasm targets, `perf_counter()` returns 0, so this becomes a no-op-ish
///   counter (still records 0 deltas); this keeps unit tests compiling cleanly.
#[macro_export]
macro_rules! perf {
    ($($label:tt)*) => {{
        $crate::core::perf::PERF_LAST.with(|last| {
            // Use the wrapper so non-wasm builds compile.
            let now = $crate::core::perf::perf_counter();
            let then = *last.borrow();
            let delta = now.saturating_sub(then);

            // Update last checkpoint.
            *last.borrow_mut() = now;

            // Format label + pretty-print counters.
            let label = format!($($label)*);
            let delta_fmt = $crate::utils::instructions::format_instructions(delta);
            let now_fmt = $crate::utils::instructions::format_instructions(now);

            // ❌ NO structured recording here
            // ✔️ Debug log only
            $crate::core::log!(
                Info,
                Topic::Perf,
                "{}: '{}' used {}i since last (total: {}i)",
                module_path!(),
                label,
                delta_fmt,
                now_fmt
            );
        });
    }};
}

// -----------------------------------------------------------------------------
// Auth macros
// -----------------------------------------------------------------------------

/// Enforce that every supplied rule future succeeds for the current caller.
///
/// This is a convenience wrapper around `require_all`, allowing guard
/// checks to stay in expression position within async endpoints.
#[macro_export]
macro_rules! auth_require_all {
    ($($f:expr),* $(,)?) => {{
        $crate::core::access::auth::require_all(vec![
            $( Box::new(move |caller| Box::pin($f(caller))) ),*
        ]).await
    }};
}

/// Enforce that at least one supplied rule future succeeds for the current
/// caller.
///
/// See [`auth_require_all!`] for details on accepted rule shapes.
#[macro_export]
macro_rules! auth_require_any {
    ($($f:expr),* $(,)?) => {{
        $crate::core::access::auth::require_any(vec![
            $( Box::new(move |caller| Box::pin($f(caller))) ),*
        ]).await
    }};
}
