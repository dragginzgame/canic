use std::{
    fmt::Write as _,
    fs,
    ops::Deref,
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

const REQUIRED_COMMON_GATEWAY_METHOD_FIELDS: &[&str] = &[
    "Source repository or local source identifier",
    "Source commit SHA",
    "Source file path",
    "Mode",
    "Candid signature",
    "Request DTO shape",
    "Response DTO shape",
    "Unauthorized behavior",
    "Production-vs-local differences",
];

const TOKO_SOURCE_COMMIT_SHA: &str = "abcdef0123456789abcdef0123456789abcdef01";

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

struct TempWorkspace {
    root: PathBuf,
}

impl TempWorkspace {
    fn new(name: &str) -> Self {
        let root = unique_temp_repo(name);
        fs::create_dir_all(&root).expect("temp workspace should be created");
        write_file(&root, "Cargo.toml", "[workspace]\n");
        fs::create_dir_all(root.join("crates")).expect("crates directory should be created");
        fs::create_dir_all(root.join("canisters")).expect("canisters directory should be created");
        fs::create_dir_all(root.join("apps")).expect("apps directory should be created");
        Self { root }
    }
}

impl Deref for TempWorkspace {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.root
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn create_temp_workspace(name: &str) -> TempWorkspace {
    TempWorkspace::new(name)
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

fn run_gate_without_ripgrep(root: &Path, inventory: &Path) -> Output {
    let script = workspace_root().join("scripts/ci/check-blob-storage-inventory-gate.sh");
    Command::new("/bin/bash")
        .arg(script)
        .current_dir(root)
        .env("BLOB_STORAGE_INVENTORY", inventory)
        .env("PATH", "")
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

fn run_billing_gate_without_ripgrep(root: &Path, inventory: &Path) -> Output {
    let script = workspace_root().join(format!(
        "scripts/ci/check-blob-storage-{}{}-inventory-gate.sh",
        "ca", "shier"
    ));
    Command::new("/bin/bash")
        .arg(script)
        .current_dir(root)
        .env("BLOB_STORAGE_CASHIER_INVENTORY", inventory)
        .env("PATH", "")
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

fn gateway_method_name(suffix: &str) -> String {
    format!("_immutableObject{}rage{suffix}", "Sto")
}

fn blob_storage_feature_name() -> String {
    format!("blob-{}", "storage")
}

fn complete_inventory_with_toko_section(toko_section: &str) -> String {
    let mut inventory =
        "# Blob Storage Gateway Protocol Inventory\n\nStatus: **Complete**\n".to_string();
    for suffix in REQUIRED_METHODS {
        let method = gateway_method_name(suffix);
        let method_section = complete_gateway_method_section(suffix);
        write!(&mut inventory, "\n### `{method}`\n\n{method_section}")
            .expect("writing to String should not fail");
    }
    inventory.push_str("\n## Interoperability Notes\n\n### Toko\n\n");
    inventory.push_str(toko_section);
    inventory
}

fn complete_inventory_missing_method(omitted_suffix: &str) -> String {
    let mut inventory =
        "# Blob Storage Gateway Protocol Inventory\n\nStatus: **Complete**\n".to_string();
    for suffix in REQUIRED_METHODS {
        if *suffix == omitted_suffix {
            continue;
        }
        let method = gateway_method_name(suffix);
        let method_section = complete_gateway_method_section(suffix);
        write!(&mut inventory, "\n### `{method}`\n\n{method_section}")
            .expect("writing to String should not fail");
    }
    inventory.push_str("\n## Interoperability Notes\n\n### Toko\n\n");
    inventory.push_str(&complete_toko_section());
    inventory
}

fn complete_inventory_without_toko_section() -> String {
    let mut inventory =
        "# Blob Storage Gateway Protocol Inventory\n\nStatus: **Complete**\n".to_string();
    for suffix in REQUIRED_METHODS {
        let method = gateway_method_name(suffix);
        let method_section = complete_gateway_method_section(suffix);
        write!(&mut inventory, "\n### `{method}`\n\n{method_section}")
            .expect("writing to String should not fail");
    }
    inventory
}

fn complete_inventory_with_method_section(method_suffix: &str, method_section: &str) -> String {
    let mut inventory =
        "# Blob Storage Gateway Protocol Inventory\n\nStatus: **Complete**\n".to_string();
    let target_method = gateway_method_name(method_suffix);
    for suffix in REQUIRED_METHODS {
        let method = gateway_method_name(suffix);
        if method == target_method {
            write!(&mut inventory, "\n### `{method}`\n\n{method_section}\n")
                .expect("writing to String should not fail");
        } else {
            let method_section = complete_gateway_method_section(suffix);
            write!(&mut inventory, "\n### `{method}`\n\n{method_section}")
                .expect("writing to String should not fail");
        }
    }
    inventory.push_str("\n## Interoperability Notes\n\n### Toko\n\n");
    inventory.push_str(&complete_toko_section());
    inventory
}

fn complete_toko_section() -> String {
    let blob_root_hash = format!("{}{}{}", "Blob", "Root", "Hash");
    format!(
        "\
Status: **Complete**

- Local source identifier: sibling checkout ../toko
- Source commit SHA: {TOKO_SOURCE_COMMIT_SHA}
- Mapping from Toko blob identity into Canic `{blob_root_hash}`: accepted empty-state adoption path
- Migration/read-through strategy: no existing state migration required for this release
"
    )
}

fn complete_gateway_method_section(method_suffix: &str) -> String {
    let mut section = "Status: **Complete**\n".to_string();
    for field in REQUIRED_COMMON_GATEWAY_METHOD_FIELDS {
        write_gateway_field(&mut section, field);
    }
    write_gateway_method_specific_fields(&mut section, method_suffix);
    section
}

fn write_gateway_field(section: &mut String, field: &str) {
    writeln!(section, "\n- {field}: {}", gateway_field_value(field))
        .expect("writing to String should not fail");
}

fn gateway_field_value(field: &str) -> &'static str {
    match field {
        "Source repository or local source identifier" => "https://example.invalid/gateway",
        "Source commit SHA" => "0123456789abcdef0123456789abcdef01234567",
        "Source file path" => "src/gateway.rs",
        "Mode" => "query",
        "Candid signature" => "(vec blob) -> (vec bool) query",
        "Request DTO shape" => "record { hashes : vec blob }",
        "Response DTO shape" => "vec bool",
        "Production-vs-local differences" => "no behavior differences recorded",
        _ => "captured behavior from upstream source",
    }
}

fn write_gateway_method_specific_fields(section: &mut String, method_suffix: &str) {
    match method_suffix {
        "BlobsAreLive" => write_gateway_fields(
            section,
            &[
                "Malformed input behavior",
                "Batch ordering semantics",
                "Duplicate-input semantics",
                "Absent-hash behavior",
                "Maximum batch size",
            ],
        ),
        "BlobsToDelete" => write_gateway_fields(
            section,
            &[
                "Result ordering",
                "Maximum batch size",
                "Repeat-return behavior until confirmation",
                "Empty pending-deletion behavior",
            ],
        ),
        "ConfirmBlobDeletion" => write_gateway_fields(
            section,
            &[
                "Unknown blob behavior",
                "Live-but-not-pending behavior",
                "Already-confirmed behavior",
                "Idempotency semantics",
            ],
        ),
        "CreateCertificate" => write_gateway_fields(
            section,
            &[
                "Certificate material source",
                "Mutation-before-certificate behavior",
                "Rollback or no-rollback behavior",
                "Repeated create behavior",
                "Metadata conflict/enrichment behavior",
                "Malformed request behavior",
            ],
        ),
        "UpdateGatewayPrincipals" => {
            write_gateway_field(section, &format!("{}{} dependency", "Ca", "shier"));
        }
        "FundFromProjectCycles" => write_gateway_fields(
            section,
            &[
                "Cycle attachment requirements",
                "Funding success/failure behavior",
            ],
        ),
        _ => panic!("unknown gateway method suffix"),
    }
}

fn write_gateway_fields(section: &mut String, fields: &[&str]) {
    for field in fields {
        write_gateway_field(section, field);
    }
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

fn billing_feature_name() -> String {
    format!("blob-storage-{}{}", "bill", "ing")
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
        let method_section = complete_billing_method_section(&method);
        write!(&mut inventory, "\n### `{method}`\n\n{method_section}\n")
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

fn complete_billing_method_section(method: &str) -> String {
    let mut section = "Status: **Complete**\n".to_string();
    if method == billing_method_name("gateway-principal-list") {
        section.push_str(
            "\n- Empty-list behavior: empty lists are malformed and preserve previous gateway state\n",
        );
        for field in [
            "Duplicate-principal behavior",
            "Anonymous-principal behavior",
            "Management-canister-principal behavior",
            "Malformed response behavior expected from Canic wrappers",
        ] {
            writeln!(
                &mut section,
                "\n- {field}: captured behavior from upstream source"
            )
            .expect("writing to String should not fail");
        }
    }
    section
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
        let method_section = complete_billing_method_section(&method);
        write!(&mut inventory, "\n### `{method}`\n\n{method_section}\n")
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

fn complete_billing_inventory_with_method_section(
    method_name: &str,
    method_section: &str,
) -> String {
    let mut inventory = format!(
        "# Blob Storage {} Protocol Inventory\n\nStatus: **Complete**\n",
        billing_protocol_name()
    );
    for method in REQUIRED_BILLING_METHODS {
        let method = billing_method_name(method);
        if method == method_name {
            write!(&mut inventory, "\n### `{method}`\n\n{method_section}\n")
                .expect("writing to String should not fail");
        } else {
            let method_section = complete_billing_method_section(&method);
            write!(&mut inventory, "\n### `{method}`\n\n{method_section}\n")
                .expect("writing to String should not fail");
        }
    }
    write!(
        &mut inventory,
        "\n## Optional {} Methods\n\nStatus: **Complete**\n\n- None required.\n",
        billing_protocol_name()
    )
    .expect("writing to String should not fail");
    inventory
}

struct IncompleteGateRejectionCase {
    name: &'static str,
    setup: fn(&Path),
    expected_text: &'static str,
}

fn setup_gateway_feature_metadata(root: &Path) {
    write_file(
        root,
        "crates/example/Cargo.toml",
        &format!("[features]\n{} = []\n", blob_storage_feature_name()),
    );
}

fn setup_gateway_source_path(root: &Path) {
    write_file(
        root,
        &format!("crates/example/src/{}_{}_client/mod.rs", "blob", "storage"),
        "pub fn marker() {}\n",
    );
}

fn setup_gateway_method_literal(root: &Path) {
    write_file(
        root,
        "crates/example/src/lib.rs",
        &format!(
            "pub const METHOD: &str = {:?};\n",
            gateway_method_name(REQUIRED_METHODS[0])
        ),
    );
}

fn setup_gateway_public_api(root: &Path) {
    write_file(
        root,
        "crates/example/src/lib.rs",
        &format!("pub struct {}{}Api;\n", "Blob", "Storage"),
    );
}

fn setup_gateway_billing_surface(root: &Path) {
    write_file(
        root,
        "crates/example/src/lib.rs",
        &format!(
            "pub struct BillingClient;\npub fn get_{}{}() {{}}\n",
            "blob_storage_", "status"
        ),
    );
}

fn setup_billing_method_literal(root: &Path) {
    write_file(
        root,
        "crates/example/src/lib.rs",
        &format!(
            "pub const METHOD: &str = {:?};\n",
            billing_method_name(REQUIRED_BILLING_METHODS[0])
        ),
    );
}

fn setup_billing_endpoint_literal(root: &Path) {
    write_file(
        root,
        "crates/example/src/lib.rs",
        &format!("pub fn get_{}{}() {{}}\n", "blob_storage_", "status"),
    );
}

fn setup_billing_feature_metadata(root: &Path) {
    write_file(
        root,
        "crates/example/Cargo.toml",
        &format!("[features]\n{} = []\n", billing_feature_name()),
    );
}

fn setup_billing_source_path(root: &Path) {
    write_file(
        root,
        &format!("crates/example/src/{}{}_client/mod.rs", "ca", "shier"),
        "pub fn marker() {}\n",
    );
}

fn setup_billing_public_type(root: &Path) {
    write_file(
        root,
        "crates/example/src/lib.rs",
        &format!("pub struct {}{}{}Config;\n", "Blob", "Storage", "Billing"),
    );
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
}

#[test]
fn incomplete_inventory_missing_ripgrep_reports_setup_action() {
    let root = create_temp_workspace("blob-gate-missing-rg");
    let inventory = root.join("BLOB_STORAGE_INVENTORY.md");
    fs::write(&inventory, incomplete_inventory()).expect("inventory should be written");

    let output = run_gate_without_ripgrep(&root, &inventory);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "gate should fail with a setup hint when ripgrep is unavailable"
    );
    assert!(text.contains("missing required tool: rg"));
    assert!(text.contains("make install-dev"));
    assert!(text.contains("make update-dev"));
    assert!(!text.contains("command not found"));
}

#[test]
fn incomplete_inventory_rejects_forbidden_gateway_surfaces() {
    for case in [
        IncompleteGateRejectionCase {
            name: "feature",
            setup: setup_gateway_feature_metadata,
            expected_text: "feature or dependency metadata",
        },
        IncompleteGateRejectionCase {
            name: "path",
            setup: setup_gateway_source_path,
            expected_text: "source/module path",
        },
        IncompleteGateRejectionCase {
            name: "method",
            setup: setup_gateway_method_literal,
            expected_text: "gateway method literal",
        },
        IncompleteGateRejectionCase {
            name: "public-api",
            setup: setup_gateway_public_api,
            expected_text: "internal blob-storage API/model type",
        },
        IncompleteGateRejectionCase {
            name: "billing",
            setup: setup_gateway_billing_surface,
            expected_text: "implementation surface",
        },
    ] {
        let root = create_temp_workspace(&format!("blob-gate-{}", case.name));
        let inventory = root.join("BLOB_STORAGE_INVENTORY.md");
        fs::write(&inventory, incomplete_inventory()).expect("inventory should be written");
        (case.setup)(&root);

        let output = run_gate(&root, &inventory);
        let text = output_text(&output);

        assert!(
            !output.status.success(),
            "gate should reject {} while inventory is incomplete",
            case.name
        );
        assert!(
            text.contains(case.expected_text),
            "expected output to contain {:?}, got:\n{text}",
            case.expected_text
        );
    }
}

#[test]
fn complete_inventory_rejects_incomplete_method_status() {
    let root = create_temp_workspace("blob-gate-method-status");
    let inventory = root.join("BLOB_STORAGE_INVENTORY.md");
    fs::write(
        &inventory,
        complete_inventory_with_method_section(REQUIRED_METHODS[0], "Status: **Captured**\n"),
    )
    .expect("inventory should be written");

    let output = run_gate(&root, &inventory);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "gate should reject a required gateway method that is not complete"
    );
    assert!(text.contains("method is not complete"));
}

#[test]
fn complete_inventory_rejects_unresolved_method_fields() {
    let root = create_temp_workspace("blob-gate-method-tbd");
    let inventory = root.join("BLOB_STORAGE_INVENTORY.md");
    fs::write(
        &inventory,
        complete_inventory_with_method_section(
            REQUIRED_METHODS[1],
            "Status: **Complete**\n\n- Deletion queue behavior: TBD\n",
        ),
    )
    .expect("inventory should be written");

    let output = run_gate(&root, &inventory);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "gate should reject unresolved required gateway method fields"
    );
    assert!(text.contains("method still has TBD fields"));
}

#[test]
fn complete_inventory_rejects_placeholder_method_evidence() {
    let root = create_temp_workspace("blob-gate-method-placeholder");
    let inventory = root.join("BLOB_STORAGE_INVENTORY.md");
    let method_section = complete_gateway_method_section(REQUIRED_METHODS[0]).replace(
        "- Source file path: src/gateway.rs",
        "- Source file path: placeholder",
    );
    fs::write(
        &inventory,
        complete_inventory_with_method_section(REQUIRED_METHODS[0], &method_section),
    )
    .expect("inventory should be written");

    let output = run_gate(&root, &inventory);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "gate should reject placeholder method evidence"
    );
    assert!(text.contains("method still has placeholder evidence"));
}

#[test]
fn complete_inventory_rejects_invalid_source_commit_sha() {
    let root = create_temp_workspace("blob-gate-method-sha");
    let inventory = root.join("BLOB_STORAGE_INVENTORY.md");
    let method_section = complete_gateway_method_section(REQUIRED_METHODS[0]).replace(
        "- Source commit SHA: 0123456789abcdef0123456789abcdef01234567",
        "- Source commit SHA: not-a-sha",
    );
    fs::write(
        &inventory,
        complete_inventory_with_method_section(REQUIRED_METHODS[0], &method_section),
    )
    .expect("inventory should be written");

    let output = run_gate(&root, &inventory);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "gate should reject invalid method source commit SHA"
    );
    assert!(text.contains("method has invalid source commit SHA"));
}

#[test]
fn complete_inventory_rejects_skeletal_method_section() {
    let root = create_temp_workspace("blob-gate-method-skeletal");
    let inventory = root.join("BLOB_STORAGE_INVENTORY.md");
    fs::write(
        &inventory,
        complete_inventory_with_method_section(REQUIRED_METHODS[3], "Status: **Complete**\n"),
    )
    .expect("inventory should be written");

    let output = run_gate(&root, &inventory);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "gate should reject method sections without required evidence fields"
    );
    assert!(text.contains("method missing required field"));
}

#[test]
fn complete_inventory_rejects_missing_method_section() {
    let root = create_temp_workspace("blob-gate-missing-method");
    let inventory = root.join("BLOB_STORAGE_INVENTORY.md");
    fs::write(
        &inventory,
        complete_inventory_missing_method(REQUIRED_METHODS[2]),
    )
    .expect("inventory should be written");

    let output = run_gate(&root, &inventory);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "gate should reject a Complete inventory missing a gateway method"
    );
    assert!(text.contains("missing method section"));
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
    assert!(text.contains("Toko interoperability notes still have TBD fields"));
}

