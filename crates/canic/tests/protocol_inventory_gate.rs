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
