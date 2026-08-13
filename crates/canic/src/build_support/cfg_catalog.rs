/// Custom cfg names consumed by Canic's exported macros.
///
/// Both Canic's own build script and downstream `canic::build!` expansions
/// register this exact catalog with rustc.
pub const CANIC_CUSTOM_CFG_NAMES: &[&str] = &[
    "canic_delegated_token_issuer",
    "canic_disable_bundle_metrics",
    "canic_disable_bundle_observability_env",
    "canic_export_candid",
    "canic_has_scaling",
    "canic_has_sharding",
    "canic_icrc21_enabled",
    "canic_is_root",
    "canic_memory_ledger_enabled",
    "canic_metrics_core",
    "canic_metrics_placement",
    "canic_metrics_platform",
    "canic_metrics_runtime",
    "canic_metrics_security",
    "canic_metrics_storage",
];
