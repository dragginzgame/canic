use super::{FrontendError, model::*, ops, policy};
use crate::test_support::temp_dir;
use candid::Principal;
use canic_core::ids::{CanisterRole, CanonicalNetworkId};
use std::fs;

fn input() -> FrontendEnvironmentInput {
    FrontendEnvironmentInput {
        schema_version: 1,
        environment: "ic".to_string(),
        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
        api_origin: "https://icp-api.io".to_string(),
        identity: FrontendIdentityInput {
            canister_id: Principal::from_slice(&[1]),
            provider_origin: "https://identity.example.com".to_string(),
            derivation_origin: "https://service.example.com".to_string(),
            alternative_origins: vec!["https://app.example.com".to_string()],
        },
        asset: Some(FrontendAssetInput {
            canister_id: Principal::from_slice(&[2]),
            origin: "https://app.example.com".to_string(),
        }),
        roles: vec![FrontendRoleInput {
            role: CanisterRole::from("app"),
            canister_id: Principal::from_slice(&[3]),
        }],
    }
}

#[test]
fn origin_validation_preserves_the_principal_namespace() {
    for origin in [
        "https://service.example.com",
        "https://service.example.com:8443",
    ] {
        policy::validate_origin(origin, false).unwrap();
    }
    for origin in [
        "https://service.example.com/",
        "https://service.example.com/path",
        "https://service.example.com?query=1",
        "https://service.example.com#fragment",
        "https://user@service.example.com",
        "https://service.example.com:443",
        "https://SERVICE.example.com",
        "http://service.example.com",
        "file:///tmp/frontend",
    ] {
        assert!(matches!(
            policy::validate_origin(origin, false),
            Err(FrontendError::Origin(_))
        ));
    }
    for origin in [
        "http://localhost:8000",
        "http://127.0.0.1:8000",
        "http://[::1]:8000",
        "http://asset.localhost:8000",
    ] {
        policy::validate_origin(origin, true).unwrap();
        assert!(matches!(
            policy::validate_origin(origin, false),
            Err(FrontendError::Origin(_))
        ));
    }
    assert!(matches!(
        policy::validate_origin("http://localhost.example.com:8000", true),
        Err(FrontendError::Origin(_))
    ));
}

#[test]
fn admission_origin_is_required_for_nonempty_fleet_admission() {
    let configuration = input();
    let check = |origin| {
        policy::validate_input(
            &configuration,
            "ic",
            configuration.canonical_network_id,
            true,
            origin,
        )
    };
    check(Some("https://service.example.com")).unwrap();
    for origin in [None, Some("https://other.example.com")] {
        assert!(matches!(check(origin), Err(FrontendError::AdmissionOrigin)));
    }
    assert!(matches!(
        policy::validate_input(
            &configuration,
            "local",
            configuration.canonical_network_id,
            false,
            None
        ),
        Err(FrontendError::Environment)
    ));
}

#[test]
fn alternative_origins_follow_the_current_ii_bound_and_asset_binding() {
    let mut configuration = input();
    configuration.identity.alternative_origins = (0..policy::MAX_ALTERNATIVE_ORIGINS)
        .map(|index| format!("https://app{index}.example.com"))
        .collect();
    configuration.asset.as_mut().unwrap().origin =
        configuration.identity.alternative_origins[0].clone();
    policy::validate_input(
        &configuration,
        "ic",
        configuration.canonical_network_id,
        false,
        None,
    )
    .unwrap();
    configuration
        .identity
        .alternative_origins
        .push("https://overflow.example.com".to_string());
    assert!(matches!(
        policy::validate_input(
            &configuration,
            "ic",
            configuration.canonical_network_id,
            false,
            None
        ),
        Err(FrontendError::Bound(_))
    ));
    configuration.identity.alternative_origins = vec!["https://app0.example.com".to_string(); 2];
    assert!(matches!(
        policy::validate_input(
            &configuration,
            "ic",
            configuration.canonical_network_id,
            false,
            None
        ),
        Err(FrontendError::OriginSet)
    ));
    configuration.identity.alternative_origins.clear();
    configuration
        .asset
        .as_mut()
        .unwrap()
        .origin
        .clone_from(&configuration.identity.derivation_origin);
    policy::validate_input(
        &configuration,
        "ic",
        configuration.canonical_network_id,
        false,
        None,
    )
    .unwrap();
}

