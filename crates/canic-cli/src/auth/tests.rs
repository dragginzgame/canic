use super::*;
use crate::{cli::globals, run};
use canic_contracts::{
    dto::{
        auth::{
            ActiveDelegationProofStatus, DelegationAudience, RootIssuerRenewalBatchStatus,
            RootIssuerRenewalBatchView,
        },
        issuer_renewal::{RootIssuerRenewalStateView, RootIssuerRenewalTemplateView},
    },
    ids::{CanonicalNetworkId, FleetId, FleetKey},
};
use canic_core::cdk::utils::hash::hex_bytes;
use ic_platform::Principal;
use std::{cell::RefCell, collections::VecDeque};

fn fleet() -> FleetKey {
    FleetKey {
        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
        fleet_id: FleetId::from_generated_bytes([1; 32]),
    }
}

#[test]
fn icp_io_failure_keeps_usage_exit_class() {
    let error = AuthCommandError::Icp(IcpCommandError::Io(std::io::Error::other("sample")));

    assert_eq!(error.exit_code(), 1);
    let query = AuthCommandError::RootQuery(canic_host::icp::IcpWireQueryError::Icp(
        IcpCommandError::Io(std::io::Error::other("sample")),
    ));
    assert_eq!(query.exit_code(), 1);
}

