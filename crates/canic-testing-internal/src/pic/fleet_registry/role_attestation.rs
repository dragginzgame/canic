//! Module: pic::fleet_registry::role_attestation
//!
//! Responsibility: assert Registry-bound role-attestation behavior in PocketIC.
//! Does not own: fixture construction, Component lifecycle, or proof implementation.
//! Boundary: consumes an active Registry-issued Component binding from the Fleet journey.

use canic::{
    dto::{
        auth::{RoleAttestationGetRequest, RoleAttestationRequest, SignedRoleAttestation},
        error::Error,
        metrics::{MetricEntry, MetricValue, MetricsKind},
        page::PageRequest,
        role::MetricsStatusRequest,
        rpc::RootRequestMetadata,
    },
    ids::{CanisterRole, ComponentBinding},
};
use canic_contracts::{
    dto::role_attestation as native_attestation, protocol::role_attestation as attestation_wire,
};
use ic_platform::pocket_ic::PocketIc;
use ic_platform::principal::ExternalPrincipal as Principal;
use std::time::Duration;

/// Exercise issuance, verification, and guard metrics through an active issuer Component.
pub(super) fn assert_registry_bound_role_attestation(
    pic: &PocketIc,
    root: Principal,
    issuer: &ComponentBinding,
    verifier: &ComponentBinding,
) {
    assert_role_attestation_admission(pic, root, issuer);
    assert_role_attestation_verification(pic, root, issuer, verifier);
    assert_issuer_guard_metrics(pic, root, issuer.canister_id);
}

fn assert_role_attestation_admission(pic: &PocketIc, root: Principal, issuer: &ComponentBinding) {
    let mut subject_drift =
        role_attestation_request(issuer, issuer.canister_id, 60_000_000_000, 11);
    subject_drift.subject = root;
    assert_role_prepare_forbidden(
        pic,
        root,
        issuer.canister_id,
        subject_drift,
        canic_contracts::diagnostics::codes::AUTHORITY_CONFLICT.raw_code(),
    );

    let mut role_drift = role_attestation_request(issuer, issuer.canister_id, 60_000_000_000, 12);
    role_drift.role = CanisterRole::from("other_role");
    assert_role_prepare_forbidden(
        pic,
        root,
        issuer.canister_id,
        role_drift,
        canic_contracts::diagnostics::codes::AUTHORITY_CONFLICT.raw_code(),
    );

    let mut subnet_drift = role_attestation_request(issuer, issuer.canister_id, 60_000_000_000, 13);
    subnet_drift.subnet_id = Some(Principal::from_slice(&[0x61; 29]));
    assert_role_prepare_forbidden(
        pic,
        root,
        issuer.canister_id,
        subnet_drift,
        canic_contracts::diagnostics::codes::AUTHORITY_CONFLICT.raw_code(),
    );

    let mut unregistered = role_attestation_request(issuer, issuer.canister_id, 60_000_000_000, 14);
    unregistered.subject = Principal::anonymous();
    assert_role_prepare_forbidden(
        pic,
        root,
        Principal::anonymous(),
        unregistered,
        canic_contracts::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code(),
    );
}

fn assert_role_attestation_verification(
    pic: &PocketIc,
    root: Principal,
    issuer: &ComponentBinding,
    verifier: &ComponentBinding,
) {
    let attestation =
        issue_role_attestation(pic, root, issuer, verifier.canister_id, 60_000_000_000, 21);
    verify_role_attestation(
        pic,
        verifier.canister_id,
        issuer.canister_id,
        attestation.clone(),
        0,
    )
    .expect("fresh Registry-bound role attestation");
    require_attested_local_subnet(pic, verifier.canister_id, issuer.canister_id, attestation)
        .expect("attested caller placement must match the receiver's live Subnet");

    let mut without_subnet =
        role_attestation_request(issuer, verifier.canister_id, 60_000_000_000, 26);
    without_subnet.subnet_id = None;
    let without_subnet = issue_requested_role_attestation(pic, root, issuer, without_subnet);
    let missing_subnet = require_attested_local_subnet(
        pic,
        verifier.canister_id,
        issuer.canister_id,
        without_subnet,
    )
    .expect_err("local-Subnet access must reject an attestation without a Subnet claim");
    assert_eq!(
        missing_subnet.code(),
        canic_contracts::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
    );

    let attestation =
        issue_role_attestation(pic, root, issuer, verifier.canister_id, 60_000_000_000, 22);
    let caller_mismatch = verify_role_attestation(
        pic,
        verifier.canister_id,
        Principal::anonymous(),
        attestation,
        0,
    )
    .expect_err("role attestation caller mismatch must fail");
    assert_eq!(
        caller_mismatch.code(),
        canic_contracts::diagnostics::codes::AUTHORITY_CONFLICT.raw_code()
    );

    let attestation = issue_role_attestation(pic, root, issuer, root, 60_000_000_000, 23);
    let audience_mismatch = verify_role_attestation(
        pic,
        verifier.canister_id,
        issuer.canister_id,
        attestation,
        0,
    )
    .expect_err("role attestation audience mismatch must fail");
    assert_eq!(
        audience_mismatch.code(),
        canic_contracts::diagnostics::codes::AUTHORITY_CONFLICT.raw_code()
    );

    let attestation =
        issue_role_attestation(pic, root, issuer, verifier.canister_id, 60_000_000_000, 24);
    let epoch_mismatch = verify_role_attestation(
        pic,
        verifier.canister_id,
        issuer.canister_id,
        attestation,
        issuer.authority.epoch.saturating_add(1),
    )
    .expect_err("role attestation epoch floor mismatch must fail");
    assert_eq!(
        epoch_mismatch.code(),
        canic_contracts::diagnostics::codes::VERSION_INACTIVE.raw_code()
    );

    let attestation =
        issue_role_attestation(pic, root, issuer, verifier.canister_id, 1_000_000_000, 25);
    pic.advance_time(Duration::from_secs(2));
    pic.tick();
    let expired = verify_role_attestation(
        pic,
        verifier.canister_id,
        issuer.canister_id,
        attestation,
        0,
    )
    .expect_err("expired role attestation must fail");
    assert_eq!(
        expired.code(),
        canic_contracts::diagnostics::codes::SECURITY_EXPIRED.raw_code()
    );
}

