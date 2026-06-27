use super::*;
use crate::test_support::temp_dir;
use std::fs;

// Ensure medic options parse the deployment target, network, and ICP CLI selectors.
#[test]
fn parses_medic_options() {
    let options = MedicOptions::parse_info([
        OsString::from("demo"),
        OsString::from(crate::cli::globals::INTERNAL_NETWORK_OPTION),
        OsString::from("local"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/tmp/icp"),
    ])
    .expect("parse medic options");

    assert_eq!(options.deployment, "demo");
    assert_eq!(options.blob_storage, None);
    assert_eq!(options.auth_renewal, None);
    assert_eq!(options.network, "local");
    assert_eq!(options.icp, "/tmp/icp");
}

// Ensure targeted blob-storage medic diagnostics are opt-in.
#[test]
fn parses_blob_storage_medic_target() {
    let options = MedicOptions::parse_info([
        OsString::from("demo"),
        OsString::from("--blob-storage"),
        OsString::from("backend"),
    ])
    .expect("parse medic options");

    assert_eq!(options.deployment, "demo");
    assert_eq!(options.blob_storage.as_deref(), Some("backend"));
}

// Ensure targeted auth-renewal medic diagnostics are opt-in.
#[test]
fn parses_auth_renewal_medic_target() {
    let options = MedicOptions::parse_info([
        OsString::from("demo"),
        OsString::from("--auth-renewal"),
        OsString::from("rrkah-fqaaa-aaaaa-aaaaq-cai"),
    ])
    .expect("parse medic options");

    assert_eq!(options.deployment, "demo");
    assert_eq!(
        options.auth_renewal.as_deref(),
        Some("rrkah-fqaaa-aaaaa-aaaaq-cai")
    );
}

// Ensure medic help explains the diagnostic command rather than printing a one-liner.
#[test]
fn medic_usage_includes_examples() {
    let text = info_usage();

    assert!(text.contains("Diagnose local Canic deployment target setup"));
    assert!(text.contains("Usage: canic info medic [OPTIONS] <deployment>"));
    assert!(text.contains("<deployment>"));
    assert!(text.contains("--blob-storage <canister-or-role>"));
    assert!(text.contains("--auth-renewal <issuer-principal>"));
    assert!(!text.contains("--fleet <name>"));
    assert!(!text.contains("--network"));
    assert!(!text.contains("--icp"));
    assert!(text.contains("Examples:"));
}

// Ensure the medic report is stable, readable block text instead of a wide table.
#[test]
fn renders_medic_report() {
    let report = render_medic_report(&[
        MedicCheck::ok("network", "local", "-"),
        MedicCheck::warn(
            "deployment state",
            "no installed deployment found",
            "run canic install",
        ),
    ]);

    assert!(report.starts_with("network [ok]"));
    assert!(report.contains("\n  detail: local\n"));
    assert!(report.contains("deployment state [warn]"));
    assert!(report.contains("  next: run canic install"));
    assert!(!report.contains("CHECK"));
}

// Ensure blob-storage medic uses the shared status summary without reinterpreting readiness.
#[test]
fn renders_blob_storage_medic_summary() {
    let check = blob_storage_medic_check_from_summary(BlobStorageMedicSummary {
        status: BlobStorageMedicStatus::Blocked,
        detail: "readiness=blocked; configured=true; gateways=0; funding=funding_needed"
            .to_string(),
        next: "canic blob-storage sync-gateways demo backend".to_string(),
    });
    let report = render_medic_report(&[check]);

    assert!(report.contains("blob-storage billing [warn]"));
    assert!(report.contains("readiness=blocked"));
    assert!(report.contains("canic blob-storage sync-gateways demo backend"));
}

// Ensure auth-renewal medic uses the shared auth summary without mutating renewal state.
#[test]
fn renders_auth_renewal_medic_summary() {
    let check = auth_renewal_medic_check_from_summary(AuthRenewalMedicSummary {
        status: AuthRenewalMedicStatus::Warning,
        detail: "status=drift_detected; issuer_observation=drift_detected; drift_detected=true"
            .to_string(),
        next: "canic auth renewal status demo --issuer rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
    });
    let report = render_medic_report(&[check]);

    assert!(report.contains("auth renewal [warn]"));
    assert!(report.contains("status=drift_detected"));
    assert!(report.contains("canic auth renewal status demo --issuer"));
}

// Ensure default medic can discover blob-storage-capable local Candid sidecars passively.
#[test]
fn passive_blob_storage_hint_uses_local_candid_only() {
    let root = temp_dir("canic-cli-medic-blob-storage-passive");
    write_candid(
        &root,
        "local",
        "backend",
        r#"
            service : {
                get_blob_storage_status : () -> () query;
                "_immutableObjectStorageUpdateGatewayPrincipals" : () -> ();
                "_immutableObjectStorageFundFromProjectCycles" : (nat) -> ();
            }
        "#,
    );
    write_candid(
        &root,
        "local",
        "other",
        r"
            service : {
                get_blob_storage_status : () -> () query;
            }
        ",
    );
    write_candid(
        &root,
        "local",
        "partial",
        r#"
            service : {
                get_blob_storage_status : () -> () query;
                "_immutableObjectStorageUpdateGatewayPrincipals" : () -> ();
            }
        "#,
    );

    let roles = blob_storage_billing_roles_from_candid_dir(&root, "local");
    let options = MedicOptions {
        deployment: "demo".to_string(),
        blob_storage: None,
        auth_renewal: None,
        network: "local".to_string(),
        icp: "icp".to_string(),
    };
    let check = check_blob_storage_passive_hint(&options, &root).expect("passive hint");

    assert_eq!(roles, vec!["backend".to_string()]);
    assert_eq!(check.status, MedicStatus::Ok);
    assert!(check.detail.contains("backend"));
    assert_eq!(
        check.next,
        "run canic info medic demo --blob-storage backend"
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure passive Candid detection only accepts the full billing endpoint trio.
#[test]
fn blob_storage_passive_detection_rejects_partial_or_unrelated_candid() {
    assert!(candid_declares_blob_storage_billing(
        r#"
            service : {
                get_blob_storage_status : () -> () query;
                "_immutableObjectStorageUpdateGatewayPrincipals" : () -> ();
                "_immutableObjectStorageFundFromProjectCycles" : (nat) -> ();
            }
        "#
    ));
    assert!(!candid_declares_blob_storage_billing(
        r#"
            service : {
                get_blob_storage_status : () -> () query;
                "_immutableObjectStorageUpdateGatewayPrincipals" : () -> ();
            }
        "#
    ));
    assert!(!candid_declares_blob_storage_billing(
        r"
            service : {
                canic_ready : () -> (bool) query;
            }
        "
    ));
}

// Ensure long medic details and next actions wrap to terminal-readable lines.
#[test]
fn wraps_long_medic_report_fields() {
    let report = render_medic_report(&[MedicCheck::warn(
        "deployment state",
        "this is a deliberately long diagnostic message that should wrap across multiple indented lines instead of widening a terminal table",
        "run canic install <fleet-template> or canic deploy register <deployment> --fleet-template <fleet-template> --root <principal> --allow-unverified",
    )]);

    assert!(report.contains("deployment state [warn]"));
    assert!(
        report
            .lines()
            .all(|line| line.chars().count() <= MEDIC_REPORT_WIDTH)
    );
    assert!(
        report
            .lines()
            .any(|line| line.starts_with("          ") && !line.trim().is_empty())
    );
}

// Ensure ICP identity session guidance stays informational and versionless.
#[test]
fn icp_identity_session_cache_hint_is_informational() {
    let check = check_icp_identity_session_cache_hint();

    assert_eq!(check.status, MedicStatus::Ok);
    assert_eq!(check.name, "icp identity session");
    assert!(check.detail.contains("PEM identities"));
    assert!(check.next.contains("icp settings session-length"));
    assert!(check.next.contains("icp identity reauth"));
    assert!(!check.next.contains("1.0.0"));
}

// Ensure host installed-deployment missing-state errors remain warnings, not failures.
#[test]
fn missing_installed_deployment_error_is_warnable() {
    assert!(is_missing_installed_deployment(
        "deployment target demo is not installed on network local"
    ));
    assert!(!is_missing_installed_deployment(
        "failed to read canic deployment state: bad json"
    ));
}

fn write_candid(root: &std::path::Path, network: &str, role: &str, candid: &str) {
    let path = local_canister_candid_path(root, network, role);
    fs::create_dir_all(path.parent().expect("candid parent")).expect("create candid parent");
    fs::write(path, candid).expect("write candid");
}
