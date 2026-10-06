//! Facade macros for downstream canister crates.
mod build;
mod endpoints;
mod start;

// -----------------------------------------------------------------------------
// Application scope macro
// -----------------------------------------------------------------------------

/// Construct one statically validated canonical application scope.
///
/// ```
/// const READ: canic::access::auth::ApplicationScopeRef<'static> =
///     canic::application_scope!("my_app:read");
/// assert_eq!(READ.as_str(), "my_app:read");
/// ```
///
/// ```compile_fail
/// const INVALID: canic::access::auth::ApplicationScopeRef<'static> =
///     canic::application_scope!("my.app:read");
/// ```
#[macro_export]
macro_rules! application_scope {
    ($scope:literal) => {{ const { $crate::access::auth::ApplicationScopeRef::from_static($scope) } }};
}

// -----------------------------------------------------------------------------
// Log macro
// -----------------------------------------------------------------------------

/// Log a runtime entry using Canic's structured logger.
#[macro_export]
macro_rules! log {
    ($($tt:tt)*) => {{
        $crate::__internal::core::log!($($tt)*);
    }};
}

// -----------------------------------------------------------------------------
// Perf macro
// -----------------------------------------------------------------------------

/// Record and log instructions since the preceding checkpoint in this invocation.
///
/// Generated endpoints own their checkpoint baseline across awaits. Background
/// futures can use `canic::api::ops::with_async_perf_context`. Unscoped calls
/// produce no sample because they have no attributable interval.
/// Checkpoint keys remain `module_path!()` plus the formatted label. Native
/// counters measure zero while retaining sample counts inside an owned scope.
///
/// ```
/// canic::perf!("load_state");
/// canic::prelude::perf!("loaded {} rows", 3);
/// canic::api::ops::perf!("finish");
/// ```
#[macro_export]
macro_rules! perf {
    ($($label:tt)*) => {{
        let label = format!($($label)*);
        if let Some(sample) = $crate::__internal::core::perf::checkpoint(module_path!(), &label) {
            let delta_fmt = $crate::__internal::instructions::format_instructions(sample.instructions);
            let now_fmt = $crate::__internal::instructions::format_instructions(sample.call_context_instructions);
            $crate::__internal::core::log!(
                $crate::__internal::core::log::Topic::Perf,
                Info,
                "{}: '{}' used {}i since last (total: {}i)",
                module_path!(), label, delta_fmt, now_fmt
            );
        }
    }};
}

#[cfg(test)]
mod tests {
    use crate::__internal::core::perf::{PerfKey, entries};

    #[test]
    fn public_perf_paths_record_formatted_checkpoints() {
        let call = crate::__internal::core::ids::EndpointCall {
            endpoint: crate::__internal::core::ids::EndpointId::new("perf_macro_probe"),
            kind: crate::__internal::core::ids::EndpointCallKind::Query,
        };
        crate::__internal::core::dispatch::measure_endpoint(call, || {
            crate::perf!("loaded {} rows", 3);
            crate::prelude::perf!("loaded {} rows", 3);
            crate::api::ops::perf!("loaded {} rows", 3);
        });

        let checkpoint = entries()
            .into_iter()
            .find(|entry| {
                matches!(
                    &entry.key,
                    PerfKey::Checkpoint { scope, label }
                        if scope == module_path!() && label == "loaded 3 rows"
                )
            })
            .expect("public perf macros record one shared checkpoint");
        assert_eq!(checkpoint.count, 3);
        assert_eq!(checkpoint.total_instructions, 0);
    }
}
