use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use candid::{decode_one, encode_one};
use candid_parser::utils::CandidSource;
use canic::{
    cdk::types::Principal,
    dto::auth::{
        ActiveDelegationProofStatus, ActiveDelegationProofStatusResponse, AuthRequestMetadata,
        DelegatedRoleGrant, DelegationAudience, DelegationCert, DelegationProof,
        IcCanisterSignatureProofV1, IssuerProofAlgorithm, IssuerProofBinding,
        RootDelegationProofBatchEntry, RootDelegationProofBatchGetRequest,
        RootDelegationProofBatchGetResponse, RootDelegationProofBatchInstallRequest,
        RootDelegationProofBatchInstallResponse, RootDelegationProofBatchInstallResult,
        RootDelegationProofBatchPrepareEntry, RootDelegationProofBatchPrepareRequest,
        RootDelegationProofBatchPrepareResponse, RootDelegationProofBatchProof,
        RootDelegationProofBatchProofRef, RootDelegationProofInstallOutcome, RootProof,
    },
    ids::CanisterRole,
};

// Returns the repository root so wire-surface fixtures can be read from disk.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate directory should have a parent")
        .parent()
        .expect("workspace root should exist")
        .to_path_buf()
}

// Reads a checked-in protocol artifact so the test can pin the public surface.
fn read_text(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn assert_candid_roundtrip<T>(value: T)
where
    T: candid::CandidType + for<'de> candid::Deserialize<'de> + Eq + Debug,
{
    let encoded = encode_one(&value).expect("encode Candid value");
    let decoded = decode_one::<T>(&encoded).expect("decode Candid value");
    assert_eq!(decoded, value);
}

fn preceding_attribute<'a>(source: &'a str, signature: &str) -> &'a str {
    source
        .split(signature)
        .next()
        .unwrap_or_else(|| panic!("source should contain {signature}"))
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with("#["))
        .unwrap_or_else(|| panic!("{signature} should have a preceding attribute"))
}

#[test]
fn wasm_store_exposes_standard_cycle_tracker() {
    let did_path = workspace_root().join("crates/canic-wasm-store/wasm_store.did");
    let did = read_text(&did_path);

    assert!(
        did.contains("type PageRequest = record { offset : nat64; limit : nat64 };")
            && did.contains("  canic_cycle_tracker : (PageRequest) -> ("),
        "missing `canic_cycle_tracker` method in {}",
        did_path.display()
    );
    assert!(
        did.contains("type CycleTopupEvent = record")
            && did.contains("  canic_cycle_topups : (PageRequest) -> ("),
        "missing `canic_cycle_topups` method in {}",
        did_path.display()
    );
}

#[test]
fn wasm_store_excludes_default_memory_diagnostics() {
    let did_path = workspace_root().join("crates/canic-wasm-store/wasm_store.did");
    let did = read_text(&did_path);

    assert!(
        !did.contains("type MemoryLedgerResponse = record")
            && !did.contains("  canic_memory_ledger :"),
        "unexpected default `canic_memory_ledger` method in {}",
        did_path.display()
    );
    assert!(
        !did.contains("  canic_memory_registry :"),
        "unexpected `canic_memory_registry` method in {}",
        did_path.display()
    );
}

#[test]
fn wasm_store_canonical_did_parses() {
    let did_path = workspace_root().join("crates/canic-wasm-store/wasm_store.did");
    let did = read_text(&did_path);
    let (env, actor) = CandidSource::Text(&did)
        .load()
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", did_path.display()));

    let actor = actor.unwrap_or_else(|| panic!("missing service in {}", did_path.display()));
    let service = env
        .as_service(&actor)
        .unwrap_or_else(|err| panic!("invalid service in {}: {err}", did_path.display()));

    assert!(
        service
            .iter()
            .all(|(name, _)| name != "canic_memory_ledger"),
        "parsed default wasm_store service must not include canic_memory_ledger"
    );
}

