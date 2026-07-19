use super::*;
use crate::test_support::temp_dir;
use std::{collections::BTreeMap, fmt::Write as _, fs, path::Path};

#[test]
fn defaults_local_gateway_port_without_network_config() {
    let source = "canisters: []\n";

    assert_eq!(
        local_gateway_port_from_yaml(source),
        DEFAULT_LOCAL_GATEWAY_PORT
    );
}

#[test]
fn reads_local_gateway_port_from_network_config() {
    let source = "networks:\n  - name: local\n    mode: managed\n    gateway:\n      bind: 127.0.0.1\n      port: 8001\n";

    assert_eq!(local_gateway_port_from_yaml(source), 8001);
}

#[test]
fn ignores_nested_networks_keys_when_reading_local_gateway_port() {
    let source = "canisters:\n  - name: root\n    metadata:\n      networks:\n        - local\n\nnetworks:\n  - name: local\n    mode: managed\n    gateway:\n      bind: 127.0.0.1\n      port: 8010\n";

    assert_eq!(local_gateway_port_from_yaml(source), 8010);
}

#[test]
fn resolves_implicit_build_networks_without_project_config() {
    let root = temp_dir("canic-icp-build-network-implicit");

    assert_eq!(
        resolve_icp_build_network_from_root(&root, "local").expect("resolve local"),
        BuildNetwork::Local
    );
    assert_eq!(
        resolve_icp_build_network_from_root(&root, "ic").expect("resolve ic"),
        BuildNetwork::Ic
    );
}

#[test]
fn resolves_named_environment_from_declared_backing_network() {
    let source = r"
networks:
  - name: local
    mode: managed

environments:
  - name: demo
    network: local
  - name: staging
    network: ic
";

    assert_eq!(
        resolve_icp_build_network_from_yaml(source, "demo").expect("resolve demo"),
        BuildNetwork::Local
    );
    assert_eq!(
        resolve_icp_build_network_from_yaml(source, "staging").expect("resolve staging"),
        BuildNetwork::Ic
    );
}

#[test]
fn resolves_declared_nonmainnet_networks_as_local_builds() {
    let source = r"
networks:
  - name: local-container
    mode: managed
  - name: testnet
    mode: connected
    url: https://testnet.example

environments:
  - name: docker
    network: local-container
  - name: test
    network: testnet
";

    assert_eq!(
        resolve_icp_build_network_from_yaml(source, "docker").expect("resolve managed"),
        BuildNetwork::Local
    );
    assert_eq!(
        resolve_icp_build_network_from_yaml(source, "test").expect("resolve connected"),
        BuildNetwork::Local
    );
}

#[test]
fn build_network_resolution_rejects_incomplete_config() {
    let missing_target = resolve_icp_build_network_from_yaml("environments: []\n", "staging")
        .expect_err("missing target environment should fail");
    let missing_backing_network = resolve_icp_build_network_from_yaml(
        "environments:\n  - name: staging\n    network: private\n",
        "staging",
    )
    .expect_err("missing backing network should fail");
    let unsupported_mode = resolve_icp_build_network_from_yaml(
        "networks:\n  - name: private\n    mode: mystery\nenvironments:\n  - name: staging\n    network: private\n",
        "staging",
    )
    .expect_err("unsupported network mode should fail");

    assert!(missing_target.contains("is not declared"));
    assert!(missing_backing_network.contains("references undeclared backing network 'private'"));
    assert!(unsupported_mode.contains("unsupported mode 'mystery'"));
}