#[test]
fn complete_inventory_rejects_missing_toko_evidence_fields() {
    let root = create_temp_workspace("blob-gate-complete-toko-missing-field");
    let inventory = root.join("BLOB_STORAGE_INVENTORY.md");
    fs::write(
        &inventory,
        complete_inventory_with_toko_section("Status: **Complete**\n\n- Mapping accepted.\n"),
    )
    .expect("inventory should be written");

    let output = run_gate(&root, &inventory);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "gate should reject Toko interoperability notes without required evidence fields"
    );
    assert!(text.contains("Toko interoperability notes missing required field"));
}

#[test]
fn complete_inventory_rejects_invalid_toko_source_commit_sha() {
    let root = create_temp_workspace("blob-gate-complete-toko-sha");
    let inventory = root.join("BLOB_STORAGE_INVENTORY.md");
    let toko_section = complete_toko_section().replace(
        &format!("- Source commit SHA: {TOKO_SOURCE_COMMIT_SHA}"),
        "- Source commit SHA: not-a-sha",
    );
    fs::write(
        &inventory,
        complete_inventory_with_toko_section(&toko_section),
    )
    .expect("inventory should be written");

    let output = run_gate(&root, &inventory);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "gate should reject invalid Toko source commit SHA"
    );
    assert!(text.contains("Toko interoperability notes have invalid source commit SHA"));
}