#[test]
fn public_protocol_reexports_wasm_store_root_update_manifest() {
    assert_eq!(
        canic::protocol::CANIC_WASM_STORE_ROOT_UPDATE_METHODS,
        canic_core::protocol::CANIC_WASM_STORE_ROOT_UPDATE_METHODS
    );
    assert_eq!(
        canic::protocol::CANIC_WASM_STORE_STRUCTURAL_QUERY_METHODS,
        canic_core::protocol::CANIC_WASM_STORE_STRUCTURAL_QUERY_METHODS
    );

    for method in canic::protocol::CANIC_WASM_STORE_ROOT_UPDATE_METHODS {
        assert!(!canic::protocol::canic_wasm_store_method_requires_internal_proof(method));
    }
    for method in canic::protocol::CANIC_WASM_STORE_STRUCTURAL_QUERY_METHODS {
        assert!(!canic::protocol::canic_wasm_store_method_requires_internal_proof(method));
    }
}

#[test]
fn active_delegation_proof_installer_surface_is_issuer_gated() {
    assert_eq!(
        canic::protocol::CANIC_ACTIVE_DELEGATION_PROOF_STATUS,
        canic_core::protocol::CANIC_ACTIVE_DELEGATION_PROOF_STATUS
    );
    assert_eq!(
        canic::protocol::CANIC_INSTALL_ACTIVE_DELEGATION_PROOF,
        canic_core::protocol::CANIC_INSTALL_ACTIVE_DELEGATION_PROOF
    );
    assert_eq!(
        canic::protocol::CANIC_INSTALL_ACTIVE_DELEGATION_PROOF,
        "canic_install_active_delegation_proof"
    );

    let bundle_path = workspace_root().join("crates/canic/src/macros/endpoints/bundles.rs");
    let bundle = read_text(&bundle_path);
    assert!(
        bundle.contains("#[cfg(canic_delegated_token_issuer)]\n        $crate::canic_emit_nonroot_auth_attestation_endpoints!();"),
        "non-root issuer provisioning endpoints must be gated by canic_delegated_token_issuer"
    );

    let endpoint_path = workspace_root().join("crates/canic/src/macros/endpoints/nonroot.rs");
    let endpoint_source = read_text(&endpoint_path);
    let endpoint = endpoint_source
        .split("fn canic_install_active_delegation_proof(")
        .nth(1)
        .expect("non-root auth endpoint should emit active proof installer");
    let prefix = endpoint_source
        .split("fn canic_install_active_delegation_proof(")
        .next()
        .expect("source should have endpoint prefix");
    let preceding_attribute = prefix
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with("#["))
        .expect("active proof installer endpoint should have an attribute");

    assert!(
        preceding_attribute.contains("canic_update")
            && preceding_attribute.contains("caller::is_controller()"),
        "active proof installer must be a controller-gated update endpoint"
    );
    assert!(
        endpoint.contains("InstallActiveDelegationProofRequest")
            && endpoint.contains("InstallActiveDelegationProofResponse")
            && endpoint.contains("AuthApi::install_active_delegation_proof"),
        "active proof installer must call the auth API with the install DTOs"
    );
    assert!(
        endpoint_source.contains("fn canic_active_delegation_proof_status(")
            && endpoint_source.contains("ActiveDelegationProofStatusResponse")
            && endpoint_source.contains("AuthApi::active_delegation_proof_status"),
        "delegated-token issuer bundle must expose active proof status"
    );

    let did_path = workspace_root().join("crates/canic-wasm-store/wasm_store.did");
    let did = read_text(&did_path);
    assert!(
        !did.contains("canic_install_active_delegation_proof")
            && !did.contains("canic_active_delegation_proof_status")
            && !did.contains("canic_prepare_delegated_token")
            && !did.contains("canic_get_delegated_token")
            && !did.contains("type InstallActiveDelegationProofRequest = record")
            && !did.contains("type DelegatedTokenPrepareRequest = record"),
        "canonical wasm_store DID must not expose delegated-token issuer provisioning"
    );
}

