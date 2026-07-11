use super::codec::hex_bytes;
use super::*;
use crate::{cli::globals, run};
use std::{cell::RefCell, collections::VecDeque};

#[test]
fn parses_renewal_status_options() {
    let command = AuthOptions::parse([
        OsString::from("renewal"),
        OsString::from("status"),
        OsString::from("local"),
        OsString::from("--issuer"),
        OsString::from("rrkah-fqaaa-aaaaa-aaaaq-cai"),
        OsString::from("--json"),
        OsString::from(globals::INTERNAL_NETWORK_OPTION),
        OsString::from("local"),
        OsString::from(globals::INTERNAL_ICP_OPTION),
        OsString::from("/bin/icp"),
    ])
    .expect("parse auth renewal status options");

    let AuthCommand::RenewalStatus(options) = command;
    assert_eq!(options.deployment, "local");
    assert_eq!(options.issuer, "rrkah-fqaaa-aaaaa-aaaaq-cai");
    assert_eq!(options.common.network, "local");
    assert_eq!(options.common.icp, "/bin/icp");
    assert!(options.json);
}

#[test]
fn renewal_help_names_chain_key_status_surface() {
    let text = render_usage(renewal_command);

    assert!(text.contains("Inspect root-managed chain-key delegation proof renewal"));
    assert!(text.contains("status"));
    assert!(!text.contains("run-once"));
    assert!(!text.contains("provisioner"));
}

#[test]
fn top_level_forwards_auth_global_icp_and_network() {
    let err = run([
        OsString::from("--icp"),
        OsString::from("/bin/icp"),
        OsString::from("--network"),
        OsString::from("local"),
        OsString::from("auth"),
        OsString::from("renewal"),
        OsString::from("status"),
    ])
    .expect_err("missing status arguments should be parsed after global options");

    assert!(err.to_string().contains("Usage: canic auth"));
}

#[test]
fn renewal_status_queries_root_status_endpoint() {
    let issuer = "rrkah-fqaaa-aaaaa-aaaaq-cai";
    let runtime = ScriptedAuthRenewalRuntime::new([scripted_response(
        CANIC_ROOT_ISSUER_RENEWAL_STATUS,
        Some(root_issuer_renewal_status_arg(issuer)),
        Some("json"),
        serde_json::json!({
            "template": {
                "enabled": true,
                "cert_ttl_ns": "300000000000"
            },
            "state": {
                "last_outcome": "Installed",
                "consecutive_failures": 0,
                "last_installed_expires_at_ns": ["1620329000000000000"],
                "last_installed_refresh_after_ns": ["1620328900000000000"],
                "next_attempt_after_ns": "1620328900000000000",
                "active_attempt_id": [vec![1_u8; 32]]
            },
            "active_attempt": {
                "status": "Prepared",
                "batch_id": vec![2_u8; 32],
                "prepared_expires_at_ns": "1620329000000000000",
                "failure": null
            }
        })
        .to_string(),
    )]);
    let result = renewal_status_result_with_runtime(
        &runtime,
        &RenewalStatusOptions {
            deployment: "local".to_string(),
            issuer: issuer.to_string(),
            json: true,
            common: CommonOptions {
                network: "local".to_string(),
                icp: "icp".to_string(),
            },
        },
    )
    .expect("status should query scripted endpoint");

    assert_eq!(result.kind, AuthRenewalReportKind::Status);
    assert_eq!(result.schema_version, AUTH_RENEWAL_STATUS_SCHEMA_VERSION);
    assert_eq!(result.issuer_pid, issuer);
    assert_eq!(result.status, AuthRenewalStatusCode::IssuerUnregistered);
    assert_eq!(result.renewal.template.enabled, Some(true));
    assert_eq!(
        result.issuer_observation.status,
        AuthRenewalStatusCode::Unavailable.label()
    );
    assert_eq!(
        result.issuer_observation.reason.as_deref(),
        Some(ISSUER_NOT_IN_SUBNET_REGISTRY_REASON)
    );
    assert_eq!(
        result.renewal.state.last_outcome.as_deref(),
        Some("installed")
    );
    assert_eq!(
        result.renewal.active_attempt.status.as_deref(),
        Some("prepared")
    );
    let json = serde_json::to_value(&result).expect("serialize auth renewal result");
    assert_eq!(json["kind"], "auth_renewal_status");
    assert_eq!(json["target"]["candid_source"], "installed_deployment");
    assert_eq!(json["status"], "issuer_unregistered");
    assert_eq!(json["issuer_observation"]["status"], "unavailable");
    assert_eq!(
        runtime.called_methods(),
        vec![CANIC_ROOT_ISSUER_RENEWAL_STATUS]
    );
    let medic = auth_renewal_medic_summary_from_result(&result);
    assert_eq!(medic.status, AuthRenewalMedicStatus::Warning);
    assert!(
        medic
            .next
            .contains("reinstalling the affected dependency closure")
    );
    assert!(
        medic
            .next
            .contains("do not provision delegation proof state manually")
    );
}

