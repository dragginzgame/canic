use super::*;
use crate::{cli::globals, run};
use std::{cell::RefCell, collections::VecDeque};

#[test]
fn parses_renewal_run_once_options() {
    let command = AuthOptions::parse([
        OsString::from("renewal"),
        OsString::from("run-once"),
        OsString::from("local"),
        OsString::from("--json"),
        OsString::from(globals::INTERNAL_NETWORK_OPTION),
        OsString::from("local"),
        OsString::from(globals::INTERNAL_ICP_OPTION),
        OsString::from("/bin/icp"),
    ])
    .expect("parse auth renewal run-once options");

    let AuthCommand::RenewalRunOnce(options) = command else {
        panic!("expected renewal run-once command");
    };
    assert_eq!(options.deployment, "local");
    assert_eq!(options.common.network, "local");
    assert_eq!(options.common.icp, "/bin/icp");
    assert!(options.json);
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

    let AuthCommand::RenewalStatus(options) = command else {
        panic!("expected renewal status command");
    };
    assert_eq!(options.deployment, "local");
    assert_eq!(options.issuer, "rrkah-fqaaa-aaaaa-aaaaq-cai");
    assert_eq!(options.common.network, "local");
    assert_eq!(options.common.icp, "/bin/icp");
    assert!(options.json);
}

#[test]
fn parses_renewal_provisioner_options() {
    let list = AuthOptions::parse([
        OsString::from("renewal"),
        OsString::from("provisioner"),
        OsString::from("list"),
        OsString::from("local"),
        OsString::from("--json"),
        OsString::from(globals::INTERNAL_NETWORK_OPTION),
        OsString::from("local"),
        OsString::from(globals::INTERNAL_ICP_OPTION),
        OsString::from("/bin/icp"),
    ])
    .expect("parse auth renewal provisioner list options");
    let AuthCommand::RenewalProvisionerList(options) = list else {
        panic!("expected renewal provisioner list command");
    };
    assert_eq!(options.deployment, "local");
    assert_eq!(options.common.network, "local");
    assert_eq!(options.common.icp, "/bin/icp");
    assert!(options.json);

    let disable = AuthOptions::parse([
        OsString::from("renewal"),
        OsString::from("provisioner"),
        OsString::from("disable"),
        OsString::from("local"),
        OsString::from("rrkah-fqaaa-aaaaa-aaaaq-cai"),
    ])
    .expect("parse auth renewal provisioner disable options");
    let AuthCommand::RenewalProvisionerUpsert(options) = disable else {
        panic!("expected renewal provisioner upsert command");
    };
    assert_eq!(options.deployment, "local");
    assert_eq!(options.principal, "rrkah-fqaaa-aaaaa-aaaaq-cai");
    assert!(!options.enabled);
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
        OsString::from("run-once"),
    ])
    .expect_err("missing deployment should be parsed after global options");

    assert!(err.to_string().contains("Usage: canic auth"));
}

#[test]
fn parses_work_batches_from_json_and_candid() {
    let json = serde_json::json!({
        "batches": [{
            "batch_id": vec![7_u8; 32],
            "attempt_count": "2",
            "attempts": []
        }]
    })
    .to_string();
    assert_eq!(
        parse_work_batches(&json),
        Some(vec![AuthRenewalBatchWork {
            batch_id: [7; 32],
            attempt_count: Some(2),
        }])
    );

    let candid = r#"{"response_candid":"(record { batches = vec { record { batch_id = blob \"\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\"; attempt_count = 1 : nat64; attempts = vec {} } } })"}"#;
    assert_eq!(
        parse_work_batches(candid),
        Some(vec![AuthRenewalBatchWork {
            batch_id: [8; 32],
            attempt_count: Some(1),
        }])
    );
}

#[test]
fn parses_renewal_provisioners_from_json_and_candid() {
    let json = serde_json::json!({
        "provisioners": [{
            "principal": "rrkah-fqaaa-aaaaa-aaaaq-cai",
            "enabled": true
        }]
    })
    .to_string();
    assert_eq!(
        parse_renewal_provisioners(&json),
        Some(vec![AuthRenewalProvisioner {
            principal: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
            enabled: true,
        }])
    );

    let candid = r#"{"response_candid":"(record { provisioners = vec { record { \"principal\" = principal \"rrkah-fqaaa-aaaaa-aaaaq-cai\"; enabled = false } } })"}"#;
    assert_eq!(
        parse_renewal_provisioners(candid),
        Some(vec![AuthRenewalProvisioner {
            principal: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
            enabled: false,
        }])
    );
}