#[test]
#[ignore = "requires Node.js; run explicitly for frontend consumer qualification"]
fn generated_bindings_are_valid_javascript_and_reject_unbound_imports() {
    let bindings = ops::bindings(b"service : { whoami : () -> (principal) query; }").unwrap();
    let root = temp_dir("frontend-bindings");
    fs::create_dir_all(&root).unwrap();
    let javascript = root.join("service.mjs");
    fs::write(&javascript, bindings.javascript).unwrap();
    let checked = std::process::Command::new("node")
        .arg("--check")
        .arg(javascript)
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    assert!(!bindings.typescript.is_empty());
    assert!(matches!(
        ops::bindings(b"import \"outside.did\"; service : {};"),
        Err(FrontendError::Candid(_))
    ));
    fs::remove_dir_all(root).unwrap();
}

fn prepared_bundle(root: &std::path::Path) -> super::view::FrontendBundleView {
    let configuration = input();
    fs::create_dir_all(root).unwrap();
    let candid = b"service : { whoami : () -> (principal) query; }";
    let sidecar = crate::icp::local_canister_candid_path(root, "ic", "app");
    fs::create_dir_all(sidecar.parent().unwrap()).unwrap();
    fs::write(sidecar, candid).unwrap();
    let capabilities = std::collections::BTreeSet::new();
    let role = CanisterRole::from("app");
    let hashes = canic_core::role_contract::derive_protocol_profile_hashes(
        "fixture-release",
        &role,
        &capabilities,
        candid,
    );
    let authority = super::view::FrontendAuthorityView {
        app: "fixture".to_string(),
        fleet: "fixture".to_string(),
        fleet_id: "41".repeat(32),
        source_plan_sha256: "42".repeat(32),
        network: CanonicalNetworkId::ic_mainnet(),
        admission_nonempty: true,
        admission_origin: Some(configuration.identity.derivation_origin.clone()),
        entries: vec![crate::registry::RegistryEntry {
            pid: configuration.roles[0].canister_id.to_text(),
            role: Some(role.to_string()),
            parent_pid: None,
            module_hash: Some("43".repeat(32)),
            protocol_binding: Some(crate::protocol_binding::RegistryProtocolBinding {
                release_identity: "fixture-release".to_string(),
                role,
                capabilities,
                candid_sha256: hashes.candid_sha256,
                protocol_profile_digest: hashes.protocol_profile_digest,
            }),
        }],
    };
    ops::prepare_bundle(root, &configuration, authority).unwrap()
}