#[test]
fn renewal_status_reports_matching_issuer_observation() {
    let issuer = "rrkah-fqaaa-aaaaa-aaaaq-cai";
    let runtime = ScriptedAuthRenewalRuntime::new([
        scripted_response(
            CANIC_ROOT_ISSUER_RENEWAL_STATUS,
            Some(root_issuer_renewal_status_arg(issuer)),
            Some("json"),
            renewal_status_response_json(issuer, [3; 32], "1620329000000000000"),
        ),
        scripted_response(
            CANIC_ACTIVE_DELEGATION_PROOF_STATUS,
            None,
            Some("json"),
            issuer_status_response_json([3; 32], "1620329000000000000"),
        ),
    ])
    .with_issuer_available();

    let result = renewal_status_result_with_runtime(&runtime, &renewal_status_options(issuer))
        .expect("status should include issuer observation");

    assert_eq!(result.status, AuthRenewalStatusCode::Configured);
    assert!(result.issuer_observation.available);
    assert!(!result.issuer_observation.drift_detected);
    assert_eq!(
        result.issuer_observation.cert_hash,
        Some(hex_bytes(&[3; 32]))
    );
    assert_eq!(
        runtime.called_methods(),
        vec![
            CANIC_ROOT_ISSUER_RENEWAL_STATUS,
            CANIC_ACTIVE_DELEGATION_PROOF_STATUS,
        ]
    );
}

#[test]
fn renewal_status_reports_root_issuer_drift() {
    let issuer = "rrkah-fqaaa-aaaaa-aaaaq-cai";
    let runtime = ScriptedAuthRenewalRuntime::new([
        scripted_response(
            CANIC_ROOT_ISSUER_RENEWAL_STATUS,
            Some(root_issuer_renewal_status_arg(issuer)),
            Some("json"),
            renewal_status_response_json(issuer, [3; 32], "1620329000000000000"),
        ),
        scripted_response(
            CANIC_ACTIVE_DELEGATION_PROOF_STATUS,
            None,
            Some("json"),
            issuer_status_response_json([4; 32], "1620329000000000000"),
        ),
    ])
    .with_issuer_available();

    let result = renewal_status_result_with_runtime(&runtime, &renewal_status_options(issuer))
        .expect("status should include drift observation");
    let rendered = render::render_renewal_status_result(&result);

    assert_eq!(result.status, AuthRenewalStatusCode::DriftDetected);
    assert!(result.issuer_observation.drift_detected);
    assert!(rendered.contains("Issuer observation: drift_detected"));
    assert!(rendered.contains(&hex_bytes(&[4; 32])));
}

#[test]
fn renewal_status_rejects_invalid_issuer_principal() {
    let runtime = ScriptedAuthRenewalRuntime::empty();
    let err = renewal_status_result_with_runtime(
        &runtime,
        &RenewalStatusOptions {
            deployment: "local".to_string(),
            issuer: "not a principal".to_string(),
            json: false,
            common: CommonOptions {
                network: "local".to_string(),
                icp: "icp".to_string(),
            },
        },
    )
    .expect_err("invalid issuer principal should fail before transport");

    assert!(matches!(
        err,
        AuthCommandError::InvalidIssuerPrincipal { .. }
    ));
    assert!(runtime.called_methods().is_empty());
}

#[test]
fn renewal_response_reports_the_malformed_field() {
    assert_eq!(
        codec::parse_renewal_status_summary(r#"{"template":{"enabled":"yes"}}"#),
        Err(codec::AuthResponseParseError::InvalidField {
            kind: codec::AuthResponseKind::RenewalStatus,
            field: "template.enabled"
        })
    );
}

fn renewal_status_options(issuer: &str) -> RenewalStatusOptions {
    RenewalStatusOptions {
        deployment: "local".to_string(),
        issuer: issuer.to_string(),
        json: true,
        common: CommonOptions {
            network: "local".to_string(),
            icp: "icp".to_string(),
        },
    }
}

fn renewal_status_response_json(_issuer: &str, cert_hash: [u8; 32], expires_at_ns: &str) -> String {
    serde_json::json!({
        "template": {
            "enabled": true,
            "cert_ttl_ns": "300000000000"
        },
        "state": {
            "last_installed_cert_hash": [cert_hash.to_vec()],
            "last_outcome": "Installed",
            "consecutive_failures": 0,
            "last_installed_expires_at_ns": [expires_at_ns],
            "last_installed_refresh_after_ns": ["1620328900000000000"],
            "next_attempt_after_ns": "1620328900000000000",
            "active_attempt_id": null
        },
        "active_attempt": null
    })
    .to_string()
}

fn issuer_status_response_json(cert_hash: [u8; 32], expires_at_ns: &str) -> String {
    serde_json::json!({
        "status": "Valid",
        "root_pid": ["r7inp-6aaaa-aaaaa-aaabq-cai"],
        "issuer_pid": ["rrkah-fqaaa-aaaaa-aaaaq-cai"],
        "cert_hash": [cert_hash.to_vec()],
        "expires_at_ns": [expires_at_ns],
        "refresh_after_ns": ["1620328900000000000"]
    })
    .to_string()
}

struct ScriptedAuthRenewalRuntime {
    responses: RefCell<VecDeque<ScriptedAuthRenewalResponse>>,
    calls: RefCell<Vec<String>>,
    issuer_available: bool,
}

impl ScriptedAuthRenewalRuntime {
    fn empty() -> Self {
        Self {
            responses: RefCell::new(VecDeque::new()),
            calls: RefCell::new(Vec::new()),
            issuer_available: false,
        }
    }

    fn new<const N: usize>(responses: [ScriptedAuthRenewalResponse; N]) -> Self {
        Self {
            responses: RefCell::new(VecDeque::from(responses)),
            calls: RefCell::new(Vec::new()),
            issuer_available: false,
        }
    }

    fn with_issuer_available(mut self) -> Self {
        self.issuer_available = true;
        self
    }

    fn called_methods(&self) -> Vec<&'static str> {
        self.calls
            .borrow()
            .iter()
            .map(String::as_str)
            .map(|method| match method {
                CANIC_ROOT_ISSUER_RENEWAL_STATUS => CANIC_ROOT_ISSUER_RENEWAL_STATUS,
                CANIC_ACTIVE_DELEGATION_PROOF_STATUS => CANIC_ACTIVE_DELEGATION_PROOF_STATUS,
                _ => panic!("unexpected method {method}"),
            })
            .collect()
    }
}