#[test]
fn run_once_retrieves_and_installs_scheduled_batches() {
    let runtime = ScriptedAuthRenewalRuntime::new([
        scripted_response(
            CANIC_DELEGATION_RENEWAL_WORK,
            None,
            Some("json"),
            serde_json::json!({
                "batches": [{
                    "batch_id": vec![9_u8; 32],
                    "attempt_count": 1,
                    "attempts": []
                }]
            })
            .to_string(),
        ),
        scripted_response(
            CANIC_GET_DELEGATION_RENEWAL_PROOF_BATCH,
            Some(root_delegation_renewal_batch_get_arg([9; 32])),
            None,
            "(record { batch_id = blob \"\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\"; proofs = vec {} })".to_string(),
        ),
        scripted_response(
            CANIC_INSTALL_DELEGATION_PROOF_BATCH,
            Some("(record { batch_id = blob \"\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\"; proofs = vec {} })".to_string()),
            Some("json"),
            "{}".to_string(),
        ),
    ]);
    let result = renewal_once_result_with_runtime(
        &runtime,
        &RenewalRunOnceOptions {
            deployment: "local".to_string(),
            json: true,
            common: CommonOptions {
                network: "local".to_string(),
                icp: "icp".to_string(),
            },
        },
    )
    .expect("run-once should retrieve and install scripted batch");

    assert_eq!(result.status, AUTH_RENEWAL_STATUS_INSTALLED);
    assert_eq!(result.schema_version, AUTH_RENEWAL_RUN_ONCE_SCHEMA_VERSION);
    assert_eq!(result.batches.len(), 1);
    assert_eq!(result.batches[0].batch_id, hex_bytes(&[9; 32]));
    assert_eq!(
        runtime.called_methods(),
        vec![
            CANIC_DELEGATION_RENEWAL_WORK,
            CANIC_GET_DELEGATION_RENEWAL_PROOF_BATCH,
            CANIC_INSTALL_DELEGATION_PROOF_BATCH,
        ]
    );
}

#[test]
fn run_once_noops_when_no_work_is_scheduled() {
    let runtime = ScriptedAuthRenewalRuntime::new([scripted_response(
        CANIC_DELEGATION_RENEWAL_WORK,
        None,
        Some("json"),
        serde_json::json!({ "batches": [] }).to_string(),
    )]);
    let result = renewal_once_result_with_runtime(
        &runtime,
        &RenewalRunOnceOptions {
            deployment: "local".to_string(),
            json: false,
            common: CommonOptions {
                network: "local".to_string(),
                icp: "icp".to_string(),
            },
        },
    )
    .expect("run-once should tolerate empty work");

    assert_eq!(result.status, AUTH_RENEWAL_STATUS_NO_WORK);
    assert!(result.batches.is_empty());
    assert_eq!(
        runtime.called_methods(),
        vec![CANIC_DELEGATION_RENEWAL_WORK]
    );
}

#[test]
fn renewal_provisioner_list_queries_acl_endpoint() {
    let runtime = ScriptedAuthRenewalRuntime::new([scripted_response(
        CANIC_DELEGATION_RENEWAL_PROVISIONERS,
        None,
        Some("json"),
        serde_json::json!({
            "provisioners": [{
                "principal": "rrkah-fqaaa-aaaaa-aaaaq-cai",
                "enabled": true
            }]
        })
        .to_string(),
    )]);

    let result = renewal_provisioner_list_result_with_runtime(
        &runtime,
        &RenewalProvisionerListOptions {
            deployment: "local".to_string(),
            json: true,
            common: CommonOptions {
                network: "local".to_string(),
                icp: "icp".to_string(),
            },
        },
    )
    .expect("provisioner list should query scripted endpoint");

    assert_eq!(result.kind, AUTH_RENEWAL_PROVISIONER_LIST_KIND);
    assert_eq!(result.provisioners.len(), 1);
    assert_eq!(
        result.provisioners[0].principal,
        "rrkah-fqaaa-aaaaa-aaaaq-cai"
    );
    assert!(result.provisioners[0].enabled);
    assert_eq!(
        runtime.called_methods(),
        vec![CANIC_DELEGATION_RENEWAL_PROVISIONERS]
    );
}

