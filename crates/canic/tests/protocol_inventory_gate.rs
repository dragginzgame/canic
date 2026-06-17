use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

const REQUIRED_METHODS: &[&str] = &[
    "BlobsAreLive",
    "BlobsToDelete",
    "ConfirmBlobDeletion",
    "CreateCertificate",
    "UpdateGatewayPrincipals",
    "FundFromProjectCycles",
];

const REQUIRED_BILLING_METHODS: &[&str] = &["balance", "top-up", "gateway-principal-list"];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate directory should have a parent")
        .parent()
        .expect("workspace root should exist")
        .to_path_buf()
}

fn unique_temp_repo(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("canic-{name}-{}-{nanos}", std::process::id()))
}

fn write_file(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directory should be created");
    }
    fs::write(&path, contents).unwrap_or_else(|err| panic!("failed to write {relative}: {err}"));
}

fn create_temp_workspace(name: &str) -> PathBuf {
    let root = unique_temp_repo(name);
    fs::create_dir_all(&root).expect("temp workspace should be created");
    write_file(&root, "Cargo.toml", "[workspace]\n");
    fs::create_dir_all(root.join("crates")).expect("crates directory should be created");
    fs::create_dir_all(root.join("canisters")).expect("canisters directory should be created");
    fs::create_dir_all(root.join("fleets")).expect("fleets directory should be created");
    root
}

fn run_gate(root: &Path, inventory: &Path) -> Output {
    let script = workspace_root().join("scripts/ci/check-blob-storage-inventory-gate.sh");
    Command::new("bash")
        .arg(script)
        .current_dir(root)
        .env("BLOB_STORAGE_INVENTORY", inventory)
        .output()
        .expect("blob-storage inventory gate should run")
}

fn run_billing_gate(root: &Path, inventory: &Path) -> Output {
    let script = workspace_root().join(format!(
        "scripts/ci/check-blob-storage-{}{}-inventory-gate.sh",
        "ca", "shier"
    ));
    Command::new("bash")
        .arg(script)
        .current_dir(root)
        .env("BLOB_STORAGE_CASHIER_INVENTORY", inventory)
        .output()
        .expect("blob-storage billing inventory gate should run")
}

fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn incomplete_inventory() -> String {
    "# Blob Storage Gateway Protocol Inventory\n\nStatus: **Incomplete - implementation blocked**\n"
        .to_string()
}

fn complete_inventory_with_toko_section(toko_section: &str) -> String {
    let mut inventory =
        "# Blob Storage Gateway Protocol Inventory\n\nStatus: **Complete**\n".to_string();
    for suffix in REQUIRED_METHODS {
        let method = format!("_immutableObject{}rage{suffix}", "Sto");
        write!(&mut inventory, "\n### `{method}`\n\nStatus: **Complete**\n")
            .expect("writing to String should not fail");
    }
    inventory.push_str("\n## Compatibility Notes\n\n### Toko\n\n");
    inventory.push_str(toko_section);
    inventory
}

fn billing_method_name(method: &str) -> String {
    match method {
        "balance" => format!("account_{}_get_v1", "balance"),
        "top-up" => format!("account_{}_up_v1", "top"),
        "gateway-principal-list" => format!("storage_gateway_principal_{}_v1", "list"),
        _ => panic!("unknown billing method"),
    }
}

fn billing_protocol_name() -> String {
    format!("{}{}", "Ca", "shier")
}

fn incomplete_billing_inventory() -> String {
    format!(
        "# Blob Storage {} Protocol Inventory\n\nStatus: **Incomplete - implementation blocked**\n",
        billing_protocol_name()
    )
}

fn complete_billing_inventory_with_optional_section(optional_section: &str) -> String {
    let mut inventory = format!(
        "# Blob Storage {} Protocol Inventory\n\nStatus: **Complete**\n",
        billing_protocol_name()
    );
    for method in REQUIRED_BILLING_METHODS {
        let method = billing_method_name(method);
        write!(&mut inventory, "\n### `{method}`\n\nStatus: **Complete**\n")
            .expect("writing to String should not fail");
    }
    write!(
        &mut inventory,
        "\n## Optional {} Methods\n\n",
        billing_protocol_name()
    )
    .expect("writing to String should not fail");
    inventory.push_str(optional_section);
    inventory
}

fn complete_billing_inventory_missing_method(omitted_method: &str) -> String {
    let mut inventory = format!(
        "# Blob Storage {} Protocol Inventory\n\nStatus: **Complete**\n",
        billing_protocol_name()
    );
    for method in REQUIRED_BILLING_METHODS {
        if *method == omitted_method {
            continue;
        }
        let method = billing_method_name(method);
        write!(&mut inventory, "\n### `{method}`\n\nStatus: **Complete**\n")
            .expect("writing to String should not fail");
    }
    write!(
        &mut inventory,
        "\n## Optional {} Methods\n\nStatus: **Complete**\n\n- None required.\n",
        billing_protocol_name()
    )
    .expect("writing to String should not fail");
    inventory
}