#[test]
fn parses_renewal_status_options() {
    let options = AuthOptions::parse([
        OsString::from("renewal"),
        OsString::from("status"),
        OsString::from("local"),
        OsString::from("--issuer"),
        OsString::from("rrkah-fqaaa-aaaaa-aaaaq-cai"),
        OsString::from("--json"),
        OsString::from(globals::INTERNAL_ENVIRONMENT_OPTION),
        OsString::from("local"),
        OsString::from(globals::INTERNAL_ICP_OPTION),
        OsString::from("/bin/icp"),
    ])
    .expect("parse auth renewal status options");

    assert_eq!(options.fleet, "local");
    assert_eq!(options.issuer, "rrkah-fqaaa-aaaaa-aaaaq-cai");
    assert_eq!(options.common.environment, "local");
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
fn top_level_forwards_auth_global_icp_and_environment() {
    let err = run([
        OsString::from("--icp"),
        OsString::from("/bin/icp"),
        OsString::from("--environment"),
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
        renewal_protocol::METHOD,
        Some(issuer.to_string()),
        None,
        renewal_status_with_batch_response_json(issuer),
    )]);
    let result = renewal_status_result_with_runtime(
        &runtime,
        &RenewalStatusOptions {
            fleet: "local".to_string(),
            issuer: issuer.to_string(),
            json: true,
            common: CommonOptions {
                environment: "local".to_string(),
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
    assert_eq!(result.issuer_observation.status, None);
    assert_eq!(
        result.issuer_observation.reason,
        Some(AuthIssuerUnavailableReason::IssuerNotInComponentRegistry)
    );
    assert_eq!(
        result.renewal.latest_batch.status,
        Some(RootIssuerRenewalBatchStatus::Prepared)
    );
    let json = serde_json::to_value(&result).expect("serialize auth renewal result");
    assert_eq!(json["kind"], "auth_renewal_status");
    assert_eq!(json["target"]["protocol_source"], "installed_release");
    assert_eq!(json["status"], "issuer_unregistered");
    assert_eq!(json["issuer_observation"]["status"], "unavailable");
    assert_eq!(runtime.called_methods(), vec![renewal_protocol::METHOD]);
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
            renewal_protocol::METHOD,
            Some(issuer.to_string()),
            None,
            renewal_status_response_json(issuer, [3; 32], 1_620_329_000_000_000_000),
        ),
        scripted_response(
            proof_status::METHOD,
            None,
            None,
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
        vec![renewal_protocol::METHOD, proof_status::METHOD]
    );
}

#[test]
fn renewal_medic_requires_configured_renewal_and_a_usable_observed_proof() {
    let issuer = "rrkah-fqaaa-aaaaa-aaaaq-cai";
    let runtime = ScriptedAuthRenewalRuntime::new([
        scripted_response(
            renewal_protocol::METHOD,
            Some(issuer.to_string()),
            None,
            renewal_status_response_json(issuer, [3; 32], 1_620_329_000_000_000_000),
        ),
        scripted_response(
            proof_status::METHOD,
            None,
            None,
            issuer_status_response_json([3; 32], 1_620_329_000_000_000_000),
        ),
    ])
    .with_issuer_available();
    let baseline = renewal_status_result_with_runtime(&runtime, &renewal_status_options(issuer))
        .expect("matching installed proof");

    for proof_status in [
        ActiveDelegationProofStatus::Valid,
        ActiveDelegationProofStatus::RefreshNeeded,
    ] {
        for (expected, present, enabled, batch) in [
            (AuthRenewalStatusCode::Configured, true, Some(true), None),
            (AuthRenewalStatusCode::Disabled, true, Some(false), None),
            (AuthRenewalStatusCode::Missing, false, None, None),
            (
                AuthRenewalStatusCode::BatchFailed,
                true,
                Some(true),
                Some(RootIssuerRenewalBatchStatus::FailedRetryable),
            ),
            (
                AuthRenewalStatusCode::BatchPending,
                true,
                Some(true),
                Some(RootIssuerRenewalBatchStatus::Prepared),
            ),
            (
                AuthRenewalStatusCode::BatchPending,
                true,
                Some(true),
                Some(RootIssuerRenewalBatchStatus::Signing),
            ),
            (
                AuthRenewalStatusCode::BatchPending,
                true,
                Some(true),
                Some(RootIssuerRenewalBatchStatus::Signed),
            ),
            (
                AuthRenewalStatusCode::BatchPending,
                true,
                Some(true),
                Some(RootIssuerRenewalBatchStatus::Installing),
            ),
        ] {
            let mut result = baseline.clone();
            result.issuer_observation.status = Some(proof_status.clone());
            result.renewal.template.present = present;
            result.renewal.template.enabled = enabled;
            result.renewal.latest_batch.status = batch;
            result.status = renewal_status_code(&result.renewal, &result.issuer_observation);
            assert_eq!(result.status, expected);
            let medic = auth_renewal_medic_summary_from_result(&result);
            let expected_medic = if expected == AuthRenewalStatusCode::Configured {
                AuthRenewalMedicStatus::Ready
            } else {
                assert_ne!(medic.next, "-");
                AuthRenewalMedicStatus::Warning
            };
            assert_eq!(
                medic.status, expected_medic,
                "{proof_status:?}: {expected:?}"
            );
        }
    }
    for reason in [
        AuthIssuerUnavailableReason::IssuerStatusQueryFailed,
        AuthIssuerUnavailableReason::IssuerStatusParseFailed,
    ] {
        let mut result = baseline.clone();
        result.issuer_observation = unavailable_issuer_observation(reason);
        result.status = renewal_status_code(&result.renewal, &result.issuer_observation);
        assert_eq!(result.status, AuthRenewalStatusCode::Unavailable);
        assert_eq!(
            auth_renewal_medic_summary_from_result(&result).status,
            AuthRenewalMedicStatus::Warning
        );
    }
}

#[test]
fn registered_issuer_without_retained_release_reports_unavailable() {
    let issuer = "rrkah-fqaaa-aaaaa-aaaaq-cai";
    let options = renewal_status_options(issuer);
    let runtime = ScriptedAuthRenewalRuntime::new([scripted_response(
        renewal_protocol::METHOD,
        Some(issuer.to_string()),
        None,
        renewal_status_response_json(issuer, [3; 32], 1_620_329_000_000_000_000),
    )])
    .with_local_issuer_metadata();
    let mut root = runtime
        .resolve_root_target(&options.common, "local")
        .unwrap();
    for role in [Some("issuer".to_string()), None] {
        root.registry_entries[0].role = role;
        let error = resolve_auth_issuer_call_target(&options.common, &root, issuer)
            .expect_err("registered issuer must preserve the metadata refusal");
        assert!(matches!(
            error,
            AuthCommandError::IssuerProtocolBinding { .. }
        ));
    }

    let result = renewal_status_result_with_runtime(&runtime, &options).unwrap();
    assert_eq!(result.status, AuthRenewalStatusCode::Unavailable);
    assert_eq!(
        result.issuer_observation.reason,
        Some(AuthIssuerUnavailableReason::IssuerStatusMetadataUnavailable)
    );
    assert_eq!(runtime.called_methods(), vec![renewal_protocol::METHOD]);
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["status"], "unavailable");
    assert_eq!(
        json["issuer_observation"]["reason"],
        "issuer_status_metadata_unavailable"
    );
    let medic = auth_renewal_medic_summary_from_result(&result);
    assert_eq!(medic.status, AuthRenewalMedicStatus::Warning);
    assert!(medic.next.contains(&format!(
        "canic auth renewal status {} --issuer {issuer}",
        options.fleet
    )));
}

#[test]
fn renewal_status_reports_root_issuer_drift() {
    let issuer = "rrkah-fqaaa-aaaaa-aaaaq-cai";
    let runtime = ScriptedAuthRenewalRuntime::new([
        scripted_response(
            renewal_protocol::METHOD,
            Some(issuer.to_string()),
            None,
            renewal_status_response_json(issuer, [3; 32], 1_620_329_000_000_000_000),
        ),
        scripted_response(
            proof_status::METHOD,
            None,
            None,
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
            renewal_protocol::METHOD,
            Some(issuer.to_string()),
            None,
            renewal_status_without_state_response_json(issuer),
        ),
        scripted_response(
            proof_status::METHOD,
            None,
            None,
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
fn report_serialization_preserves_auth_state_and_reason_labels() {
    let issuer = "rrkah-fqaaa-aaaaa-aaaaq-cai";
    let response = serde_json::from_str(&renewal_status_with_batch_response_json(issuer))
        .expect("native renewal response");
    let mut batch = renewal_status_summary(response).latest_batch;
    for (status, label) in [
        (
            RootIssuerRenewalBatchStatus::FailedRetryable,
            "failed_retryable",
        ),
        (RootIssuerRenewalBatchStatus::Installed, "installed"),
        (RootIssuerRenewalBatchStatus::Installing, "installing"),
        (RootIssuerRenewalBatchStatus::Prepared, "prepared"),
        (RootIssuerRenewalBatchStatus::Signed, "signed"),
        (RootIssuerRenewalBatchStatus::Signing, "signing"),
    ] {
        batch.status = Some(status);
        assert_eq!(
            serde_json::to_value(&batch).expect("batch JSON")["status"],
            label
        );
    }
    batch.status = None;
    assert!(serde_json::to_value(&batch).expect("batch JSON")["status"].is_null());

    for (reason, label) in [
        (
            AuthIssuerUnavailableReason::IssuerNotInComponentRegistry,
            "issuer_not_in_component_registry",
        ),
        (
            AuthIssuerUnavailableReason::IssuerStatusMetadataUnavailable,
            "issuer_status_metadata_unavailable",
        ),
        (
            AuthIssuerUnavailableReason::IssuerStatusParseFailed,
            "issuer_status_parse_failed",
        ),
        (
            AuthIssuerUnavailableReason::IssuerStatusQueryFailed,
            "issuer_status_query_failed",
        ),
    ] {
        let observation = unavailable_issuer_observation(reason);
        let json = serde_json::to_value(&observation).expect("issuer JSON");
        assert_eq!(json["status"], "unavailable");
        assert_eq!(json["reason"], label);
        assert_eq!(
            render::render_issuer_observation(&observation),
            "unavailable"
        );
    }

    for (status, label) in [
        (ActiveDelegationProofStatus::Expired, "expired"),
        (ActiveDelegationProofStatus::Missing, "missing"),
        (ActiveDelegationProofStatus::RefreshNeeded, "refresh_needed"),
        (ActiveDelegationProofStatus::Valid, "valid"),
    ] {
        let observation = AuthIssuerObservation {
            available: true,
            status: Some(status),
            reason: None,
            ..unavailable_issuer_observation(AuthIssuerUnavailableReason::IssuerStatusQueryFailed)
        };
        assert_eq!(
            serde_json::to_value(&observation).expect("issuer JSON")["status"],
            label
        );
        assert_eq!(render::render_issuer_observation(&observation), label);
    }
}

#[test]
fn renewal_status_rejects_invalid_issuer_principal() {
    let runtime = ScriptedAuthRenewalRuntime::empty();
    let err = renewal_status_result_with_runtime(
        &runtime,
        &RenewalStatusOptions {
            fleet: "local".to_string(),
            issuer: "not a principal".to_string(),
            json: false,
            common: CommonOptions {
                environment: "local".to_string(),
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

fn renewal_template(issuer: &str) -> RootIssuerRenewalTemplateView {
    RootIssuerRenewalTemplateView {
        issuer_pid: Principal::from_text(issuer).expect("issuer principal"),
        enabled: true,
        aud: DelegationAudience::Fleet(fleet()),
        grants: Vec::new(),
        cert_ttl_ns: 300_000_000_000,
    }
}

fn renewal_status_with_batch_response_json(issuer: &str) -> String {
    serde_json::to_string(&RootIssuerRenewalStatusResponse {
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
    })
    .unwrap()
}

fn renewal_status_without_state_response_json(issuer: &str) -> String {
    serde_json::to_string(&RootIssuerRenewalStatusResponse {
        template: Some(renewal_template(issuer)),
        state: None,
        latest_batch: None,
    })
    .unwrap()
}

fn issuer_missing_status_response_json() -> String {
    serde_json::to_string(&ActiveDelegationProofStatusResponse {
        status: ActiveDelegationProofStatus::Missing,
        root_pid: None,
        issuer_pid: None,
        cert_hash: None,
        expires_at_ns: None,
        refresh_after_ns: None,
    })
    .unwrap()
}

fn renewal_status_options(issuer: &str) -> RenewalStatusOptions {
    RenewalStatusOptions {
        fleet: "local".to_string(),
        issuer: issuer.to_string(),
        json: true,
        common: CommonOptions {
            environment: "local".to_string(),
            icp: "icp".to_string(),
        },
    }
}

fn renewal_status_response_json(issuer: &str, cert_hash: [u8; 32], expires_at_ns: u64) -> String {
    serde_json::to_string(&RootIssuerRenewalStatusResponse {
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
    })
    .unwrap()
}

fn issuer_status_response_json(cert_hash: [u8; 32], expires_at_ns: u64) -> String {
    serde_json::to_string(&ActiveDelegationProofStatusResponse {
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
    })
    .unwrap()
}

struct ScriptedAuthRenewalRuntime {
    responses: RefCell<VecDeque<ScriptedAuthRenewalResponse>>,
    calls: RefCell<Vec<String>>,
    issuer_available: bool,
    resolve_issuer_metadata: bool,
}

impl ScriptedAuthRenewalRuntime {
    fn empty() -> Self {
        Self {
            responses: RefCell::new(VecDeque::new()),
            calls: RefCell::new(Vec::new()),
            issuer_available: false,
            resolve_issuer_metadata: false,
        }
    }

    fn new<const N: usize>(responses: [ScriptedAuthRenewalResponse; N]) -> Self {
        Self {
            responses: RefCell::new(VecDeque::from(responses)),
            calls: RefCell::new(Vec::new()),
            issuer_available: false,
            resolve_issuer_metadata: false,
        }
    }

    fn with_issuer_available(mut self) -> Self {
        self.issuer_available = true;
        self
    }

    fn with_local_issuer_metadata(mut self) -> Self {
        self.issuer_available = true;
        self.resolve_issuer_metadata = true;
        self
    }

    fn called_methods(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

impl AuthRenewalRuntime for ScriptedAuthRenewalRuntime {
    fn resolve_root_target(
        &self,
        _options: &CommonOptions,
        fleet: &str,
    ) -> Result<AuthRootCallTarget, AuthCommandError> {
        Ok(AuthRootCallTarget {
            fleet: fleet.to_string(),
            target: AuthRootTarget {
                input: ROOT_ROLE.to_string(),
                role: ROOT_ROLE.to_string(),
                canister_id: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
                protocol_source: AuthRenewalProtocolSource::InstalledRelease,
            },
            icp_root: PathBuf::from("."),
            registry_entries: if self.issuer_available {
                vec![RegistryEntry {
                    pid: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
                    role: Some("issuer".to_string()),
                    parent_pid: None,
                    module_hash: None,
                    protocol_binding: None,
                }]
            } else {
                Vec::new()
            },
        })
    }

    fn query_renewal_status(
        &self,
        _options: &CommonOptions,
        _target: &AuthRootCallTarget,
        issuer_pid: &str,
    ) -> Result<RootIssuerRenewalStatusResponse, AuthCommandError> {
        Ok(serde_json::from_str(&self.call(
            renewal_protocol::METHOD,
            Some(issuer_pid),
            None,
        ))?)
    }

    fn resolve_issuer_target(
        &self,
        options: &CommonOptions,
        root_target: &AuthRootCallTarget,
        issuer_pid: &str,
    ) -> Result<Option<AuthIssuerCallTarget>, AuthCommandError> {
        if self.resolve_issuer_metadata {
            return resolve_auth_issuer_call_target(options, root_target, issuer_pid);
        }
        if root_target
            .registry_entries
            .iter()
            .any(|entry| entry.pid == issuer_pid)
        {
            Ok(Some(AuthIssuerCallTarget {
                canister_id: issuer_pid.to_string(),
                icp_root: PathBuf::from("."),
            }))
        } else {
            Ok(None)
        }
    }

    fn query_issuer_status(
        &self,
        _options: &CommonOptions,
        _target: &AuthIssuerCallTarget,
    ) -> Result<ActiveDelegationProofStatusResponse, AuthCommandError> {
        Ok(serde_json::from_str(&self.call(
            proof_status::METHOD,
            None,
            None,
        ))?)
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
