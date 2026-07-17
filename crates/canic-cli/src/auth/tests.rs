use super::*;
use crate::{cli::globals, run};
use candid::{CandidType, Encode, Principal};
use canic_core::cdk::utils::hash::hex_bytes;
use canic_core::dto::{
    auth::{
        ActiveDelegationProofStatus, ActiveDelegationProofStatusResponse, DelegationAudience,
        RootIssuerRenewalBatchStatus, RootIssuerRenewalBatchView, RootIssuerRenewalStateView,
        RootIssuerRenewalStatusResponse, RootIssuerRenewalTemplateView,
    },
    error::{Error as CanicError, ErrorCode},
};
use std::{cell::RefCell, collections::VecDeque};

#[test]
fn icp_io_failure_keeps_usage_exit_class() {
    let error = AuthCommandError::Icp(IcpCommandError::Io(std::io::Error::other("sample")));

    assert_eq!(error.exit_code(), 1);
}

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
        renewal_status_with_batch_response_json(issuer),
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
        result.renewal.latest_batch.status.as_deref(),
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
            renewal_status_response_json(issuer, [3; 32], 1_620_329_000_000_000_000),
        ),
        scripted_response(
            CANIC_ACTIVE_DELEGATION_PROOF_STATUS,
            None,
            Some("json"),
            issuer_status_response_json([3; 32], 1_620_329_000_000_000_000),
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
        Some(hex_bytes([3; 32]))
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
            renewal_status_response_json(issuer, [3; 32], 1_620_329_000_000_000_000),
        ),
        scripted_response(
            CANIC_ACTIVE_DELEGATION_PROOF_STATUS,
            None,
            Some("json"),
            issuer_status_response_json([4; 32], 1_620_329_000_000_000_000),
        ),
    ])
    .with_issuer_available();

    let result = renewal_status_result_with_runtime(&runtime, &renewal_status_options(issuer))
        .expect("status should include drift observation");
    let rendered = render::render_renewal_status_result(&result);

    assert_eq!(result.status, AuthRenewalStatusCode::DriftDetected);
    assert!(result.issuer_observation.drift_detected);
    assert!(rendered.contains("Issuer observation: drift_detected"));
    assert!(rendered.contains(&hex_bytes([4; 32])));
}