#[test]
fn root_delegation_proof_batch_surface_is_pinned() {
    assert_eq!(
        canic::protocol::CANIC_PREPARE_DELEGATION_PROOF_BATCH,
        canic_core::protocol::CANIC_PREPARE_DELEGATION_PROOF_BATCH
    );
    assert_eq!(
        canic::protocol::CANIC_GET_DELEGATION_PROOF_BATCH,
        canic_core::protocol::CANIC_GET_DELEGATION_PROOF_BATCH
    );
    assert_eq!(
        canic::protocol::CANIC_INSTALL_DELEGATION_PROOF_BATCH,
        canic_core::protocol::CANIC_INSTALL_DELEGATION_PROOF_BATCH
    );
    assert_eq!(
        canic::protocol::CANIC_PREPARE_DELEGATION_PROOF_BATCH,
        "canic_prepare_delegation_proof_batch"
    );
    assert_eq!(
        canic::protocol::CANIC_GET_DELEGATION_PROOF_BATCH,
        "canic_get_delegation_proof_batch"
    );
    assert_eq!(
        canic::protocol::CANIC_INSTALL_DELEGATION_PROOF_BATCH,
        "canic_install_delegation_proof_batch"
    );

    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/root.rs");
    let source = read_text(&macro_path);
    assert!(
        !source.contains("fn canic_prepare_delegation_proof(")
            && !source.contains("fn canic_get_delegation_proof("),
        "legacy single-proof root delegation endpoints must stay removed"
    );
    let prepare_attr = preceding_attribute(&source, "fn canic_prepare_delegation_proof_batch(");
    let get_attr = preceding_attribute(&source, "fn canic_get_delegation_proof_batch(");
    let install_attr = preceding_attribute(&source, "fn canic_install_delegation_proof_batch(");
    assert!(
        prepare_attr.contains("canic_update")
            && prepare_attr.contains("caller::is_controller()")
            && !prepare_attr.contains("internal"),
        "root batch prepare must remain a public controller-gated update"
    );
    assert!(
        get_attr.contains("canic_query")
            && get_attr.contains("caller::is_controller()")
            && !get_attr.contains("internal")
            && !get_attr.contains("caller::is_registered_to_subnet()"),
        "root batch get must remain a direct controller-gated root query"
    );
    assert!(
        install_attr.contains("canic_update")
            && install_attr.contains("caller::is_controller()")
            && !install_attr.contains("internal"),
        "root batch install must remain a public controller-gated update"
    );
    assert!(
        source.contains("fn canic_prepare_delegation_proof_batch(")
            && source.contains("RootDelegationProofBatchPrepareRequest")
            && source.contains("RootDelegationProofBatchPrepareResponse")
            && source.contains("AuthApi::prepare_delegation_proof_batch_root"),
        "root auth endpoint bundle must expose batch prepare"
    );
    assert!(
        source.contains("fn canic_get_delegation_proof_batch(")
            && source.contains("RootDelegationProofBatchGetRequest")
            && source.contains("RootDelegationProofBatchGetResponse")
            && source.contains("AuthApi::get_delegation_proof_batch_root"),
        "root auth endpoint bundle must expose batch get"
    );
    assert!(
        source.contains("fn canic_install_delegation_proof_batch(")
            && source.contains("RootDelegationProofBatchInstallRequest")
            && source.contains("RootDelegationProofBatchInstallResponse")
            && source.contains("AuthApi::install_delegation_proof_batch_root"),
        "root auth endpoint bundle must expose batch install"
    );
}