fn role_attestation_request(
    issuer: &ComponentBinding,
    audience: Principal,
    ttl_ns: u64,
    request_id_seed: u8,
) -> RoleAttestationRequest {
    RoleAttestationRequest {
        subject: issuer.canister_id,
        role: issuer.role.clone(),
        subnet_id: Some(issuer.placement_subnet.into_principal()),
        audience,
        ttl_ns,
        epoch: issuer.authority.epoch,
        metadata: Some(RootRequestMetadata {
            request_id: [request_id_seed; 32],
            ttl_ns: 60_000_000_000,
        }),
    }
}

fn assert_role_prepare_forbidden(
    pic: &PocketIc,
    root: Principal,
    caller: Principal,
    request: RoleAttestationRequest,
    expected_code: canic_contracts::diagnostics::DiagnosticCode,
) {
    let response = prepare_attestation(pic, root, caller, request);
    assert_eq!(
        response
            .expect_err("invalid Component attestation request must fail")
            .code(),
        expected_code
    );
}

fn issue_role_attestation(
    pic: &PocketIc,
    root: Principal,
    issuer: &ComponentBinding,
    audience: Principal,
    ttl_ns: u64,
    request_id_seed: u8,
) -> SignedRoleAttestation {
    issue_requested_role_attestation(
        pic,
        root,
        issuer,
        role_attestation_request(issuer, audience, ttl_ns, request_id_seed),
    )
}

fn issue_requested_role_attestation(
    pic: &PocketIc,
    root: Principal,
    issuer: &ComponentBinding,
    request: RoleAttestationRequest,
) -> SignedRoleAttestation {
    let prepared = prepare_attestation(pic, root, issuer.canister_id, request)
        .expect("role attestation prepare");
    let request = RoleAttestationGetRequest {
        payload_hash: prepared.payload_hash,
    };
    let bytes = ic_wire::native::encode_request(
        attestation_wire::get::METHOD_ID,
        &request,
        attestation_wire::REQUEST_LIMITS,
    )
    .unwrap();
    let reply = ic_testkit::pic::try_pocket_ic_operation(|| {
        pic.query_call(
            root,
            issuer.canister_id,
            attestation_wire::get::METHOD,
            bytes,
        )
    })
    .unwrap()
    .unwrap();
    let signed: Result<native_attestation::SignedRoleAttestation, Error> =
        ic_wire::native::decode_response(
            attestation_wire::get::METHOD_ID,
            &reply,
            attestation_wire::RESPONSE_LIMITS,
        )
        .unwrap();
    signed.expect("role attestation retrieval").into()
}

fn prepare_attestation(
    pic: &PocketIc,
    root: Principal,
    caller: Principal,
    request: RoleAttestationRequest,
) -> Result<native_attestation::RoleAttestationPrepareResponse, Error> {
    let request = native_attestation::RoleAttestationRequest::from(request);
    let bytes = ic_wire::native::encode_request(
        attestation_wire::prepare::METHOD_ID,
        &request,
        attestation_wire::REQUEST_LIMITS,
    )
    .unwrap();
    let reply = ic_testkit::pic::try_pocket_ic_operation(|| {
        pic.update_call(root, caller, attestation_wire::prepare::METHOD, bytes)
    })
    .unwrap()
    .unwrap();
    ic_wire::native::decode_response(
        attestation_wire::prepare::METHOD_ID,
        &reply,
        attestation_wire::RESPONSE_LIMITS,
    )
    .unwrap()
}

