// -----------------------------------------------------------------------------
// Build macros
// -----------------------------------------------------------------------------

/// Embed the shared Canic configuration into a canister crate's build script.
///
/// Reads the provided TOML file (relative to the crate manifest dir), validates it
/// using the shared config schema, and emits both a compact source copy and a
/// generated Rust config model for runtime bootstrap. Canister crates typically
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

/// Internal helper shared by Canic build macros.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_build_internal {
    ($file:expr, |$cfg_str:ident, $cfg_path:ident, $cfg:ident| $body:block) => {{
        let manifest_dir =
            std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
        let __canic_role_name =
            $crate::__build::required_package_role(std::path::Path::new(&manifest_dir));
        let default_cfg_path = std::path::PathBuf::from(&manifest_dir).join($file);
        let env_cfg = std::env::var("CANIC_CONFIG_PATH").ok();
        let mut $cfg_path = env_cfg.as_ref().map_or(default_cfg_path, |value| {
            let path = std::path::PathBuf::from(value);
            if path.is_relative() {
                std::path::PathBuf::from(&manifest_dir).join(path)
            } else {
                path
            }
        });
        println!("cargo:rerun-if-changed={}", $cfg_path.display());
        println!("cargo:rerun-if-env-changed=ICP_ENVIRONMENT");
        println!("cargo:rerun-if-env-changed=CANIC_CONFIG_PATH");
        println!("cargo:rerun-if-env-changed=CANIC_INTERNAL_TEST_ENDPOINTS");

        let __canic_default_role = (__canic_role_name != "root").then(|| __canic_role_name.clone());

        let ($cfg_str, generated_default_config) =
            $crate::__build::read_config_source_or_default(
                &$cfg_path,
                env_cfg.is_some(),
                __canic_default_role.as_deref(),
            );

        if generated_default_config {
            let out_dir =
                std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR must be set"));
            let generated_cfg_path = out_dir.join("canic.default.toml");
            std::fs::write(&generated_cfg_path, &$cfg_str).expect("write default canic config");
            $cfg_path = generated_cfg_path;
            println!("cargo:rerun-if-changed={}", $cfg_path.display());
        } else if let Some(parent) = $cfg_path.parent() {
            println!("cargo:rerun-if-changed={}", parent.display());
        }

        // Validate once on the host, then emit a precompiled runtime model.
        let $cfg = ::std::sync::Arc::new(
            $crate::__internal::core::bootstrap::parse_config_model(&$cfg_str)
                .expect("invalid canic config")
        );
        let compact_cfg = $crate::__internal::core::bootstrap::compact_config_source(&$cfg_str);
        let compiled_cfg =
            $crate::__internal::core::bootstrap::emit_config_model_source($cfg.as_ref());

        // Run the extra body (per-canister or nothing)
        $body

        // Emit compile-time endpoint surface flags from validated config.
        println!("cargo:rustc-check-cfg=cfg(canic_role_attestation_refresh)");
        println!("cargo:rustc-check-cfg=cfg(canic_delegated_tokens_enabled)");
        println!("cargo:rustc-check-cfg=cfg(canic_icrc21_enabled)");
        println!("cargo:rustc-check-cfg=cfg(canic_is_root)");
        println!("cargo:rustc-check-cfg=cfg(canic_has_root_wasm_store_bootstrap_release_set)");
        println!("cargo:rustc-check-cfg=cfg(canic_has_scaling)");
        println!("cargo:rustc-check-cfg=cfg(canic_has_sharding)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_icrc_standards)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_metadata)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_observability_env)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_observability_log)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_metrics)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_cycle_tracker)");
        println!("cargo:rustc-check-cfg=cfg(canic_metrics_core)");
        println!("cargo:rustc-check-cfg=cfg(canic_metrics_placement)");
        println!("cargo:rustc-check-cfg=cfg(canic_metrics_platform)");
        println!("cargo:rustc-check-cfg=cfg(canic_metrics_runtime)");
        println!("cargo:rustc-check-cfg=cfg(canic_metrics_security)");
        println!("cargo:rustc-check-cfg=cfg(canic_metrics_storage)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_auth_attestation)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_topology_state)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_topology_index)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_topology_children)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_topology_placement)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_nonroot_sync_topology)");
        if std::env::var_os("CANIC_INTERNAL_TEST_ENDPOINTS").is_none() {
            // Default builds ship the slimmer demo/reference surface; internal
            // observability and topology test endpoints opt back in explicitly.
            println!("cargo:rustc-cfg=canic_disable_bundle_observability_env");
            println!("cargo:rustc-cfg=canic_disable_bundle_topology_index");
        }
        if $cfg.auth.delegated_tokens.enabled {
            println!("cargo:rustc-cfg=canic_delegated_tokens_enabled");
        }

        let role_name = __canic_role_name.as_str();
        let mut role_attestation_refresh = false;
        let mut has_icrc21 = false;
        let mut has_scaling = false;
        let mut has_sharding = false;
        let mut metrics_core = false;
        let mut metrics_placement = false;
        let mut metrics_platform = false;
        let mut metrics_runtime = false;
        let mut metrics_security = false;
        let mut metrics_storage = false;
        let role_id: $crate::__internal::core::ids::CanisterRole = role_name.to_string().into();
        if !$crate::__build::config_contains_role($cfg.as_ref(), role_name) {
            panic!(
                "canister role '{}' from [package.metadata.canic] was not found in {}",
                role_name,
                $cfg_path.display()
            );
        }

        for subnet in $cfg.subnets.values() {
            if let Some(canister_cfg) = subnet.get_canister(&role_id) {
                role_attestation_refresh |= canister_cfg.auth.role_attestation_cache;
                has_icrc21 |= canister_cfg.standards.icrc21;
                has_scaling |= canister_cfg.scaling.is_some();
                has_sharding |= canister_cfg.sharding.is_some();
                let profile = canister_cfg.resolved_metrics_profile(&role_id);
                let tier_mask = $crate::__build::metrics_profile_tier_mask(profile);
                metrics_core |= tier_mask & $crate::__build::METRICS_TIER_CORE != 0;
                metrics_placement |= tier_mask & $crate::__build::METRICS_TIER_PLACEMENT != 0;
                metrics_platform |= tier_mask & $crate::__build::METRICS_TIER_PLATFORM != 0;
                metrics_runtime |= tier_mask & $crate::__build::METRICS_TIER_RUNTIME != 0;
                metrics_security |= tier_mask & $crate::__build::METRICS_TIER_SECURITY != 0;
                metrics_storage |= tier_mask & $crate::__build::METRICS_TIER_STORAGE != 0;
            }
        }

        if role_name == "root" {
            println!("cargo:rustc-cfg=canic_is_root");
            if $crate::__build::emit_root_wasm_store_bootstrap_release_set(&$cfg_path) {
                println!("cargo:rustc-cfg=canic_has_root_wasm_store_bootstrap_release_set");
            }
        }

        if has_icrc21 && $cfg.standards.as_ref().is_some_and(|standards| standards.icrc21) {
            println!("cargo:rustc-cfg=canic_icrc21_enabled");
        }

        if role_attestation_refresh {
            println!("cargo:rustc-cfg=canic_role_attestation_refresh");
        }

        if has_scaling {
            println!("cargo:rustc-cfg=canic_has_scaling");
        }

        if has_sharding {
            println!("cargo:rustc-cfg=canic_has_sharding");
        }

        if metrics_core {
            println!("cargo:rustc-cfg=canic_metrics_core");
        }

        if metrics_placement {
            println!("cargo:rustc-cfg=canic_metrics_placement");
        }

        if metrics_platform {
            println!("cargo:rustc-cfg=canic_metrics_platform");
        }

        if metrics_runtime {
            println!("cargo:rustc-cfg=canic_metrics_runtime");
        }

        if metrics_security {
            println!("cargo:rustc-cfg=canic_metrics_security");
        }

        if metrics_storage {
            println!("cargo:rustc-cfg=canic_metrics_storage");
        }

        let out_dir =
            std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR must be set"));
        let compact_cfg_path = out_dir.join("canic.compact.toml");
        let compiled_cfg_path = out_dir.join("canic.compiled.rs");
        std::fs::write(&compact_cfg_path, compact_cfg).expect("write compact canic config");
        std::fs::write(&compiled_cfg_path, compiled_cfg).expect("write compiled canic config");

        let compact_abs = compact_cfg_path
            .canonicalize()
            .expect("canonicalize compact canic config path");
        let compiled_abs = compiled_cfg_path
            .canonicalize()
            .expect("canonicalize compiled canic config path");
        let source_abs = $cfg_path
            .canonicalize()
            .expect("canonicalize source canic config path");

        println!("cargo:rustc-env=CANIC_CANISTER_ROLE={role_name}");
        println!("cargo:rustc-env=CANIC_CONFIG_PATH={}", source_abs.display());
        println!(
            "cargo:rustc-env=CANIC_CONFIG_SOURCE_PATH={}",
            compact_abs.display()
        );
        println!(
            "cargo:rustc-env=CANIC_CONFIG_MODEL_PATH={}",
            compiled_abs.display()
        );
        println!("cargo:rerun-if-changed={}", compact_abs.display());
        println!("cargo:rerun-if-changed={}", compiled_abs.display());
    }};
}