#[test]
fn complete_inventory_rejects_missing_toko_section() {
    let root = create_temp_workspace("blob-gate-missing-toko");
    let inventory = root.join("BLOB_STORAGE_INVENTORY.md");
    fs::write(&inventory, complete_inventory_without_toko_section())
        .expect("inventory should be written");

    let output = run_gate(&root, &inventory);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "gate should reject a Complete inventory without Toko interoperability notes"
    );
    assert!(text.contains("missing Toko interoperability section"));
}

#[test]
fn complete_inventory_allows_resolved_inventory() {
    let root = create_temp_workspace("blob-gate-complete");
    let inventory = root.join("BLOB_STORAGE_INVENTORY.md");
    fs::write(
        &inventory,
        complete_inventory_with_toko_section(&complete_toko_section()),
    )
    .expect("inventory should be written");

    let output = run_gate(&root, &inventory);

    assert!(
        output.status.success(),
        "gate should accept a resolved Complete gateway inventory\n{}",
        output_text(&output)
    );
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
}

#[test]
fn incomplete_billing_inventory_missing_ripgrep_reports_setup_action() {
    let root = create_temp_workspace("billing-gate-missing-rg");
    let inventory = root.join("BLOB_STORAGE_CASHIER_INVENTORY.md");
    fs::write(&inventory, incomplete_billing_inventory()).expect("inventory should be written");

    let output = run_billing_gate_without_ripgrep(&root, &inventory);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "billing gate should fail with a setup hint when ripgrep is unavailable"
    );
    assert!(text.contains("missing required tool: rg"));
    assert!(text.contains("make install-dev"));
    assert!(text.contains("make update-dev"));
    assert!(!text.contains("command not found"));
}

