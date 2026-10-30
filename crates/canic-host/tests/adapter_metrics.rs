//! Real TOML → build cfg → managed adapter export selection, without an IcyDB engine.

use std::{env, fs, path::Path, process::Command};

fn run(root: &Path, args: &[&str], label: &str) {
    let output = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .current_dir(root)
        .env("CARGO_NET_OFFLINE", "true")
        .env("CARGO_TARGET_DIR", root.join("target"))
        .args(args)
        .output()
        .unwrap();
    let log = root.join(format!("{label}.log"));
    fs::write(&log, [&output.stdout[..], &output.stderr[..]].concat()).unwrap();
    assert!(
        output.status.success(),
        "{label} failed; retained {}",
        log.display()
    );
}

fn assert_locked_dependency_identity(workspace: &Path, fixture: &Path) {
    let read = |root: &Path| -> toml::Value {
        toml::from_str(&fs::read_to_string(root.join("Cargo.lock")).unwrap()).unwrap()
    };
    let selected = read(fixture);
    let authoritative = read(workspace);
    for package in selected["package"].as_array().unwrap() {
        if package.get("source").is_none() {
            continue;
        }
        assert!(
            authoritative["package"]
                .as_array()
                .unwrap()
                .iter()
                .any(|expected| {
                    ["name", "version", "source", "checksum"]
                        .iter()
                        .all(|key| package.get(key) == expected.get(key))
                }),
            "fixture changed locked dependency identity: {}",
            package["name"]
        );
    }
}

fn prepare_fixture(workspace: &Path, root: &Path) {
    fs::create_dir_all(root.join("src/metrics_support")).unwrap();
    let catalog: toml::Value =
        toml::from_str(&fs::read_to_string(workspace.join("Cargo.toml")).unwrap()).unwrap();
    let mut dependencies = toml::map::Map::new();
    for name in ["canic", "canic-icydb"] {
        dependencies.insert(
            name.into(),
            toml::Value::Table(toml::map::Map::from_iter([(
                "path".into(),
                toml::Value::String(workspace.join("crates").join(name).display().to_string()),
            )])),
        );
    }
    let mut serde = catalog["workspace"]["dependencies"]["serde"].clone();
    serde.as_table_mut().unwrap().insert(
        "features".into(),
        toml::Value::Array(vec![toml::Value::String("derive".into())]),
    );
    dependencies.insert("serde".into(), serde);
    dependencies.insert(
        "ic-metrics".into(),
        catalog["workspace"]["dependencies"]["ic-metrics"].clone(),
    );
    dependencies.insert(
        "ic-wire".into(),
        catalog["workspace"]["dependencies"]["ic-wire"].clone(),
    );
    let mut manifest: toml::Value = toml::from_str(
        r#"
[package]
name = "adapter-metrics-config-fixture"
version = "0.0.0"
edition = "2024"
publish = false
[package.metadata.canic]
app = "standalone"
role = "probe"
[workspace]
"#,
    )
    .unwrap();
    let build_dependencies =
        toml::map::Map::from_iter([("canic".into(), dependencies["canic"].clone())]);
    let table = manifest.as_table_mut().unwrap();
    table.insert("dependencies".into(), toml::Value::Table(dependencies));
    table.insert(
        "build-dependencies".into(),
        toml::Value::Table(build_dependencies),
    );
    table.insert("profile".into(), catalog["profile"].clone());
    fs::write(root.join("Cargo.toml"), toml::to_string(&manifest).unwrap()).unwrap();
    fs::copy(workspace.join("Cargo.lock"), root.join("Cargo.lock")).unwrap();
    fs::copy(
        workspace.join("crates/canic-icydb/tests/metrics_support/mod.rs"),
        root.join("src/metrics_support/mod.rs"),
    )
    .unwrap();
    fs::write(
        root.join("build.rs"),
        "fn main() { canic::build!(\"canic.toml\"); }\n",
    )
    .unwrap();
}

#[test]
fn managed_metrics_config_controls_read_and_reset_exports_for_both_policies() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let root = workspace.join("target/adapter-metrics-config-fixture");
    prepare_fixture(workspace, &root);
    for policy in ["public", "controller"] {
        for (selection, mounted, expected_capability) in [
            ("false", true, false),
            ("true", true, true),
            ("default", true, true),
            ("true", false, true),
        ] {
            let declarations = if mounted {
                format!("icydb_metrics(authorization = {policy}); icydb_metrics_reset;")
            } else {
                String::new()
            };
            fs::write(
                root.join("src/lib.rs"),
                include_str!("adapter_metrics_support/consumer.rs")
                    .replace("METRICS_DECLARATIONS", &declarations)
                    .replace("METRICS_MOUNTED", if mounted { "all()" } else { "any()" })
                    .replace("EXPECTED_CAPABILITY", &expected_capability.to_string()),
            )
            .unwrap();
            let metrics = if selection == "default" {
                String::new()
            } else {
                format!("metrics = {selection}\n")
            };
            // History deliberately remains enabled: it must not re-enable reads/reset.
            fs::write(root.join("canic.toml"), format!(
                "[app]\nname = \"standalone\"\ninit_mode = \"enabled\"\n[roles.probe]\nkind = \"canister\"\npackage = \".\"\n[roles.probe.observability]\n{metrics}history = true\n[auth.delegated_tokens]\nenabled = false\n",
            )).unwrap();
            // Resolve only in this disposable fixture; production locks stay untouched.
            run(
                &root,
                &["metadata", "--offline", "--format-version", "1"],
                "metadata",
            );
            assert_locked_dependency_identity(workspace, &root);
            run(
                &root,
                &["test", "--offline", "--locked", "--lib"],
                &format!("{policy}-{selection}-mounted-{mounted}"),
            );
        }
    }
}
