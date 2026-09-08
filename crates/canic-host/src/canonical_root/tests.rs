use super::*;
use crate::fleet_package::dependency_patch_table;
use crate::role_contract::{
    RolePackageValidation, resolve_declared_role_package_contract, validate_declared_role_package,
};
use canic_core::{bootstrap::parse_config_model, role_contract::RoleContractResolution};
use std::fs;

fn entrypoint_calls(source: &str) -> Vec<String> {
    let file = syn::parse_file(source).unwrap();
    assert!(file.attrs.iter().all(|attr| attr.path().is_ident("doc")));
    file.items
        .into_iter()
        .map(|item| {
            let syn::Item::Macro(item) = item else {
                panic!("canonical entrypoint must contain only lifecycle composition");
            };
            assert!(item.attrs.iter().all(|attr| attr.path().is_ident("doc")));
            assert!(item.mac.tokens.is_empty());
            item.mac
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("::")
        })
        .collect()
}

#[test]
fn standalone_consumer_build_resolves_root_without_an_existing_lockfile() {
    let scratch = crate::test_support::temp_dir("canonical-root-consumer");
    fs::create_dir_all(scratch.join("src")).unwrap();
    fs::write(scratch.join("src/lib.rs"), "").unwrap();
    let canic_manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../canic/Cargo.toml")
        .canonicalize()
        .unwrap();
    let document = serde_json::json!({
        "package": {"name": "root-consumer", "version": "0.0.0", "edition": "2024"},
        "workspace": {},
        "dependencies": {"canic": {"path": canic_manifest.parent().unwrap(), "default-features": false}},
    });
    let mut manifest = toml::to_string(&toml::Value::try_from(document).unwrap()).unwrap();
    manifest.push_str(&dependency_patch_table(&canic_manifest, env!("CARGO_PKG_VERSION")).unwrap());
    fs::write(scratch.join("Cargo.toml"), manifest).unwrap();
    let source = "[app]\nname = 'consumer'\n[roles.root]\nkind = 'root'\n[auth.delegated_tokens]\nenabled = false\n";
    let config_path = scratch.join("canic.toml");
    fs::write(&config_path, source).unwrap();
    let config = parse_config_model(source).unwrap();
    let validation = validate_declared_role_package(
        &config_path,
        &config,
        &CanisterRole::ROOT,
        PackageValidationMode::Build,
    );
    assert!(
        matches!(validation, RolePackageValidation::Supported(_)),
        "{validation:?}"
    );
    assert!(scratch.join("Cargo.lock").is_file());
    fs::remove_dir_all(scratch).unwrap();
}

#[test]
fn canonical_root_build_selects_exact_configuration_capabilities() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let unique = crate::test_support::temp_dir("canonical-root");
    let scratch = workspace.join(".tmp").join(unique.file_name().unwrap());
    fs::create_dir_all(&scratch).unwrap();
    let config_path = scratch.join("canic.toml");
    let source = fs::read_to_string(workspace.join("apps/test/canic.toml")).unwrap();
    fs::write(&config_path, &source).unwrap();
    let mut config = parse_config_model(&source).unwrap();

    for enabled in [true, false] {
        config.auth.delegated_tokens.enabled = enabled;
        let validation = validate_declared_role_package(
            &config_path,
            &config,
            &CanisterRole::ROOT,
            PackageValidationMode::Build,
        );
        let RolePackageValidation::Supported(evidence) = validation else {
            panic!("canonical Root package must resolve: {validation:?}");
        };
        assert_eq!(evidence.role_package_name, PACKAGE);
        assert!(!evidence.default_features_enabled);
        let required = required_features_for_role(&config, &CanisterRole::ROOT)
            .unwrap()
            .into_iter()
            .map(|requirement| requirement.feature)
            .collect();
        assert_eq!(evidence.direct_features, required);
        assert!(matches!(
            resolve_declared_role_package_contract(&config, &evidence),
            RoleContractResolution::Resolved { .. }
        ));
        assert_eq!(evidence.canic_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(
            entrypoint_calls(
                &fs::read_to_string(
                    evidence
                        .role_manifest_path
                        .parent()
                        .unwrap()
                        .join("src/lib.rs")
                )
                .unwrap()
            ),
            ["canic::start_fleet_root", "canic::finish"]
        );
    }
    fs::remove_dir_all(scratch).unwrap();
}