#[test]
fn renewal_status_warns_when_active_proof_is_missing() {
    let issuer = "rrkah-fqaaa-aaaaa-aaaaq-cai";
    let runtime = ScriptedAuthRenewalRuntime::new([
        scripted_response(
            CANIC_ROOT_ISSUER_RENEWAL_STATUS,
            Some(root_issuer_renewal_status_arg(issuer)),
            Some("json"),
            renewal_status_without_state_response_json(issuer),
        ),
        scripted_response(
            CANIC_ACTIVE_DELEGATION_PROOF_STATUS,
            None,
            Some("json"),
            issuer_missing_status_response_json(),
        ),
    ])
    .with_issuer_available();

    let result = renewal_status_result_with_runtime(&runtime, &renewal_status_options(issuer))
        .expect("missing proof status should remain observable");
    let medic = auth_renewal_medic_summary_from_result(&result);

    assert_eq!(result.status, AuthRenewalStatusCode::ProofUnavailable);
    assert_eq!(medic.status, AuthRenewalMedicStatus::Warning);
    assert!(medic.next.contains("root readiness facade"));
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
fn renewal_response_rejects_unwrapped_and_nested_payloads() {
    for output in [
        r#"{"response_candid":"(variant { Ok })"}"#,
        r#"{"transport":{"response_bytes":"00"}}"#,
    ] {
        assert_eq!(
            codec::parse_renewal_status_summary(output),
            Err(codec::AuthResponseParseError::InvalidPayload(
                codec::AuthResponseKind::RenewalStatus,
            ))
        );
    }
}

#[test]
fn renewal_response_preserves_typed_remote_error() {
    let response = icp_json_response(Err::<RootIssuerRenewalStatusResponse, _>(CanicError::new(
        ErrorCode::Unauthorized,
        "caller is not authorized".to_string(),
    )));

    assert_eq!(
        codec::parse_renewal_status_summary(&response),
        Err(codec::AuthResponseParseError::RemoteError {
            kind: codec::AuthResponseKind::RenewalStatus,
            code: ErrorCode::Unauthorized,
            message: "caller is not authorized".to_string(),
        })
    );
}

#[test]
fn renewal_response_rejects_invalid_response_bytes() {
    assert!(matches!(
        codec::parse_renewal_status_summary(r#"{"response_bytes":"no"}"#),
        Err(codec::AuthResponseParseError::InvalidResponseBytes {
            kind: codec::AuthResponseKind::RenewalStatus,
            ..
        })
    ));
}

#[test]
fn renewal_response_rejects_wrong_candid_type() {
    let response = icp_json_response(42_u64);

    assert!(matches!(
        codec::parse_renewal_status_summary(&response),
        Err(codec::AuthResponseParseError::InvalidCandid {
            kind: codec::AuthResponseKind::RenewalStatus,
            ..
        })
    ));
}

#[test]
fn renewal_response_rejects_invalid_json() {
    assert!(matches!(
        codec::parse_renewal_status_summary("not json"),
        Err(codec::AuthResponseParseError::InvalidJson {
            kind: codec::AuthResponseKind::RenewalStatus,
            ..
        })
    ));
}

fn icp_json_response<T: CandidType>(response: T) -> String {
    let response_bytes = Encode!(&response).expect("encode scripted Candid response");
    serde_json::json!({
        "response_bytes": canic_core::cdk::utils::hash::hex_bytes(response_bytes),
        "response_text": null,
        "response_candid": "scripted"
    })
    .to_string()
}

fn renewal_template(issuer: &str) -> RootIssuerRenewalTemplateView {
    RootIssuerRenewalTemplateView {
        issuer_pid: Principal::from_text(issuer).expect("issuer principal"),
        enabled: true,
        aud: DelegationAudience::Project("test".to_string()),
        grants: Vec::new(),
        cert_ttl_ns: 300_000_000_000,
    }
}

fn renewal_status_with_batch_response_json(issuer: &str) -> String {
    icp_json_response(Ok::<_, CanicError>(RootIssuerRenewalStatusResponse {
        template: Some(renewal_template(issuer)),
        state: Some(RootIssuerRenewalStateView {
            issuer_pid: Principal::from_text(issuer).expect("issuer principal"),
            template_fingerprint: [1; 32],
            last_installed_cert_hash: None,
            last_installed_expires_at_ns: Some(1_620_329_000_000_000_000),
            last_installed_refresh_after_ns: Some(1_620_328_900_000_000_000),
            next_attempt_after_ns: 1_620_328_900_000_000_000,
            updated_at_ns: 1_620_328_800_000_000_000,
        }),
        latest_batch: Some(RootIssuerRenewalBatchView {
            batch_id: [2; 32],
            status: RootIssuerRenewalBatchStatus::Prepared,
            cert_hash: [3; 32],
            proof_epoch: 4,
            prepared_at_ns: 1_620_328_800_000_000_000,
            expires_at_ns: 1_620_329_000_000_000_000,
            installed_at_ns: None,
            retry_after_ns: None,
            failure: None,
        }),
    }))
}

fn renewal_status_without_state_response_json(issuer: &str) -> String {
    icp_json_response(Ok::<_, CanicError>(RootIssuerRenewalStatusResponse {
        template: Some(renewal_template(issuer)),
        state: None,
        latest_batch: None,
    }))
}

fn issuer_missing_status_response_json() -> String {
    icp_json_response(Ok::<_, CanicError>(ActiveDelegationProofStatusResponse {
        status: ActiveDelegationProofStatus::Missing,
        root_pid: None,
        issuer_pid: None,
        cert_hash: None,
        expires_at_ns: None,
        refresh_after_ns: None,
    }))
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

fn renewal_status_response_json(issuer: &str, cert_hash: [u8; 32], expires_at_ns: u64) -> String {
    icp_json_response(Ok::<_, CanicError>(RootIssuerRenewalStatusResponse {
        template: Some(renewal_template(issuer)),
        state: Some(RootIssuerRenewalStateView {
            issuer_pid: Principal::from_text(issuer).expect("issuer principal"),
            template_fingerprint: [1; 32],
            last_installed_cert_hash: Some(cert_hash),
            last_installed_expires_at_ns: Some(expires_at_ns),
            last_installed_refresh_after_ns: Some(1_620_328_900_000_000_000),
            next_attempt_after_ns: 1_620_328_900_000_000_000,
            updated_at_ns: 1_620_328_800_000_000_000,
        }),
        latest_batch: None,
    }))
}

fn issuer_status_response_json(cert_hash: [u8; 32], expires_at_ns: u64) -> String {
    icp_json_response(Ok::<_, CanicError>(ActiveDelegationProofStatusResponse {
        status: ActiveDelegationProofStatus::Valid,
        root_pid: Some(
            Principal::from_text("r7inp-6aaaa-aaaaa-aaabq-cai").expect("root principal"),
        ),
        issuer_pid: Some(
            Principal::from_text("rrkah-fqaaa-aaaaa-aaaaq-cai").expect("issuer principal"),
        ),
        cert_hash: Some(cert_hash),
        expires_at_ns: Some(expires_at_ns),
        refresh_after_ns: Some(1_620_328_900_000_000_000),
    }))
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