#[test]
fn complete_bundle_and_interrupted_publication_have_identical_verified_bytes() {
    let root = temp_dir("frontend-publish");
    let bundle = prepared_bundle(&root);
    let destination = root.join("browser");
    let (name, bytes) = bundle.files.first_key_value().unwrap();
    crate::durable_io::create_new_bytes_with_parents(&destination.join(name), bytes).unwrap();
    assert!(!destination.join("canic-frontend.json").exists());
    ops::publish_bundle(&destination, &bundle).unwrap();
    let verified = ops::verify_bundle(&destination, &bundle.manifest.manifest_sha256).unwrap();
    assert_eq!(verified, bundle.manifest);
    ops::publish_bundle(&destination, &bundle).unwrap();
    assert!(matches!(
        ops::verify_bundle(&destination, &"00".repeat(32)),
        Err(FrontendError::Integrity)
    ));
    let json = serde_json::to_value(&verified).unwrap();
    for field in ["controllers", "operator", "admission", "provisioning"] {
        assert!(json.get(field).is_none());
    }
    assert!(verified.local_root_key_der_hex.is_none());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn tampered_files_and_conflicting_publication_do_not_replace_browser_assets() {
    let root = temp_dir("frontend-tamper");
    let bundle = prepared_bundle(&root);
    let destination = root.join("browser");
    ops::publish_bundle(&destination, &bundle).unwrap();
    let javascript = destination.join(&bundle.manifest.roles[0].javascript.path);
    fs::write(&javascript, "modified").unwrap();
    assert!(matches!(
        ops::verify_bundle(&destination, &bundle.manifest.manifest_sha256),
        Err(FrontendError::Integrity)
    ));
    assert!(matches!(
        ops::publish_bundle(&destination, &bundle),
        Err(FrontendError::Integrity)
    ));
    assert_eq!(fs::read(javascript).unwrap(), b"modified");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_prepared_bundle_never_publishes_a_manifest() {
    let root = temp_dir("frontend-invalid-prepared");
    let mut bundle = prepared_bundle(&root);
    bundle.manifest.roles[0].candid.path = "../outside.did".to_string();
    bundle.manifest.manifest_sha256 = ops::manifest_digest(&bundle.manifest).unwrap();
    let destination = root.join("browser");
    assert!(matches!(
        ops::publish_bundle(&destination, &bundle),
        Err(FrontendError::Integrity)
    ));
    assert!(!destination.join("canic-frontend.json").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn input_does_not_accept_operator_authority_fields() {
    let mut json = serde_json::to_value(input()).unwrap();
    json["operator"] = serde_json::json!(Principal::from_slice(&[7]).to_text());
    assert!(serde_json::from_value::<FrontendEnvironmentInput>(json).is_err());
}

#[test]
fn payload_inventory_is_content_bound_and_enforces_explicit_limits() {
    let root = temp_dir("frontend-payload");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("index.html"), b"page").unwrap();
    fs::write(root.join("nested/script.js"), b"script").unwrap();
    let first = ops::payload_inventory(&root, 10, 2).unwrap();
    assert_eq!((first.files, first.bytes), (2, 10));
    assert_eq!(ops::payload_inventory(&root, 10, 2).unwrap(), first);
    assert!(matches!(
        ops::payload_inventory(&root, 9, 2),
        Err(FrontendError::Bound("payload bytes"))
    ));
    assert!(matches!(
        ops::payload_inventory(&root, 10, 1),
        Err(FrontendError::Bound("payload files"))
    ));
    assert!(matches!(
        ops::payload_inventory(&root.join("index.html"), 10, 2),
        Err(FrontendError::Integrity)
    ));
    fs::write(root.join("index.html"), b"edit").unwrap();
    assert_ne!(
        ops::payload_inventory(&root, 10, 2).unwrap().sha256,
        first.sha256
    );
    fs::rename(
        root.join("nested/script.js"),
        root.join("nested/renamed.js"),
    )
    .unwrap();
    assert_ne!(
        ops::payload_inventory(&root, 10, 2).unwrap().sha256,
        first.sha256
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[cfg(unix)]
fn payload_inventory_rejects_symlinks_and_special_files_without_reading_them() {
    let root = temp_dir("frontend-payload-files");
    fs::create_dir_all(&root).unwrap();
    std::os::unix::fs::symlink("missing", root.join("link")).unwrap();
    assert!(matches!(
        ops::payload_inventory(&root, 10, 2),
        Err(FrontendError::Integrity)
    ));
    fs::remove_file(root.join("link")).unwrap();
    crate::test_support::create_fifo(&root.join("fifo"));
    assert!(matches!(
        ops::payload_inventory(&root, 10, 2),
        Err(FrontendError::Integrity)
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires the pinned Node.js consumer dependencies; run explicitly for frontend qualification"]
fn independent_sdk_consumer_verifies_canonical_bundle_and_rejects_tampering() {
    let root = temp_dir("frontend-sdk-consumer");
    let bundle = prepared_bundle(&root);
    let destination = root.join("browser");
    ops::publish_bundle(&destination, &bundle).unwrap();
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/frontend-consumer/qualify.mjs");
    let verify = |digest: &str| {
        std::process::Command::new("node")
            .arg(&script)
            .arg("verify")
            .arg(&destination)
            .arg(digest)
            .output()
            .unwrap()
    };
    let result = verify(&bundle.manifest.manifest_sha256);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!verify(&"00".repeat(32)).status.success());
    fs::write(
        destination.join(&bundle.manifest.roles[0].javascript.path),
        "modified",
    )
    .unwrap();
    assert!(!verify(&bundle.manifest.manifest_sha256).status.success());
    fs::remove_dir_all(root).unwrap();
}