#[test]
fn root_delegation_proof_batch_dtos_roundtrip_through_candid() {
    let issuer_pid = Principal::from_slice(&[17; 29]);
    let root_pid = Principal::from_slice(&[18; 29]);
    let batch_id = [19; 32];
    let cert_hash = [20; 32];
    let grant = DelegatedRoleGrant {
        target: CanisterRole::new("test"),
        scopes: vec!["verify".to_string()],
    };
    let audience = DelegationAudience::Project("test".to_string());
    let prepare_entry = RootDelegationProofBatchPrepareEntry {
        issuer_pid,
        aud: audience.clone(),
        grants: vec![grant.clone()],
        cert_ttl_ns: 60,
    };
    let prepare_request = RootDelegationProofBatchPrepareRequest {
        metadata: Some(AuthRequestMetadata {
            request_id: batch_id,
            ttl_ns: 30,
        }),
        entries: vec![prepare_entry.clone()],
    };
    let prepare_response = RootDelegationProofBatchPrepareResponse {
        batch_id,
        entries: vec![RootDelegationProofBatchEntry {
            issuer_pid,
            cert_hash,
            expires_at_ns: 90,
            refresh_after_ns: 72,
        }],
        retrieval_expires_at_ns: 45,
    };
    let proof_ref = RootDelegationProofBatchProofRef {
        issuer_pid,
        cert_hash,
    };
    let get_request = RootDelegationProofBatchGetRequest {
        batch_id,
        entries: vec![proof_ref],
    };
    let proof = DelegationProof {
        cert: DelegationCert {
            root_pid,
            issuer_pid,
            issuer_proof_alg: IssuerProofAlgorithm::IcCanisterSignatureV1,
            issuer_proof_binding_hash: [21; 32],
            issuer_proof_binding: IssuerProofBinding::IcCanisterSignatureV1 {
                seed_hash: [22; 32],
            },
            issued_at_ns: 1,
            not_before_ns: 1,
            expires_at_ns: 90,
            max_token_ttl_ns: 10,
            aud: audience,
            grants: vec![grant],
        },
        root_proof: RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
            signature_cbor: vec![1, 2, 3],
            public_key_der: vec![4, 5, 6],
        }),
    };
    let batch_proof = RootDelegationProofBatchProof {
        issuer_pid,
        cert_hash,
        proof,
    };
    let get_response = RootDelegationProofBatchGetResponse {
        batch_id,
        proofs: vec![batch_proof.clone()],
    };
    let install_request = RootDelegationProofBatchInstallRequest {
        batch_id,
        proofs: vec![batch_proof],
    };
    let install_response = RootDelegationProofBatchInstallResponse {
        batch_id,
        outcomes: vec![RootDelegationProofBatchInstallResult {
            issuer_pid,
            cert_hash,
            outcome: RootDelegationProofInstallOutcome::Installed,
        }],
    };
    let status = ActiveDelegationProofStatusResponse {
        status: ActiveDelegationProofStatus::RefreshNeeded,
        root_pid: Some(root_pid),
        issuer_pid: Some(issuer_pid),
        cert_hash: Some(cert_hash),
        expires_at_ns: Some(90),
        refresh_after_ns: Some(72),
    };

    assert_candid_roundtrip(prepare_entry);
    assert_candid_roundtrip(prepare_request);
    assert_candid_roundtrip(prepare_response);
    assert_candid_roundtrip(get_request);
    assert_candid_roundtrip(get_response);
    assert_candid_roundtrip(install_request);
    assert_candid_roundtrip(install_response);
    assert_candid_roundtrip(status);
}

#[test]
fn root_role_attestation_prepare_get_surface_is_pinned() {
    assert_eq!(
        canic::protocol::CANIC_PREPARE_ROLE_ATTESTATION,
        canic_core::protocol::CANIC_PREPARE_ROLE_ATTESTATION
    );
    assert_eq!(
        canic::protocol::CANIC_GET_ROLE_ATTESTATION,
        canic_core::protocol::CANIC_GET_ROLE_ATTESTATION
    );
    assert_eq!(
        canic::protocol::CANIC_PREPARE_ROLE_ATTESTATION,
        "canic_prepare_role_attestation"
    );
    assert_eq!(
        canic::protocol::CANIC_GET_ROLE_ATTESTATION,
        "canic_get_role_attestation"
    );

    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/root.rs");
    let source = read_text(&macro_path);
    assert!(
        source.contains("fn canic_prepare_role_attestation(")
            && source.contains("RoleAttestationPrepareResponse")
            && source.contains("AuthApi::prepare_role_attestation_root"),
        "root auth endpoint bundle must expose role-attestation prepare"
    );
    assert!(
        source.contains("fn canic_get_role_attestation(")
            && source.contains("RoleAttestationGetRequest")
            && source.contains("AuthApi::get_role_attestation_root"),
        "root auth endpoint bundle must expose role-attestation get"
    );
}

