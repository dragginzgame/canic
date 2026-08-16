//! Test: reason_register
//!
//! Responsibility: keep released identities, the current reason ledger, runtime constants, and the host catalogue exact.
//! Does not own: producer mappings, public projection, or handling policy.
//! Boundary: repository-only generation evidence; none of this data enters canister Wasm.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
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

fn released_identities() -> BTreeMap<u16, String> {
    let source = read(&workspace_root().join("crates/canic-host/diagnostics/released.toml"));
    let document = toml::from_str::<toml::Table>(&source).expect("valid released ledger TOML");
    assert_eq!(
        document.len(),
        2,
        "released ledger owns only its version and code/name identities"
    );
    let release = document
        .get("release")
        .and_then(toml::Value::as_str)
        .expect("released ledger version");
    assert!(
        !release.is_empty(),
        "released ledger version must not be empty"
    );

    let mut identities = BTreeMap::new();
    for (name, code) in document
        .get("identity")
        .and_then(toml::Value::as_table)
        .expect("released identity table")
    {
        let code = code
            .as_integer()
            .and_then(|value| u16::try_from(value).ok())
            .unwrap_or_else(|| panic!("released diagnostic {name} must have a u16 code"));
        assert!(
            identities.insert(code, name.clone()).is_none(),
            "released diagnostic code E{code} must be unique"
        );
    }
    identities
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
    assert_eq!(document.len(), 1, "reason ledger owns only the reason rows");
    document
        .get("reason")
        .and_then(toml::Value::as_array)
        .expect("reason ledger rows")
        .iter()
        .map(|value| {
            let row = value.as_table().expect("reason row");
            let allowed_fields = ["code", "name", "origin", "summary", "guidance", "retired"];
            let unexpected_fields = row
                .keys()
                .filter(|field| !allowed_fields.contains(&field.as_str()))
                .collect::<Vec<_>>();
            assert!(
                unexpected_fields.is_empty(),
                "reason row has unsupported fields: {unexpected_fields:?}"
            );
            let code = row
                .get("code")
                .and_then(toml::Value::as_integer)
                .and_then(|value| u16::try_from(value).ok())
                .expect("u16 reason code");
            let string = |field: &str| {
                let value = row
                    .get(field)
                    .and_then(toml::Value::as_str)
                    .unwrap_or_else(|| panic!("reason E{code} missing {field}"));
                assert!(!value.trim().is_empty(), "reason E{code} has empty {field}");
                value.to_string()
            };
            Reason {
                code,
                name: string("name"),
                origin: string("origin"),
                summary: string("summary"),
                guidance: row.get("guidance").map(|value| {
                    let guidance = value
                        .as_str()
                        .unwrap_or_else(|| panic!("reason E{code} guidance must be a string"));
                    assert!(
                        !guidance.trim().is_empty(),
                        "reason E{code} has empty guidance"
                    );
                    guidance.to_string()
                }),
                retired: row
                    .get("retired")
                    .and_then(toml::Value::as_bool)
                    .unwrap_or_else(|| panic!("reason E{code} missing boolean retired")),
            }
        })
        .collect()
}

fn render_runtime_declarations(reasons: &[Reason]) -> String {
    let mut rendered = String::from("declare_diagnostic_codes! {\n");
    for reason in reasons.iter().filter(|reason| !reason.retired) {
        writeln!(rendered, "    {} = {};", reason.name, reason.code)
            .expect("writing to a String cannot fail");
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
        writeln!(
            rendered,
            "    DiagnosticEntry::new({}, {:?}, {:?}, {:?}, {}),",
            reason.code, reason.name, reason.origin, reason.summary, guidance,
        )
        .expect("writing to a String cannot fail");
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
            writeln!(
                rendered,
                "    super::RetiredDiagnosticEntry::new({}, {:?}),",
                reason.code, reason.name,
            )
            .expect("writing to a String cannot fail");
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
fn reason_ledger_is_unique_nonzero_and_sorted() {
    let reasons = reasons();
    let codes = reasons
        .iter()
        .map(|reason| reason.code)
        .collect::<BTreeSet<_>>();
    let names = reasons
        .iter()
        .map(|reason| reason.name.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(codes.len(), reasons.len());
    assert_eq!(names.len(), reasons.len());
    assert!(reasons.iter().all(|reason| reason.code != 0));
    assert!(
        reasons.windows(2).all(|pair| pair[0].code < pair[1].code),
        "reason ledger must stay sorted for static binary lookup"
    );
}

#[test]
fn released_code_and_name_identities_are_preserved() {
    let current = reasons()
        .into_iter()
        .map(|reason| (reason.code, reason.name))
        .collect::<BTreeMap<_, _>>();

    for (code, released_name) in released_identities() {
        assert_eq!(
            current.get(&code),
            Some(&released_name),
            "released diagnostic E{code} must retain its name"
        );
    }
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