#[test]
fn incomplete_billing_inventory_rejects_forbidden_billing_surfaces() {
    for case in [
        IncompleteGateRejectionCase {
            name: "method",
            setup: setup_billing_method_literal,
            expected_text: "implementation surface",
        },
        IncompleteGateRejectionCase {
            name: "endpoint",
            setup: setup_billing_endpoint_literal,
            expected_text: "billing endpoint literal",
        },
        IncompleteGateRejectionCase {
            name: "feature",
            setup: setup_billing_feature_metadata,
            expected_text: "feature or dependency metadata",
        },
        IncompleteGateRejectionCase {
            name: "path",
            setup: setup_billing_source_path,
            expected_text: "source/module path",
        },
        IncompleteGateRejectionCase {
            name: "public-type",
            setup: setup_billing_public_type,
            expected_text: "public",
        },
    ] {
        let root = create_temp_workspace(&format!("billing-gate-{}", case.name));
        let inventory = root.join("BLOB_STORAGE_CASHIER_INVENTORY.md");
        fs::write(&inventory, incomplete_billing_inventory()).expect("inventory should be written");
        (case.setup)(&root);

        let output = run_billing_gate(&root, &inventory);
        let text = output_text(&output);

        assert!(
            !output.status.success(),
            "billing gate should reject {} while inventory is incomplete",
            case.name
        );
        assert!(
            text.contains(case.expected_text),
            "expected output to contain {:?}, got:\n{text}",
            case.expected_text
        );
    }
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
}

