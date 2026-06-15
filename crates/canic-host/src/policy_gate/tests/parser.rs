use super::*;

#[test]
fn policy_parser_accepts_minimal_policy() {
    let policy = parse_ci_policy_v1(MINIMAL_POLICY).expect("parse policy");

    assert_eq!(policy.schema_version, 1);
    assert_eq!(
        policy.envelope.required_schema,
        "canic.evidence_envelope.v1"
    );
    assert_eq!(policy.exit_class.allowed, vec![ExitClassV1::Success]);
}

#[test]
fn policy_parser_rejects_unknown_keys_and_empty_allow_lists() {
    let unknown = parse_ci_policy_v1(
        r#"
schema_version = 1
unexpected = true

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]
"#,
    )
    .expect_err("unknown policy keys fail");
    assert!(unknown.to_string().contains("failed to parse policy TOML"));

    let empty = parse_ci_policy_v1(
        r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = []
"#,
    )
    .expect_err("empty allow list fails");
    assert!(empty.to_string().contains("exit_class.allowed"));
}

#[test]
fn policy_parser_accepts_build_provenance_rules() {
    let policy = parse_ci_policy_v1(BUILD_PROVENANCE_POLICY).expect("parse policy");

    let rules = policy
        .build_provenance
        .expect("build provenance rules present");
    assert!(rules.is_enabled(PolicyBuildProvenanceRuleV1::CleanSource));
    assert!(rules.is_enabled(PolicyBuildProvenanceRuleV1::CargoLock));
    assert!(rules.is_enabled(PolicyBuildProvenanceRuleV1::WasmGzip));
    assert!(rules.is_enabled(PolicyBuildProvenanceRuleV1::Sha256));
    assert!(rules.is_enabled(PolicyBuildProvenanceRuleV1::PackageIdentityMatchesTarget));
}

#[test]
fn policy_parser_rejects_empty_build_provenance_rules() {
    let err = parse_ci_policy_v1(
        r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]

[build_provenance]
"#,
    )
    .expect_err("empty build provenance rules fail");

    assert!(matches!(err, PolicyGateError::InvalidPolicy(_)));
}

#[test]
fn policy_parser_rejects_unknown_build_provenance_keys() {
    let err = parse_ci_policy_v1(
        r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]

[build_provenance]
require_magic = true
"#,
    )
    .expect_err("unknown build provenance keys fail");

    assert!(matches!(err, PolicyGateError::Toml(_)));
}