#[test]
fn inspects_icp_yaml_without_mutating_it() {
    let root = temp_dir("canic-icp-read-only");
    let config = root.join("fleets/toko/canic.toml");
    fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
    fs::write(
        &config,
        r#"
[fleet]
name = "toko"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "service"
"#,
    )
    .expect("write config");
    let source = r"
canisters:
  - name: root

networks:
  - name: local
    mode: managed
    gateway:
      port: 8010

environments:
  - name: toko
    network: local
    canisters: [root]
";
    fs::write(root.join("icp.yaml"), source).expect("write icp yaml");

    let report = inspect_canic_icp_yaml_from_root(&root, Some("toko")).expect("inspect");

    assert_eq!(report.canisters, vec!["root", "app"]);
    assert_eq!(report.environments, vec!["toko"]);
    assert_eq!(report.missing_canisters, vec!["app"]);
    assert!(report.missing_environments.is_empty());
    assert!(report.local_network_present);
    assert!(!report.is_ready());
    assert_eq!(
        fs::read_to_string(root.join("icp.yaml")).expect("read icp yaml"),
        source
    );
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn reports_missing_icp_yaml_as_incomplete() {
    let root = temp_dir("canic-icp-missing-yaml");
    let config = root.join("fleets/toko/canic.toml");
    fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
    fs::write(
        &config,
        r#"
[fleet]
name = "toko"

[roles.root]
kind = "root"
package = "root"

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect("write config");

    let report = inspect_canic_icp_yaml_from_root(&root, Some("toko")).expect("inspect");

    assert!(!report.icp_yaml_present);
    assert_eq!(report.missing_canisters, vec!["root"]);
    assert_eq!(report.missing_environments, vec!["toko"]);
    assert!(!report.local_network_present);
    assert!(!report.is_ready());
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn discovers_root_fleet_configs_for_icp_inspection() {
    let root = temp_dir("canic-icp-inspect-root-fleets");
    let config = root.join("fleets/toko/canic.toml");
    fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
    fs::write(
        &config,
        r#"
[fleet]
name = "toko"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "service"
"#,
    )
    .expect("write config");

    let spec = discover_project_spec(&root, Some("toko")).expect("discover spec");

    assert_eq!(spec.canisters, vec!["root", "app"]);
    assert_eq!(
        spec.environments,
        BTreeMap::from([(
            "toko".to_string(),
            vec!["root".to_string(), "app".to_string()]
        )])
    );
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn fleet_filter_limits_inspected_project_spec() {
    let root = temp_dir("canic-icp-inspect-fleet-filter");
    write_test_config(
        &root.join("fleets/demo/canic.toml"),
        "demo",
        &["root", "app"],
    );
    write_test_config(
        &root.join("fleets/test/canic.toml"),
        "test",
        &["root", "scale"],
    );

    let spec = discover_project_spec(&root, Some("test")).expect("discover spec");

    assert_eq!(spec.canisters, vec!["root", "scale"]);
    assert_eq!(
        spec.environments,
        BTreeMap::from([(
            "test".to_string(),
            vec!["root".to_string(), "scale".to_string()]
        )])
    );
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn nested_commands_discover_outer_project_root_with_fleets() {
    let root = temp_dir("canic-icp-root-nested");
    let config = root.join("fleets/toko/canic.toml");
    let nested = root.join("backend/src");
    fs::create_dir_all(&nested).expect("create nested dir");
    fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
    fs::write(root.join("icp.yaml"), "").expect("write icp config");
    fs::write(&config, "[fleet]\nname = \"toko\"\n").expect("write config");

    let icp_root = crate::install_root::discover_canic_project_root_from(&nested)
        .expect("discover project root")
        .expect("project root is present");

    assert_eq!(icp_root, root.canonicalize().expect("canonical root"));
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn outer_project_root_wins_over_nested_fleets() {
    let root = temp_dir("canic-icp-root-outer-wins");
    let outer_config = root.join("fleets/toko/canic.toml");
    let nested_config = root.join("services/fleets/toko/canic.toml");
    let nested = root.join("services/src");
    fs::create_dir_all(outer_config.parent().expect("outer config parent"))
        .expect("create outer config parent");
    fs::create_dir_all(nested_config.parent().expect("nested config parent"))
        .expect("create nested config parent");
    fs::create_dir_all(&nested).expect("create nested dir");
    fs::write(root.join("icp.yaml"), "").expect("write icp config");
    fs::write(&outer_config, "[fleet]\nname = \"toko\"\n").expect("write outer config");
    fs::write(&nested_config, "[fleet]\nname = \"toko\"\n").expect("write nested config");

    let icp_root = crate::install_root::discover_canic_project_root_from(&nested)
        .expect("discover project root")
        .expect("project root is present");

    assert_eq!(icp_root, root.canonicalize().expect("canonical root"));
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn icp_inspection_rejects_missing_fleet_configs() {
    let root = temp_dir("canic-icp-inspect-missing");
    fs::create_dir_all(&root).expect("create root");

    let err = discover_project_spec(&root, None).expect_err("missing configs should fail");
    let message = err.to_string();

    assert!(message.contains("no Canic fleet configs found under"));
    assert!(message.contains("fleets/<fleet>/canic.toml"));
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn icp_inspection_preserves_invalid_fleet_config_cause() {
    let root = temp_dir("canic-icp-invalid-fleet-config");
    let config = root.join("fleets/toko/canic.toml");
    fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
    fs::write(&config, "[fleet\nname = \"toko\"\n").expect("write invalid config");

    let error = discover_project_spec(&root, None).expect_err("invalid config should fail");
    let IcpConfigError::FleetConfig(FleetConfigError::ConfigInvalid { path, .. }) = error else {
        panic!("expected typed fleet config cause");
    };

    assert_eq!(path, config);
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn icp_config_preserves_workspace_discovery_cause() {
    let error = IcpConfigError::from(WorkspaceDiscoveryError::UnsupportedPath {
        path: PathBuf::from("/project/socket"),
    });

    std::assert_matches!(
        error,
        IcpConfigError::WorkspaceDiscovery(WorkspaceDiscoveryError::UnsupportedPath { .. })
    );
}

fn write_test_config(path: &Path, fleet: &str, roles: &[&str]) {
    fs::create_dir_all(path.parent().expect("config parent")).expect("create config parent");
    let mut source = format!("[fleet]\nname = \"{fleet}\"\n");
    for role in roles {
        let kind = if *role == "root" { "root" } else { "canister" };
        write!(
            source,
            "\n[roles.{role}]\nkind = \"{kind}\"\npackage = \"{role}\"\n"
        )
        .expect("write role declaration");
    }
    for role in roles {
        let kind = if *role == "root" { "root" } else { "service" };
        write!(
            source,
            "\n[subnets.prime.canisters.{role}]\nkind = \"{kind}\"\n"
        )
        .expect("write config source");
    }
    fs::write(path, source).expect("write config");
}