#[test]
fn complete_billing_inventory_rejects_incomplete_method_status() {
    let root = create_temp_workspace("billing-gate-method-status");
    let inventory = root.join("BLOB_STORAGE_CASHIER_INVENTORY.md");
    let method = billing_method_name(REQUIRED_BILLING_METHODS[0]);
    fs::write(
        &inventory,
        complete_billing_inventory_with_method_section(&method, "Status: **Snapshot captured**\n"),
    )
    .expect("inventory should be written");

    let output = run_billing_gate(&root, &inventory);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "billing gate should reject a required method that is not complete"
    );
    assert!(text.contains("method is not complete"));
}

#[test]
fn complete_billing_inventory_rejects_unresolved_method_fields() {
    let root = create_temp_workspace("billing-gate-method-tbd");
    let inventory = root.join("BLOB_STORAGE_CASHIER_INVENTORY.md");
    let method = billing_method_name(REQUIRED_BILLING_METHODS[2]);
    fs::write(
        &inventory,
        complete_billing_inventory_with_method_section(
            &method,
            "Status: **Complete**\n\n- Empty-list behavior: TBD\n",
        ),
    )
    .expect("inventory should be written");

    let output = run_billing_gate(&root, &inventory);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "billing gate should reject unresolved required-method fields"
    );
    assert!(text.contains("method still has TBD fields"));
}

