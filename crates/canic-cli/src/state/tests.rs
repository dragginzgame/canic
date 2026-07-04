use super::*;
use crate::{CliError, cli_error_exit_code};

#[test]
fn state_command_exposes_audit_and_manifest_subcommands() {
    let parsed = parse_required_subcommand(
        state_command(),
        [OsString::from(AUDIT_COMMAND), OsString::from("--json")],
    )
    .expect("parse state audit");

    assert_eq!(parsed.0, AUDIT_COMMAND);
    assert_eq!(parsed.1, vec![OsString::from("--json")]);

    let help = usage();
    assert!(help.contains("canic state audit"));
    assert!(help.contains("canic state manifest"));
    assert!(help.contains("diagnostic-only"));
    assert!(help.contains("do not read stable"));
    assert!(help.contains("write generated files"));
    assert!(help.contains("mutate canisters"));
}

#[test]
fn parses_supported_audit_options() {
    let options = StateOptions::parse_audit([
        OsString::from("--role"),
        OsString::from("root"),
        OsString::from("--json"),
    ])
    .expect("parse audit options");

    assert_eq!(options.role.as_deref(), Some("root"));
    assert!(options.json);
}

#[test]
fn parses_supported_manifest_options() {
    let options = StateOptions::parse_manifest([
        OsString::from("--role"),
        OsString::from("root"),
        OsString::from("--json"),
    ])
    .expect("parse manifest options");

    assert_eq!(options.role.as_deref(), Some("root"));
    assert!(options.json);
}

#[test]
fn rejects_hard_cut_state_forms() {
    assert!(matches!(run([]), Err(StateCommandError::Usage(_))));
    for command in ["migrate", "repair", "explore", "dump"] {
        assert!(matches!(
            run([OsString::from(command)]),
            Err(StateCommandError::Usage(_))
        ));
        assert!(parse_required_subcommand(state_command(), [OsString::from(command)]).is_err());
    }

    for args in [
        vec![OsString::from("root")],
        vec![OsString::from("--force")],
        vec![OsString::from("--format"), OsString::from("json")],
        vec![OsString::from("--out"), OsString::from("state.json")],
    ] {
        assert!(matches!(
            StateOptions::parse_audit(args.clone()),
            Err(StateCommandError::Usage(_))
        ));
        assert!(matches!(
            StateOptions::parse_manifest(args),
            Err(StateCommandError::Usage(_))
        ));
    }

    for args in [
        vec![OsString::from(AUDIT_COMMAND), OsString::from("root")],
        vec![OsString::from(MANIFEST_COMMAND), OsString::from("root")],
        vec![
            OsString::from(AUDIT_COMMAND),
            OsString::from("--format"),
            OsString::from("json"),
        ],
        vec![
            OsString::from(MANIFEST_COMMAND),
            OsString::from("--format"),
            OsString::from("json"),
        ],
    ] {
        assert!(matches!(run(args), Err(StateCommandError::Usage(_))));
    }
}

#[test]
fn audit_json_uses_schema_version_one() {
    let report = build_state_audit_report(Some("root"));
    let json = serde_json::to_value(&report).expect("state audit report serializes");

    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["command"], "canic state audit");
    assert_eq!(json["scope"], "role");
    assert_eq!(json["role"], "root");
    assert_eq!(json["status"], "warn");
    assert!(
        json["checks"]
            .as_array()
            .expect("checks")
            .iter()
            .any(|check| check["code"] == "reserved_memory_id_declared")
    );
    assert!(
        json["checks"]
            .as_array()
            .expect("checks")
            .iter()
            .any(|check| check["code"] == "reserved_export_import_ok")
    );
}

#[test]
fn manifest_json_is_manifest_directly() {
    let manifest = declared_state_manifest(Some("root"));
    let json = serde_json::to_value(&manifest).expect("state manifest serializes");

    assert_eq!(json["schema_version"], 1);
    assert!(json.get("command").is_none());
    assert_eq!(json["roles"][0]["canister_role"], "root");
    assert!(
        json["roles"][0]["reserved_memory"]
            .as_array()
            .expect("reserved memory")
            .iter()
            .any(|entry| entry["label"] == "log_index")
    );
}

#[test]
fn text_renderers_include_stable_fields() {
    let report = build_state_audit_report(Some("root"));
    let audit = render_audit_text(&report);
    let manifest = render_manifest_text(&declared_state_manifest(Some("root")));

    assert!(audit.contains("schema_version: 1"));
    assert!(audit.contains("scope: role"));
    assert!(audit.contains("memory_id [warn] reserved_memory_id_declared"));
    assert!(audit.contains("source: state_manifest"));
    assert!(manifest.contains("canic state manifest"));
    assert!(manifest.contains("migration_policy: new_domain"));
    assert!(manifest.contains("template_manifests"));
    assert!(manifest.contains("removed_state"));
    assert!(manifest.contains("reserved_memory"));
    assert!(manifest.contains("log_index"));
    assert_eq!(storage_label(StateStorage::NotApplicable), "not_applicable");
}

#[test]
fn failing_state_audit_uses_exit_code_one() {
    let err = run_audit(vec![OsString::from("--role"), OsString::from("missing")])
        .expect_err("missing role fails audit");

    assert!(matches!(err, StateCommandError::AuditFailed));
    assert_eq!(
        cli_error_exit_code(&CliError::State(err)),
        i32::from(StateCommandError::AuditFailed.exit_code())
    );
}