impl AuthRenewalRuntime for ScriptedAuthRenewalRuntime {
    fn resolve_root_target(
        &self,
        _options: &CommonOptions,
        _deployment: &str,
        _method: &str,
        _expected_mode: AuthRenewalMethodMode,
    ) -> Result<AuthRootCallTarget, AuthCommandError> {
        Ok(AuthRootCallTarget {
            target: AuthRootTarget {
                input: ROOT_ROLE.to_string(),
                role: ROOT_ROLE.to_string(),
                canister_id: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
                candid_source: AuthRenewalCandidSource::InstalledDeployment,
            },
            candid_path: PathBuf::from(".icp/local/canisters/root/root.did"),
            icp_root: PathBuf::from("."),
            registry_entries: if self.issuer_available {
                vec![RegistryEntry {
                    pid: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
                    role: Some("issuer".to_string()),
                    kind: None,
                    parent_pid: None,
                    module_hash: None,
                }]
            } else {
                Vec::new()
            },
        })
    }

    fn query_output(
        &self,
        _options: &CommonOptions,
        _target: &AuthRootCallTarget,
        method: &str,
        arg: Option<&str>,
        output: Option<&str>,
    ) -> Result<String, AuthCommandError> {
        Ok(self.call(method, arg, output))
    }

    fn resolve_issuer_target(
        &self,
        _options: &CommonOptions,
        root_target: &AuthRootCallTarget,
        issuer_pid: &str,
        _method: &str,
        _expected_mode: AuthRenewalMethodMode,
    ) -> Result<Option<AuthIssuerCallTarget>, AuthCommandError> {
        if root_target
            .registry_entries
            .iter()
            .any(|entry| entry.pid == issuer_pid)
        {
            Ok(Some(AuthIssuerCallTarget {
                target: AuthIssuerTarget {
                    input: issuer_pid.to_string(),
                    role: Some("issuer".to_string()),
                    canister_id: issuer_pid.to_string(),
                    candid_source: AuthRenewalCandidSource::InstalledDeployment,
                },
                candid_path: PathBuf::from(".icp/local/canisters/issuer/issuer.did"),
                icp_root: PathBuf::from("."),
            }))
        } else {
            Ok(None)
        }
    }

    fn query_issuer_output(
        &self,
        _options: &CommonOptions,
        _target: &AuthIssuerCallTarget,
        method: &str,
        output: Option<&str>,
    ) -> Result<String, AuthCommandError> {
        Ok(self.call(method, None, output))
    }
}

impl ScriptedAuthRenewalRuntime {
    fn call(&self, method: &str, arg: Option<&str>, output: Option<&str>) -> String {
        self.calls.borrow_mut().push(method.to_string());
        let response = self
            .responses
            .borrow_mut()
            .pop_front()
            .expect("scripted response");

        assert_eq!(response.method, method);
        assert_eq!(response.arg.as_deref(), arg);
        assert_eq!(response.output, output);
        response.body
    }
}

struct ScriptedAuthRenewalResponse {
    method: &'static str,
    arg: Option<String>,
    output: Option<&'static str>,
    body: String,
}

fn scripted_response(
    method: &'static str,
    arg: Option<String>,
    output: Option<&'static str>,
    body: String,
) -> ScriptedAuthRenewalResponse {
    ScriptedAuthRenewalResponse {
        method,
        arg,
        output,
        body,
    }
}
