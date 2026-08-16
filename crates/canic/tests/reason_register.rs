//! Test: reason_register
//!
//! Responsibility: keep the reviewed cause families, host ledger, runtime constants, and host catalogue exact.
//! Does not own: producer mappings, public projection, handling policy, or released-version comparison.
//! Boundary: repository-only generation evidence; none of this data enters canister Wasm.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
struct Reason {
    code: u16,
    name: String,
    origin: String,
    summary: String,
    guidance: Option<String>,
    retired: bool,
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn reasons() -> Vec<Reason> {
    let source = read(&workspace_root().join("crates/canic-host/diagnostics/reasons.toml"));
    let document = toml::from_str::<toml::Table>(&source).expect("valid reason ledger TOML");
    document
        .get("reason")
        .and_then(toml::Value::as_array)
        .expect("reason ledger rows")
        .iter()
        .map(|value| {
            let row = value.as_table().expect("reason row");
            let code = row
                .get("code")
                .and_then(toml::Value::as_integer)
                .and_then(|value| u16::try_from(value).ok())
                .expect("u16 reason code");
            let string = |field: &str| {
                row.get(field)
                    .and_then(toml::Value::as_str)
                    .unwrap_or_else(|| panic!("reason E{code} missing {field}"))
                    .to_string()
            };
            Reason {
                code,
                name: string("name"),
                origin: string("origin"),
                summary: string("summary"),
                guidance: row
                    .get("guidance")
                    .and_then(toml::Value::as_str)
                    .map(str::to_string),
                retired: row
                    .get("retired")
                    .and_then(toml::Value::as_bool)
                    .unwrap_or(false),
            }
        })
        .collect()
}

fn reviewed_names(qualification: &str) -> BTreeSet<String> {
    let source =
        read(&workspace_root().join("docs/audits/working/0.102-diagnostic-inventory/index.md"));
    let (_, remainder) = source
        .split_once("<!-- BEGIN SEMANTIC CAUSE REVIEW -->")
        .expect("semantic review start");
    let (table, _) = remainder
        .split_once("<!-- END SEMANTIC CAUSE REVIEW -->")
        .expect("semantic review end");

    table
        .lines()
        .filter(|line| line.starts_with("| `"))
        .filter_map(|line| {
            let cells = line
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect::<Vec<_>>();
            (cells.len() == 6 && cells[4] == format!("`{qualification}`"))
                .then(|| cells[1].trim_matches('`').to_string())
        })
        .collect()
}

fn render_runtime_declarations(reasons: &[Reason]) -> String {
    let mut rendered = String::from("declare_diagnostic_codes! {\n");
    for reason in reasons.iter().filter(|reason| !reason.retired) {
        rendered.push_str(&format!("    {} = {};\n", reason.name, reason.code));
    }
    rendered.push('}');
    rendered
}

fn render_host_catalogue(reasons: &[Reason]) -> String {
    let mut rendered = String::from(
        "//! Generated host diagnostic catalogue.\n//!\n//! Source: `crates/canic-host/diagnostics/reasons.toml`.\n//! Do not edit by hand.\n\nuse super::DiagnosticEntry;\n\n#[rustfmt::skip]\npub(super) const CURRENT_REASONS: &[DiagnosticEntry] = &[\n",
    );
    for reason in reasons.iter().filter(|reason| !reason.retired) {
        let guidance = reason
            .guidance
            .as_ref()
            .map_or_else(|| "None".to_string(), |value| format!("Some({value:?})"));
        rendered.push_str(&format!(
            "    DiagnosticEntry::new({}, {:?}, {:?}, {:?}, {}),\n",
            reason.code, reason.name, reason.origin, reason.summary, guidance,
        ));
    }
    rendered.push_str("];\n\n");
    let retired = reasons
        .iter()
        .filter(|reason| reason.retired)
        .collect::<Vec<_>>();
    if retired.is_empty() {
        rendered.push_str(
            "pub(super) const RETIRED_REASONS: &[super::RetiredDiagnosticEntry] = &[];\n",
        );
    } else {
        rendered
            .push_str("pub(super) const RETIRED_REASONS: &[super::RetiredDiagnosticEntry] = &[\n");
        for reason in retired {
            rendered.push_str(&format!(
                "    super::RetiredDiagnosticEntry::new({}, {:?}),\n",
                reason.code, reason.name,
            ));
        }
        rendered.push_str("];\n");
    }
    rendered
}

fn generated_runtime_block(source: &str) -> &str {
    let (_, remainder) = source
        .split_once("// BEGIN GENERATED DIAGNOSTIC DECLARATIONS\n")
        .expect("runtime declaration start");
    let (block, _) = remainder
        .split_once("\n// END GENERATED DIAGNOSTIC DECLARATIONS")
        .expect("runtime declaration end");
    block
}

#[test]
fn reviewed_register_is_unique_nonzero_and_exact() {
    let reasons = reasons();
    let current = reasons
        .iter()
        .filter(|reason| !reason.retired)
        .collect::<Vec<_>>();
    let codes = reasons
        .iter()
        .map(|reason| reason.code)
        .collect::<BTreeSet<_>>();
    let names = reasons
        .iter()
        .map(|reason| reason.name.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(reasons.len(), 161);
    assert_eq!(current.len(), 161);
    assert_eq!(codes.len(), reasons.len());
    assert_eq!(names.len(), reasons.len());
    assert!(reasons.iter().all(|reason| reason.code != 0));
    assert!(
        reasons.windows(2).all(|pair| pair[0].code < pair[1].code),
        "reason ledger must stay sorted for static binary lookup"
    );
    assert_eq!(
        names,
        reviewed_names("global")
            .iter()
            .map(String::as_str)
            .collect()
    );
    assert_eq!(reviewed_names("local").len(), 10);
}

#[test]
fn generated_runtime_and_host_catalogues_match_the_reason_ledger() {
    let root = workspace_root();
    let reasons = reasons();
    let runtime = read(&root.join("crates/canic-core/src/diagnostics/codes/mod.rs"));
    let host = read(&root.join("crates/canic-host/src/diagnostics/generated/mod.rs"));

    assert_eq!(
        generated_runtime_block(&runtime),
        render_runtime_declarations(&reasons)
    );
    assert_eq!(host, render_host_catalogue(&reasons));
}