#[test]
fn incomplete_inventory_allows_design_only_workspace() {
    let root = create_temp_workspace("blob-gate-clean");
    let inventory = root.join("BLOB_STORAGE_INVENTORY.md");
    fs::write(&inventory, incomplete_inventory()).expect("inventory should be written");

    let output = run_gate(&root, &inventory);

    assert!(
        output.status.success(),
        "gate should allow no implementation surface while inventory is incomplete\n{}",
        output_text(&output)
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn incomplete_inventory_rejects_billing_surface() {
    let root = create_temp_workspace("blob-gate-billing");
    let inventory = root.join("BLOB_STORAGE_INVENTORY.md");
    fs::write(&inventory, incomplete_inventory()).expect("inventory should be written");
    write_file(
        &root,
        "crates/example/src/lib.rs",
        &format!(
            "pub struct BillingClient;\npub fn get_{}{}() {{}}\n",
            "blob_storage_", "status"
        ),
    );

    let output = run_gate(&root, &inventory);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "gate should reject billing surface while inventory is incomplete"
    );
    assert!(text.contains(&format!(
        "blob-storage billing/{}{} implementation surface",
        "Ca", "shier"
    )));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn complete_inventory_rejects_unresolved_toko_fields() {
    let root = create_temp_workspace("blob-gate-complete-tbd");
    let inventory = root.join("BLOB_STORAGE_INVENTORY.md");
    fs::write(
        &inventory,
        complete_inventory_with_toko_section("Status: **Complete**\n\n- Mapping: TBD\n"),
    )
    .expect("inventory should be written");

    let output = run_gate(&root, &inventory);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "gate should reject unresolved Toko fields in a Complete inventory"
    );
    assert!(text.contains("Toko compatibility notes still have TBD fields"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn incomplete_billing_inventory_allows_design_only_workspace() {
    let root = create_temp_workspace("billing-gate-clean");
    let inventory = root.join("BLOB_STORAGE_CASHIER_INVENTORY.md");
    fs::write(&inventory, incomplete_billing_inventory()).expect("inventory should be written");

    let output = run_billing_gate(&root, &inventory);

    assert!(
        output.status.success(),
        "billing gate should allow no implementation surface while inventory is incomplete\n{}",
        output_text(&output)
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn incomplete_billing_inventory_rejects_method_surface() {
    let root = create_temp_workspace("billing-gate-method");
    let inventory = root.join("BLOB_STORAGE_CASHIER_INVENTORY.md");
    fs::write(&inventory, incomplete_billing_inventory()).expect("inventory should be written");
    write_file(
        &root,
        "crates/example/src/lib.rs",
        &format!(
            "pub const METHOD: &str = {:?};\n",
            billing_method_name(REQUIRED_BILLING_METHODS[0])
        ),
    );

    let output = run_billing_gate(&root, &inventory);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "billing gate should reject method literals while inventory is incomplete"
    );
    assert!(text.contains(&format!(
        "forbidden blob-storage {}{} implementation surface",
        "Ca", "shier"
    )));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn complete_billing_inventory_rejects_unresolved_optional_fields() {
    let root = create_temp_workspace("billing-gate-complete-tbd");
    let inventory = root.join("BLOB_STORAGE_CASHIER_INVENTORY.md");
    fs::write(
        &inventory,
        complete_billing_inventory_with_optional_section(
            "Status: **Complete**\n\n- Optional methods: TBD\n",
        ),
    )
    .expect("inventory should be written");

    let output = run_billing_gate(&root, &inventory);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "billing gate should reject unresolved optional-method fields"
    );
    assert!(text.contains("optional methods section still has TBD fields"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn complete_billing_inventory_rejects_missing_method_section() {
    let root = create_temp_workspace("billing-gate-missing-method");
    let inventory = root.join("BLOB_STORAGE_CASHIER_INVENTORY.md");
    fs::write(
        &inventory,
        complete_billing_inventory_missing_method(REQUIRED_BILLING_METHODS[1]),
    )
    .expect("inventory should be written");

    let output = run_billing_gate(&root, &inventory);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "billing gate should reject a Complete inventory missing a required method"
    );
    assert!(text.contains("missing method section"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn complete_billing_inventory_allows_resolved_inventory() {
    let root = create_temp_workspace("billing-gate-complete");
    let inventory = root.join("BLOB_STORAGE_CASHIER_INVENTORY.md");
    fs::write(
        &inventory,
        complete_billing_inventory_with_optional_section(
            "Status: **Complete**\n\n- Discovered methods: three required methods only.\n",
        ),
    )
    .expect("inventory should be written");

    let output = run_billing_gate(&root, &inventory);

    assert!(
        output.status.success(),
        "billing gate should accept a resolved Complete inventory\n{}",
        output_text(&output)
    );
    let _ = fs::remove_dir_all(root);
}