#[test]
fn memory_ledger_diagnostic_bypasses_normal_dispatch() {
    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/shared.rs");
    let source = read_text(&macro_path);
    let endpoint = source
        .split("fn canic_memory_ledger()")
        .nth(1)
        .expect("memory ledger endpoint should exist");
    let prefix = source
        .split("fn canic_memory_ledger()")
        .next()
        .expect("source should have endpoint prefix");
    let preceding_attribute = prefix
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with("#["))
        .expect("memory ledger endpoint should have an attribute");

    assert!(
        preceding_attribute.contains("$crate::cdk::query"),
        "memory ledger diagnostic must use a raw query attribute in {}",
        macro_path.display()
    );
    assert!(
        !preceding_attribute.contains("canic_query"),
        "memory ledger diagnostic must not use normal Canic query dispatch in {}",
        macro_path.display()
    );
    assert!(
        endpoint.contains("$crate::cdk::api::is_controller")
            && endpoint.contains("MemoryQuery::ledger()"),
        "memory ledger diagnostic must be controller-gated and read the restricted ledger path"
    );
}

#[test]
fn memory_ledger_is_config_gated() {
    let bundle_path = workspace_root().join("crates/canic/src/macros/endpoints/bundles.rs");
    let bundles = read_text(&bundle_path);
    let shared_bundle = bundles
        .split("macro_rules! canic_bundle_shared_runtime_endpoints")
        .nth(1)
        .and_then(|rest| {
            rest.split("macro_rules! canic_bundle_root_only_endpoints")
                .next()
        })
        .expect("shared runtime bundle should exist");
    let wasm_store_bundle = bundles
        .split("macro_rules! canic_bundle_wasm_store_runtime_endpoints")
        .nth(1)
        .expect("wasm_store runtime bundle should exist");

    assert!(
        shared_bundle.contains("#[cfg(canic_memory_ledger_enabled)]")
            && shared_bundle.contains("canic_emit_memory_ledger_diagnostic_endpoint!"),
        "shared runtime bundle must config-gate the ABI ledger recovery endpoint"
    );
    assert!(
        wasm_store_bundle.contains("#[cfg(canic_memory_ledger_enabled)]")
            && wasm_store_bundle.contains("canic_emit_memory_ledger_diagnostic_endpoint!"),
        "wasm_store runtime bundle must config-gate the ABI ledger recovery endpoint"
    );
    assert!(
        !shared_bundle.contains("canic_emit_memory_observability_endpoints!"),
        "live memory registry diagnostics must not be in the default bundle"
    );
}

#[test]
fn missing_finish_marker_stays_actionable() {
    let macro_path = workspace_root().join("crates/canic/src/macros/start.rs");
    let source = read_text(&macro_path);
    let marker = "__canic_missing_finish_macro_add_canic_finish_at_end_after_all_endpoints";

    assert!(
        source.contains(&format!("const _: fn() = {marker};")),
        "lifecycle start macros must reference an actionable missing-finish marker"
    );
    assert!(
        source.contains(&format!("fn {marker}()")),
        "finish! must define the same missing-finish marker"
    );
    assert!(
        marker.contains("missing_finish_macro")
            && marker.contains("add_canic_finish")
            && marker.contains("after_all_endpoints"),
        "missing-finish marker should read like a compiler-error hint"
    );
}
