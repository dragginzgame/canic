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
    process::Command,
};

const RELEASED_BASELINE_COMMIT: &str = "8cf4723cecd7579cbe3304b980c63b1bc3969d68";
const REASON_LEDGER_PATH: &str = "crates/canic-host/diagnostics/reasons.toml";

#[derive(Clone, Debug, Eq, PartialEq)]
struct Reason {
    code: u16,
    name: String,
    origin: String,
    summary: String,
    guidance: Option<String>,
    retired: bool,
}

fn released_reasons() -> Vec<Reason> {
    let object = format!("{RELEASED_BASELINE_COMMIT}:{REASON_LEDGER_PATH}");
    let output = Command::new("git")
        .args(["show", object.as_str()])
        .current_dir(workspace_root())
        .output()
        .expect("git must be available for the repository release guard");
    assert!(
        output.status.success(),
        "immutable diagnostic baseline {object} is unavailable: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let source = String::from_utf8(output.stdout).expect("released reason ledger must be UTF-8");
    parse_reasons(&source)
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
    let source = read(&workspace_root().join(REASON_LEDGER_PATH));
    parse_reasons(&source)
}

fn parse_reasons(source: &str) -> Vec<Reason> {
    let document = toml::from_str::<toml::Table>(source).expect("valid reason ledger TOML");
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

fn validate_released_reasons(released: &[Reason], current: &[Reason]) -> Result<(), String> {
    let current_by_code = current
        .iter()
        .map(|reason| (reason.code, reason))
        .collect::<BTreeMap<_, _>>();
    let current_by_name = current
        .iter()
        .map(|reason| (reason.name.as_str(), reason))
        .collect::<BTreeMap<_, _>>();

    for released_reason in released {
        let current_at_code = current_by_code.get(&released_reason.code).ok_or_else(|| {
            format!(
                "released diagnostic E{} {} was deleted",
                released_reason.code, released_reason.name
            )
        })?;
        if current_at_code.name != released_reason.name {
            return Err(format!(
                "released diagnostic E{} changed name from {} to {}",
                released_reason.code, released_reason.name, current_at_code.name
            ));
        }

        let current_at_name = current_by_name
            .get(released_reason.name.as_str())
            .ok_or_else(|| format!("released diagnostic {} was deleted", released_reason.name))?;
        if current_at_name.code != released_reason.code {
            return Err(format!(
                "released diagnostic {} changed code from E{} to E{}",
                released_reason.name, released_reason.code, current_at_name.code
            ));
        }
        if released_reason.retired && !current_at_code.retired {
            return Err(format!(
                "released retired diagnostic E{} {} became active",
                released_reason.code, released_reason.name
            ));
        }
    }

    Ok(())
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
    validate_released_reasons(&released_reasons(), &reasons())
        .expect("released diagnostic identities and retirement state must be preserved");
}

#[test]
fn released_identity_guard_rejects_rebinding_deletion_and_retirement_reversal() {
    let released = vec![
        fixture_reason(1, "ONE", false),
        fixture_reason(2, "TWO", true),
    ];
    let valid = vec![
        Reason {
            origin: "reviewed-origin".to_string(),
            summary: "Updated summary.".to_string(),
            guidance: Some("Updated guidance.".to_string()),
            retired: true,
            ..fixture_reason(1, "ONE", false)
        },
        fixture_reason(2, "TWO", true),
        fixture_reason(3, "THREE", false),
    ];
    validate_released_reasons(&released, &valid)
        .expect("presentation changes, retirement, and additions are allowed");

    for invalid in [
        vec![
            fixture_reason(1, "RENAMED", false),
            fixture_reason(2, "TWO", true),
        ],
        vec![
            fixture_reason(2, "ONE", false),
            fixture_reason(3, "TWO", true),
        ],
        vec![fixture_reason(1, "ONE", false)],
        vec![
            fixture_reason(1, "ONE", false),
            fixture_reason(2, "TWO", false),
        ],
    ] {
        assert!(
            validate_released_reasons(&released, &invalid).is_err(),
            "invalid released-identity transition was accepted: {invalid:?}"
        );
    }
}

fn fixture_reason(code: u16, name: &str, retired: bool) -> Reason {
    Reason {
        code,
        name: name.to_string(),
        origin: "origin".to_string(),
        summary: "Summary.".to_string(),
        guidance: None,
        retired,
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