#[test]
fn complete_billing_inventory_rejects_missing_gateway_list_behavior_fields() {
    let root = create_temp_workspace("billing-gate-gateway-list-fields");
    let inventory = root.join("BLOB_STORAGE_CASHIER_INVENTORY.md");
    let method = billing_method_name("gateway-principal-list");
    fs::write(
        &inventory,
        complete_billing_inventory_with_method_section(&method, "Status: **Complete**\n"),
    )
    .expect("inventory should be written");

    let output = run_billing_gate(&root, &inventory);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "billing gate should reject gateway-list sections without behavior fields"
    );
    assert!(text.contains("method missing required field"));
    assert!(text.contains("Empty-list behavior"));
}

#[test]
fn complete_billing_inventory_rejects_unsafe_empty_gateway_list_behavior() {
    let root = create_temp_workspace("billing-gate-empty-list-contract");
    let inventory = root.join("BLOB_STORAGE_CASHIER_INVENTORY.md");
    let method = billing_method_name("gateway-principal-list");
    let method_section = complete_billing_method_section(&method).replace(
        "empty lists are malformed and preserve previous gateway state",
        "empty lists are accepted and replace the previous gateway set",
    );
    fs::write(
        &inventory,
        complete_billing_inventory_with_method_section(&method, &method_section),
    )
    .expect("inventory should be written");

    let output = run_billing_gate(&root, &inventory);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "billing gate should reject unsafe empty gateway-list behavior"
    );
    assert!(text.contains("invalid empty-list behavior"));
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
}
