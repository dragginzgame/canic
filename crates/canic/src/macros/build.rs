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
        println!(
            "cargo:rerun-if-env-changed={}",
            $crate::__internal::core::role_contract::CANONICAL_BUILD_MARKER_ENV
        );
        let __canic_target = std::env::var("TARGET").expect("TARGET must be set");
        let __canic_build_marker = std::env::var(
                $crate::__internal::core::role_contract::CANONICAL_BUILD_MARKER_ENV,
            )
            .ok();
        $crate::__build::assert_canonical_role_contract_build(
            &__canic_target,
            __canic_build_marker.as_deref(),
        );

        let manifest_dir =
            std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
        let __canic_package_metadata =
            $crate::__build::required_package_metadata(std::path::Path::new(&manifest_dir));
        let __canic_app_name = __canic_package_metadata.app;
        let __canic_role_name = __canic_package_metadata.role;
        let default_cfg_path = std::path::PathBuf::from(&manifest_dir).join($file);
        let __canic_config_path_env =
            $crate::__internal::core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV;
        let env_cfg = std::env::var(__canic_config_path_env).ok();
        let mut $cfg_path = env_cfg.as_ref().map_or(default_cfg_path, |value| {
            let path = std::path::PathBuf::from(value);
            if path.is_relative() {
                std::path::PathBuf::from(&manifest_dir).join(path)
            } else {
                path
            }
        });
        println!("cargo:rerun-if-changed={}", $cfg_path.display());
        println!("cargo:rerun-if-env-changed={__canic_config_path_env}");
        let __canic_release_build_id_env =
            $crate::__internal::core::ids::RELEASE_BUILD_ID_ENV;
        println!("cargo:rerun-if-env-changed={__canic_release_build_id_env}");
        if let Ok(value) = std::env::var(__canic_release_build_id_env) {
            let release_build_id = value
                .parse::<$crate::__internal::core::ids::ReleaseBuildId>()
                .expect("CANIC_RELEASE_BUILD_ID must be one canonical release-build ID");
            println!(
                "cargo:rustc-env={__canic_release_build_id_env}={release_build_id}"
            );
        }
        let __canic_protocol_profile_digest_env =
            $crate::__internal::core::role_contract::PROTOCOL_PROFILE_DIGEST_ENV;
        println!("cargo:rerun-if-env-changed={__canic_protocol_profile_digest_env}");
        if let Ok(value) = std::env::var(__canic_protocol_profile_digest_env) {
            let digest = value
                .parse::<$crate::__internal::core::role_contract::ProtocolProfileDigest>()
                .expect("CANIC_PROTOCOL_PROFILE_DIGEST must be one canonical lowercase SHA-256");
            println!(
                "cargo:rustc-env={__canic_protocol_profile_digest_env}={digest}"
            );
        }

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

        // Run the extra body (per-canister or nothing)
        $body

        // Emit compile-time endpoint surface flags from validated config.
        for custom_cfg in $crate::__build::CANIC_CUSTOM_CFG_NAMES {
            println!("cargo:rustc-check-cfg=cfg({custom_cfg})");
        }
        let __canic_candid_build_env =
            $crate::__internal::core::role_contract::CANONICAL_CANDID_BUILD_ENV;
        println!("cargo:rerun-if-env-changed={__canic_candid_build_env}");
        if std::env::var(__canic_candid_build_env).as_deref() == Ok("1") {
            println!("cargo:rustc-cfg=canic_export_candid");
        }
        let role_name = __canic_role_name.as_str();
        let role_id: $crate::__internal::core::ids::CanisterRole = role_name.to_string().into();
        let mut app_name = __canic_app_name.as_str();
        let __canic_wasm_store_special = role_name == "wasm_store";
        if __canic_wasm_store_special
            && !$crate::__build::config_declares_role($cfg.as_ref(), app_name, role_name)
        {
            app_name = $crate::__build::config_app_id($cfg.as_ref());
        }
        if !__canic_wasm_store_special
            && !$crate::__build::config_declares_role($cfg.as_ref(), app_name, role_name)
        {
            panic!(
                "canister role '{}.{}' from [package.metadata.canic] was not declared in {}",
                app_name,
                role_name,
                $cfg_path.display()
            );
        }
        let compiled_cfg =
            $crate::__internal::core::bootstrap::emit_config_model_source($cfg.as_ref());
        let role_runtime_authority =
            $crate::__internal::core::bootstrap::emit_role_runtime_authority_source(
                $cfg.as_ref(),
                &role_id,
                __canic_wasm_store_special,
            )
            .expect("compile role runtime authority");
        let metrics_tier_mask =
            $crate::__build::configured_role_metrics_tier_mask($cfg.as_ref(), &role_id);
        let metrics_core = metrics_tier_mask & $crate::__build::METRICS_TIER_CORE != 0;
        let metrics_placement = metrics_tier_mask & $crate::__build::METRICS_TIER_PLACEMENT != 0;
        let metrics_platform = metrics_tier_mask & $crate::__build::METRICS_TIER_PLATFORM != 0;
        let metrics_runtime = metrics_tier_mask & $crate::__build::METRICS_TIER_RUNTIME != 0;
        let metrics_security = metrics_tier_mask & $crate::__build::METRICS_TIER_SECURITY != 0;
        let metrics_storage = metrics_tier_mask & $crate::__build::METRICS_TIER_STORAGE != 0;

        let __canic_capabilities = if __canic_wasm_store_special {
            let kind = $crate::__internal::core::role_contract::BuiltInRoleKind::WasmStore;
            $crate::__internal::core::role_contract::built_in_role_capabilities(kind)
        } else {
            $crate::__internal::core::role_contract::derive_role_capabilities(
                $cfg.as_ref(),
                &role_id,
            )
            .unwrap_or_else(|finding| panic!("role contract rejected: {finding:?}"))
        };

        let delegated_token_issuer = __canic_capabilities.contains(
            &$crate::__internal::core::role_contract::RoleCapabilityKey::DelegatedTokenIssuer,
        );
        let has_icrc21 = __canic_capabilities
            .contains(&$crate::__internal::core::role_contract::RoleCapabilityKey::Icrc21);
        for capability in &__canic_capabilities {
            match capability {
                $crate::__internal::core::role_contract::RoleCapabilityKey::AutomaticTopup => {
                    println!("cargo:rustc-cfg=canic_capability_automatic_topup");
                }
                $crate::__internal::core::role_contract::RoleCapabilityKey::ChildProvisioning => {
                    println!("cargo:rustc-cfg=canic_capability_child_provisioning");
                }
                $crate::__internal::core::role_contract::RoleCapabilityKey::DelegatedTokenIssuer => {
                    println!("cargo:rustc-cfg=canic_capability_delegated_token_issuer");
                }
                $crate::__internal::core::role_contract::RoleCapabilityKey::DelegatedTokenVerifier => {
                    println!("cargo:rustc-cfg=canic_capability_delegated_token_verifier");
                }
                $crate::__internal::core::role_contract::RoleCapabilityKey::FleetAdmissionProjection => {
                    println!("cargo:rustc-cfg=canic_capability_fleet_admission_projection");
                }
                $crate::__internal::core::role_contract::RoleCapabilityKey::FleetCoordinator => {
                    println!("cargo:rustc-cfg=canic_capability_fleet_coordinator");
                }
                $crate::__internal::core::role_contract::RoleCapabilityKey::Index => {
                    println!("cargo:rustc-cfg=canic_capability_index");
                }
                $crate::__internal::core::role_contract::RoleCapabilityKey::LocalApplicationAuthorization => {
                    println!("cargo:rustc-cfg=canic_capability_local_application_authorization");
                }
                $crate::__internal::core::role_contract::RoleCapabilityKey::Icrc21 => {
                    println!("cargo:rustc-cfg=canic_capability_icrc21");
                }
                $crate::__internal::core::role_contract::RoleCapabilityKey::RoleAttestationSigner => {
                    println!("cargo:rustc-cfg=canic_capability_role_attestation_signer");
                }
                $crate::__internal::core::role_contract::RoleCapabilityKey::RoleAttestationVerifier => {
                    println!("cargo:rustc-cfg=canic_capability_role_attestation_verifier");
                }
                $crate::__internal::core::role_contract::RoleCapabilityKey::Root => {
                    println!("cargo:rustc-cfg=canic_capability_root");
                }
                $crate::__internal::core::role_contract::RoleCapabilityKey::RootControlPlane => {
                    println!("cargo:rustc-cfg=canic_capability_root_control_plane");
                }
                $crate::__internal::core::role_contract::RoleCapabilityKey::Runtime => {
                    println!("cargo:rustc-cfg=canic_capability_runtime");
                }
                $crate::__internal::core::role_contract::RoleCapabilityKey::Scaling => {
                    println!("cargo:rustc-cfg=canic_capability_scaling");
                }
                $crate::__internal::core::role_contract::RoleCapabilityKey::Sharding => {
                    println!("cargo:rustc-cfg=canic_capability_sharding");
                }
                $crate::__internal::core::role_contract::RoleCapabilityKey::WasmStore => {
                    println!("cargo:rustc-cfg=canic_capability_wasm_store");
                }
            }
        }

        if __canic_capabilities
            .contains(&$crate::__internal::core::role_contract::RoleCapabilityKey::Root)
        {
            println!("cargo:rustc-cfg=canic_is_root");
        }

        if has_icrc21 && $cfg.standards.as_ref().is_some_and(|standards| standards.icrc21) {
            println!("cargo:rustc-cfg=canic_icrc21_enabled");
        }

        if delegated_token_issuer {
            println!("cargo:rustc-cfg=canic_delegated_token_issuer");
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
        let role_runtime_authority_path = out_dir.join("canic.role-runtime-authority.rs");
        std::fs::write(&compact_cfg_path, compact_cfg).expect("write compact canic config");
        std::fs::write(&compiled_cfg_path, compiled_cfg).expect("write compiled canic config");
        std::fs::write(&role_runtime_authority_path, role_runtime_authority)
            .expect("write compiled role runtime authority");

        let compact_abs = compact_cfg_path
            .canonicalize()
            .expect("canonicalize compact canic config path");
        let compiled_abs = compiled_cfg_path
            .canonicalize()
            .expect("canonicalize compiled canic config path");
        let role_runtime_authority_abs = role_runtime_authority_path
            .canonicalize()
            .expect("canonicalize compiled role runtime authority path");
        let source_abs = $cfg_path
            .canonicalize()
            .expect("canonicalize source canic config path");

        println!("cargo:rustc-env=CANIC_CANISTER_ROLE={role_name}");
        println!(
            "cargo:rustc-env=CANIC_CONFIG_ORIGIN_PATH={}",
            source_abs.display()
        );
        println!(
            "cargo:rustc-env=CANIC_CONFIG_SOURCE_PATH={}",
            compact_abs.display()
        );
        println!(
            "cargo:rustc-env=CANIC_CONFIG_MODEL_PATH={}",
            compiled_abs.display()
        );
        println!(
            "cargo:rustc-env=CANIC_ROLE_RUNTIME_AUTHORITY_PATH={}",
            role_runtime_authority_abs.display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            role_runtime_authority_abs.display()
        );
        println!("cargo:rerun-if-changed={}", compact_abs.display());
        println!("cargo:rerun-if-changed={}", compiled_abs.display());
    }};
}