#[test]
fn renewal_provisioner_upsert_calls_acl_endpoint() {
    let principal = "rrkah-fqaaa-aaaaa-aaaaq-cai";
    let runtime = ScriptedAuthRenewalRuntime::new([scripted_response(
        CANIC_UPSERT_DELEGATION_RENEWAL_PROVISIONER,
        Some(renewal_provisioner_upsert_arg(principal, true)),
        Some("json"),
        serde_json::json!({
            "provisioner": {
                "principal": principal,
                "enabled": true
            }
        })
        .to_string(),
    )]);

    let result = renewal_provisioner_upsert_result_with_runtime(
        &runtime,
        &RenewalProvisionerUpsertOptions {
            deployment: "local".to_string(),
            principal: principal.to_string(),
            enabled: true,
            json: true,
            common: CommonOptions {
                network: "local".to_string(),
                icp: "icp".to_string(),
            },
        },
    )
    .expect("provisioner upsert should call scripted endpoint");

    assert_eq!(result.kind, AUTH_RENEWAL_PROVISIONER_UPSERT_KIND);
    assert_eq!(result.provisioner.principal, principal);
    assert!(result.provisioner.enabled);
    assert_eq!(
        runtime.called_methods(),
        vec![CANIC_UPSERT_DELEGATION_RENEWAL_PROVISIONER]
    );
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

    assert_eq!(result.kind, AUTH_RENEWAL_STATUS_KIND);
    assert_eq!(result.schema_version, AUTH_RENEWAL_STATUS_SCHEMA_VERSION);
    assert_eq!(result.issuer_pid, issuer);
    assert_eq!(result.status, AUTH_RENEWAL_STATUS_ACTIVE_ATTEMPT);
    assert_eq!(result.renewal.template.enabled, Some(true));
    assert_eq!(
        result.issuer_observation.status,
        AUTH_RENEWAL_STATUS_UNAVAILABLE
    );
    assert_eq!(
        result.issuer_observation.reason.as_deref(),
        Some("issuer_not_in_local_registry")
    );
    assert_eq!(
        result.renewal.state.last_outcome.as_deref(),
        Some("installed")
    );
    assert_eq!(
        result.renewal.active_attempt.status.as_deref(),
        Some("prepared")
    );
    assert_eq!(
        runtime.called_methods(),
        vec![CANIC_ROOT_ISSUER_RENEWAL_STATUS]
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

    assert_eq!(result.status, AUTH_RENEWAL_STATUS_CONFIGURED);
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

    assert_eq!(result.status, AUTH_RENEWAL_STATUS_DRIFT_DETECTED);
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
                CANIC_DELEGATION_RENEWAL_WORK => CANIC_DELEGATION_RENEWAL_WORK,
                CANIC_GET_DELEGATION_RENEWAL_PROOF_BATCH => {
                    CANIC_GET_DELEGATION_RENEWAL_PROOF_BATCH
                }
                CANIC_INSTALL_DELEGATION_PROOF_BATCH => CANIC_INSTALL_DELEGATION_PROOF_BATCH,
                CANIC_ROOT_ISSUER_RENEWAL_STATUS => CANIC_ROOT_ISSUER_RENEWAL_STATUS,
                CANIC_ACTIVE_DELEGATION_PROOF_STATUS => CANIC_ACTIVE_DELEGATION_PROOF_STATUS,
                CANIC_DELEGATION_RENEWAL_PROVISIONERS => CANIC_DELEGATION_RENEWAL_PROVISIONERS,
                CANIC_UPSERT_DELEGATION_RENEWAL_PROVISIONER => {
                    CANIC_UPSERT_DELEGATION_RENEWAL_PROVISIONER
                }
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
                candid_source: AUTH_RENEWAL_CANDID_SOURCE_INSTALLED_DEPLOYMENT.to_string(),
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

    fn call_output(
        &self,
        _options: &CommonOptions,
        _target: &AuthRootCallTarget,
        method: &str,
        arg: &str,
        output: Option<&str>,
    ) -> Result<String, AuthCommandError> {
        Ok(self.call(method, Some(arg), output))
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
                    candid_source: AUTH_RENEWAL_CANDID_SOURCE_INSTALLED_DEPLOYMENT.to_string(),
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