fn require_attested_local_subnet(
    pic: &PocketIc,
    verifier: Principal,
    caller: Principal,
    attestation: SignedRoleAttestation,
) -> Result<(), Error> {
    ic_testkit::pic::update_wire(
        pic,
        verifier,
        caller,
        crate::pic::issuer_probe::ATTESTED,
        &canic::dto::role_attestation::SignedRoleAttestation::from(attestation),
    )
    .expect("attested local-Subnet access transport")
}

fn verify_role_attestation(
    pic: &PocketIc,
    verifier: Principal,
    caller: Principal,
    attestation: SignedRoleAttestation,
    minimum_epoch: u64,
) -> Result<(), Error> {
    ic_testkit::pic::update_wire(
        pic,
        verifier,
        caller,
        crate::pic::issuer_probe::VERIFY,
        &crate::pic::issuer_probe::VerifyRequest {
            attestation: attestation.into(),
            min_accepted_epoch: minimum_epoch,
        },
    )
    .expect("role attestation verification transport")
}

fn assert_issuer_guard_metrics(pic: &PocketIc, root: Principal, issuer: Principal) {
    let denial_labels = ["access", "issuer_guard_is_root", "auth", "caller_is_root"];
    let before_denial =
        metric_count_for_labels(pic, root, issuer, MetricsKind::Security, &denial_labels);
    let denied: Result<(), Error> = ic_testkit::pic::update_wire(
        pic,
        issuer,
        Principal::anonymous(),
        crate::pic::issuer_probe::ROOT_GUARD,
        &(),
    )
    .expect("issuer root-guard denial transport");
    assert_eq!(
        denied.expect_err("anonymous root guard").code(),
        canic_contracts::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
    );
    assert_eq!(
        metric_count_for_labels(pic, root, issuer, MetricsKind::Security, &denial_labels),
        before_denial.saturating_add(1)
    );

    let success_labels = ["perf", "endpoint", "update", "issuer_guard_is_root"];
    let before_success =
        metric_count_for_labels(pic, root, issuer, MetricsKind::Runtime, &success_labels);
    let allowed: Result<(), Error> =
        ic_testkit::pic::update_wire(pic, issuer, root, crate::pic::issuer_probe::ROOT_GUARD, &())
            .expect("issuer root-guard success transport");
    allowed.expect("Fleet Subnet Root caller must satisfy issuer root guard");
    assert_eq!(
        metric_count_for_labels(pic, root, issuer, MetricsKind::Runtime, &success_labels),
        before_success.saturating_add(1)
    );
    let row = query_metric_entries(pic, root, issuer, MetricsKind::Runtime)
        .into_iter()
        .find(|entry| labels_match(entry, &success_labels))
        .expect("issuer root-guard endpoint perf metric");
    assert!(row.principal.is_none());
    assert!(matches!(row.value, MetricValue::Measurement { .. }));
}

fn metric_count_for_labels(
    pic: &PocketIc,
    caller: Principal,
    canister: Principal,
    kind: MetricsKind,
    labels: &[&str],
) -> u64 {
    query_metric_entries(pic, caller, canister, kind)
        .into_iter()
        .find_map(|entry| {
            labels_match(&entry, labels).then_some(match entry.value {
                MetricValue::Count { count, exact: true } => {
                    u64::try_from(count).expect("bounded exact count")
                }
                MetricValue::Measurement {
                    count,
                    count_overflowed: false,
                    ..
                } => count,
                MetricValue::Count { .. } | MetricValue::Measurement { .. } => {
                    panic!("role attestation requires an exact sample count")
                }
                MetricValue::U128 { .. } => 0,
            })
        })
        .unwrap_or(0)
}

fn query_metric_entries(
    pic: &PocketIc,
    caller: Principal,
    canister: Principal,
    kind: MetricsKind,
) -> Vec<MetricEntry> {
    let response: Result<canic::dto::page::Page<canic::dto::metric_page::MetricEntry>, Error> =
        ic_testkit::pic::query_wire(
            pic,
            canister,
            caller,
            ic_testkit::pic::WireCallSpec {
                method: canic::protocol::protected_metrics::METHOD,
                method_id: canic::protocol::protected_metrics::METHOD_ID,
                request_limits: canic::protocol::protected_metrics::REQUEST_LIMITS,
                response_limits: canic::protocol::protected_metrics::RESPONSE_LIMITS,
            },
            &MetricsStatusRequest {
                kind,
                page: PageRequest {
                    limit: 10_000,
                    offset: 0,
                },
            },
        )
        .expect("controller metrics Wire transport");
    response
        .expect("controller metrics application result")
        .entries
        .into_iter()
        .map(Into::into)
        .collect()
}

fn labels_match(entry: &MetricEntry, labels: &[&str]) -> bool {
    entry.labels.len() == labels.len()
        && entry
            .labels
            .iter()
            .zip(labels)
            .all(|(actual, expected)| actual == expected)
}
