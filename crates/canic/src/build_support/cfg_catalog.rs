/// Custom cfg names consumed by Canic's exported macros.
///
/// Both Canic's own build script and downstream `canic::build!` expansions
/// register this exact catalog with rustc.
pub const CANIC_CUSTOM_CFG_NAMES: &[&str] = &[
    "canic_capability_automatic_topup",
    "canic_capability_child_provisioning",
    "canic_capability_delegated_token_issuer",
    "canic_capability_delegated_token_verifier",
    "canic_capability_fleet_admission_projection",
    "canic_capability_fleet_coordinator",
    "canic_capability_icrc21",
    "canic_capability_index",
    "canic_capability_local_application_authorization",
    "canic_capability_role_attestation_signer",
    "canic_capability_role_attestation_verifier",
    "canic_capability_root",
    "canic_capability_root_control_plane",
    "canic_capability_runtime",
    "canic_capability_scaling",
    "canic_capability_sharding",
    "canic_capability_wasm_store",
    "canic_export_candid",
    "canic_icrc21_enabled",
    "canic_is_root",
    "canic_metrics_core",
    "canic_metrics_placement",
    "canic_metrics_platform",
    "canic_metrics_runtime",
    "canic_metrics_security",
    "canic_metrics_storage",
];
