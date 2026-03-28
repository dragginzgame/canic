// -----------------------------------------------------------------------------
// Build macros
// -----------------------------------------------------------------------------

/// Embed the shared Canic configuration into a canister crate's build script.
///
/// Reads the provided TOML file (relative to the crate manifest dir), validates it
/// using [`Config`](crate::__internal::core::config::Config), and sets
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
            |_cfg_str, _cfg_path, _cfg| {
                // `start_root!` must always compile the root-only capability
                // surface, even for test/support crates whose package names do
                // not follow the `canister_root` naming convention.
                println!("cargo:rustc-cfg=canic_is_root");
            }
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
        let $cfg = $crate::__internal::core::bootstrap::init_config(&$cfg_str).expect("invalid canic config");

        // Run the extra body (per-canister or nothing)
        $body

        // Emit compile-time endpoint surface flags from validated config.
        println!("cargo:rustc-check-cfg=cfg(canic_accepts_delegation_signer_proof)");
        println!("cargo:rustc-check-cfg=cfg(canic_accepts_delegation_verifier_proof)");
        println!("cargo:rustc-check-cfg=cfg(canic_delegated_tokens_enabled)");
        println!("cargo:rustc-check-cfg=cfg(canic_icrc21_enabled)");
        println!("cargo:rustc-check-cfg=cfg(canic_is_root)");
        println!("cargo:rustc-check-cfg=cfg(canic_has_scaling)");
        println!("cargo:rustc-check-cfg=cfg(canic_has_sharding)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_standards_icrc)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_standards_icts)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_observability_memory)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_observability_env)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_observability_log)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_metrics)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_auth_attestation)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_topology_state)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_topology_directory)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_topology_children)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_topology_cycles)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_topology_placement)");
        println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_nonroot_sync_topology)");

        if $cfg.auth.delegated_tokens.enabled {
            println!("cargo:rustc-cfg=canic_delegated_tokens_enabled");
        }

        if let Ok(package_name) = std::env::var("CARGO_PKG_NAME") {
            let inferred_role_name = package_name
                .strip_prefix("canister_")
                .map(str::to_string)
                .or_else(|| {
                    let mut roles = Vec::new();

                    for subnet in $cfg.subnets.values() {
                        for role in subnet.canisters.keys() {
                            if role.is_root() {
                                continue;
                            }

                            let role_name = role.as_str();
                            if roles.iter().all(|existing| existing != role_name) {
                                roles.push(role_name.to_string());
                            }
                        }
                    }

                    (roles.len() == 1).then(|| roles.remove(0))
                });

            if let Some(role_name) = inferred_role_name.as_deref() {
                let mut role_found = false;
                let mut accepts_signer_proof = false;
                let mut accepts_verifier_proof = false;
                let mut has_icrc21 = false;
                let mut has_scaling = false;
                let mut has_sharding = false;
                let role_id: $crate::__internal::core::ids::CanisterRole =
                    role_name.to_string().into();

                for subnet in $cfg.subnets.values() {
                    if let Some(canister_cfg) = subnet.get_canister(&role_id) {
                        role_found = true;
                        accepts_signer_proof |= canister_cfg.delegated_auth.signer;
                        accepts_verifier_proof |= canister_cfg.delegated_auth.verifier;
                        has_icrc21 |= canister_cfg.standards.icrc21;
                        has_scaling |= canister_cfg.scaling.is_some();
                        has_sharding |= canister_cfg.sharding.is_some();
                    }
                }

                if role_found {
                    if role_name == "root" {
                        println!("cargo:rustc-cfg=canic_is_root");
                    }

                    if has_icrc21 && $cfg.standards.as_ref().is_some_and(|standards| standards.icrc21)
                    {
                        println!("cargo:rustc-cfg=canic_icrc21_enabled");
                    }

                    if accepts_signer_proof && $cfg.auth.delegated_tokens.enabled {
                        println!("cargo:rustc-cfg=canic_accepts_delegation_signer_proof");
                    }

                    if accepts_verifier_proof && $cfg.auth.delegated_tokens.enabled {
                        println!("cargo:rustc-cfg=canic_accepts_delegation_verifier_proof");
                    }

                    if has_scaling {
                        println!("cargo:rustc-cfg=canic_has_scaling");
                    }

                    if has_sharding {
                        println!("cargo:rustc-cfg=canic_has_sharding");
                    }
                } else if package_name.starts_with("canister_") && role_name != "root" {
                    panic!(
                        "canister role '{}' from package '{}' was not found in {}",
                        role_name,
                        package_name,
                        $cfg_path.display()
                    );
                }
            }
        }

        let abs = $cfg_path.canonicalize().expect("canonicalize canic config path");
        println!("cargo:rustc-env=CANIC_CONFIG_PATH={}", abs.display());
        println!("cargo:rerun-if-changed={}", abs.display());
    }};
}
