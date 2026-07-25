use super::*;
use crate::{
    deployment_truth::{DEPLOYMENT_TRUTH_SCHEMA_VERSION, PhaseReceiptV1, VerifiedPostconditionV1},
    release_build::{
        ReleaseBuildPlanRecord, finalize_release_build_from_manifest, plan_release_build,
    },
    test_support::temp_dir,
};
use canic_core::ids::ReleaseBuildNonce;
use std::{
    fs,
    sync::{Arc, Barrier},
};

fn finalized_release(root: &Path, contents: &[u8]) -> FinalizedReleaseBuild {
    let plan = plan_release_build(root).expect("plan release build");
    let manifest = root.join("release-set.json");
    fs::write(&manifest, contents).expect("write release-set manifest");
    finalize_release_build_from_manifest(root, plan.record.release_build_id, &manifest)
        .expect("finalize release build")
}

fn request<'a>(
    root: &'a Path,
    finalized_release_build: &'a FinalizedReleaseBuild,
) -> PlanFleetInstallActivationRequest<'a> {
    PlanFleetInstallActivationRequest {
        root,
        canonical_network_id: CanonicalNetworkId::public_ic(),
        fleet_name: "toko-local".parse().expect("Fleet name"),
        app: AppId::from("toko"),
        finalized_release_build,
    }
}

fn write_root_install_receipt(
    root: &Path,
    module_hash: [u8; 32],
    activation_identity: &FleetActivationIdentity,
) -> (PathBuf, DeploymentReceiptV1) {
    fs::create_dir_all(root).expect("create receipt root");
    let root_canister = Principal::from_slice(&[42; 29]);
    let hash = hex_digest(module_hash);
    let receipt = DeploymentReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        operation_id: "local:local:toko-local:check:install_root".to_string(),
        plan_id: "plan".to_string(),
        execution_context: None,
        operation_status: DeploymentExecutionStatusV1::Complete,
        started_at: "unix:1".to_string(),
        finished_at: Some("unix:2".to_string()),
        operator_principal: None,
        root_principal: Some(root_canister.to_text()),
        previous_observed_deployment_epoch: None,
        phase_receipts: vec![PhaseReceiptV1 {
            phase: "install_root".to_string(),
            started_at: "unix:1".to_string(),
            finished_at: Some("unix:2".to_string()),
            attempted_action: "install root wasm".to_string(),
            verified_postcondition: VerifiedPostconditionV1 {
                status: ObservationStatusV1::Observed,
                evidence: vec![
                    format!("root_canister:{root_canister}"),
                    "root_wasm:/tmp/root.wasm".to_string(),
                    format!("expected_module_hash:{hash}"),
                    format!("observed_module_hash:{hash}"),
                    format!(
                        "canonical_network_id:{}",
                        activation_identity.fleet.fleet.network
                    ),
                    format!("app:{}", activation_identity.fleet.app),
                    format!("fleet_id:{}", activation_identity.fleet.fleet.fleet_id),
                    format!(
                        "activation_operation_id:{}",
                        hex_digest(activation_identity.operation_id)
                    ),
                    format!("release_build_id:{}", activation_identity.release_build_id),
                    "fleet_activation_phase:prepared".to_string(),
                ],
            },
        }],
        role_phase_receipts: Vec::new(),
        final_inventory_id: Some("inventory".to_string()),
        command_result: DeploymentCommandResultV1::Succeeded,
    };
    let path = root.join(format!("root-install-{}.json", module_hash[0]));
    let mut bytes = serde_json::to_vec_pretty(&receipt).expect("encode root-install receipt");
    bytes.push(b'\n');
    fs::write(&path, bytes).expect("write root-install receipt");
    (path, receipt)
}

fn sample_activation_identity() -> FleetActivationIdentity {
    FleetActivationIdentity {
        fleet: FleetBinding {
            fleet: FleetKey {
                network: CanonicalNetworkId::public_ic(),
                fleet_id: FleetId::from_generated_bytes([3; 32]),
            },
            app: AppId::from("toko"),
        },
        operation_id: [4; 32],
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([5; 32])),
    }
}

fn prepared_root_status(
    identity: &FleetActivationIdentity,
    root_policy_seed: u8,
) -> FleetActivationStatusResponse {
    let manifest = [1_u8, 2]
        .into_iter()
        .map(|seed| FleetCascadeManifestEntry {
            principal: Principal::from_slice(&[seed; 29]),
            state_snapshot_hash: [seed + 10; 32],
            topology_snapshot_hash: [seed + 20; 32],
        })
        .collect::<Vec<_>>();
    let cascade_manifest_hash =
        FleetActivationApi::cascade_manifest_hash(&manifest).expect("hash cascade manifest");
    let credential_manifest = FleetCredentialManifest {
        fleet: identity.fleet.fleet,
        activation_id: identity.operation_id,
        generation: 1,
        root_policy_set_hash: [root_policy_seed; 32],
        renewal_template_set_hash: [31; 32],
        entries: Vec::new(),
    };
    let credential = FleetCredentialGenerationRef {
        generation: credential_manifest.generation,
        manifest_hash: FleetActivationApi::credential_manifest_hash(&credential_manifest)
            .expect("hash credential manifest"),
    };
    FleetActivationStatusResponse {
        phase: FleetActivationPhase::Prepared,
        identity: identity.clone(),
        cascade: Some(FleetCascadeActivationEvidence::Source {
            cascade_manifest_hash,
        }),
        cascade_manifest: Some(manifest),
        credential: Some(credential),
        credential_manifest: Some(credential_manifest),
        activated_at_ns: None,
    }
}

fn active_root_status(prepared: &FleetActivationStatusResponse) -> FleetActivationStatusResponse {
    FleetActivationStatusResponse {
        phase: FleetActivationPhase::Active,
        identity: prepared.identity.clone(),
        cascade: prepared.cascade.clone(),
        cascade_manifest: prepared.cascade_manifest.clone(),
        credential: prepared.credential,
        credential_manifest: prepared.credential_manifest.clone(),
        activated_at_ns: Some(44),
    }
}

fn refresh_active_evidence(record: &mut FleetActivationHostRecord, root_canister: Principal) {
    let manifest = record
        .cascade_manifest
        .clone()
        .expect("active cascade manifest");
    let cascade_manifest_hash =
        FleetActivationApi::cascade_manifest_hash(&manifest).expect("hash active manifest");
    let credential = record.credential.expect("active credential");
    for canister in &mut record.canisters {
        let cascade = if canister.principal == root_canister {
            FleetCascadeActivationEvidence::Source {
                cascade_manifest_hash,
            }
        } else {
            let entry = manifest
                .iter()
                .find(|entry| entry.principal == canister.principal)
                .expect("active child manifest entry");
            FleetCascadeActivationEvidence::Applied {
                state_snapshot_hash: entry.state_snapshot_hash,
                topology_snapshot_hash: entry.topology_snapshot_hash,
            }
        };
        canister.activation_evidence_hash = Some(
            FleetActivationApi::activation_evidence_hash(&record.identity, &cascade, credential)
                .expect("hash active Canister evidence"),
        );
    }
}

#[test]
fn planned_journal_is_canonical_durable_and_bound_to_every_path_identity() {
    let root = temp_dir("fleet-install-activation-plan");
    let finalized = finalized_release(&root, b"{\"release\":\"exact\"}");
    let planned = plan_fleet_install_activation(request(&root, &finalized))
        .expect("plan Fleet install activation");
    let identity = &planned.journal.activation.identity;
    let expected_path = fleet_install_activation_journal_path(
        &root,
        identity.fleet.fleet.network,
        identity.fleet.fleet.fleet_id,
        identity.operation_id,
    );

    assert!(planned.created);
    assert_eq!(planned.path, expected_path);
    assert_eq!(planned.journal.sequence, 0);
    assert_eq!(planned.journal.phase, FleetInstallActivationPhase::Planned);
    assert_eq!(planned.journal.fleet_name.as_str(), "toko-local");
    assert_eq!(identity.fleet.app.as_str(), "toko");
    assert_eq!(identity.release_build_id, finalized.record.release_build_id);
    assert_eq!(planned.journal.release_build_plan_hash, finalized.plan_hash);
    assert_eq!(
        planned.journal.release_set_manifest_digest,
        match finalized.record.state {
            ReleaseBuildPlanState::Finalized {
                release_set_manifest_digest,
            } => release_set_manifest_digest,
            ReleaseBuildPlanState::Planned => unreachable!("fixture is finalized"),
        }
    );
    assert_eq!(
        planned.journal_hash,
        fleet_install_activation_journal_hash(&planned.journal)
    );
    assert_eq!(
        load_fleet_install_activation_journal(
            &root,
            identity.fleet.fleet.network,
            identity.fleet.fleet.fleet_id,
            identity.operation_id,
        )
        .expect("load journal"),
        planned.journal
    );
    assert_eq!(
        recover_activation_root_canister(&planned, &root).expect("Planned has no installed root"),
        None
    );
    let journal_bytes = fs::read(&planned.path).expect("read journal");
    assert_eq!(journal_bytes[0], 0x88);
    let Value::Array(journal_fields) =
        ciborium::de::from_reader::<Value, _>(journal_bytes.as_slice()).expect("decode journal")
    else {
        panic!("journal must be an array");
    };
    let Value::Array(activation_fields) = &journal_fields[6] else {
        panic!("activation must be an array");
    };
    let Value::Array(identity_fields) = &activation_fields[0] else {
        panic!("activation identity must be an array");
    };
    let Value::Array(binding_fields) = &identity_fields[0] else {
        panic!("Fleet binding must be an array");
    };
    assert_eq!(binding_fields[1], Value::Bytes(b"toko".to_vec()));

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn root_installed_transition_is_canonical_monotonic_and_idempotent() {
    let root = temp_dir("fleet-install-activation-root-installed");
    let finalized = finalized_release(&root, b"manifest");
    let planned =
        plan_fleet_install_activation(request(&root, &finalized)).expect("plan activation");
    let (receipt_path, _) =
        write_root_install_receipt(&root, [12; 32], &planned.journal.activation.identity);
    let receipt = admit_root_install_receipt(&receipt_path).expect("admit root-install receipt");
    let expected_receipt_hash: [u8; 32] =
        Sha256::digest(fs::read(&receipt_path).expect("read receipt")).into();

    assert_eq!(receipt.receipt_hash, expected_receipt_hash);
    assert_eq!(receipt.root_canister, Principal::from_slice(&[42; 29]));
    assert_eq!(receipt.module_hash, [12; 32]);
    assert_eq!(
        receipt.activation_identity,
        planned.journal.activation.identity
    );

    let installed = record_root_installed(&root, &planned, &receipt).expect("record RootInstalled");
    assert!(installed.advanced);
    assert_eq!(installed.journal.sequence, 1);
    assert_eq!(
        installed.journal.phase,
        FleetInstallActivationPhase::RootInstalled
    );
    assert_eq!(
        installed.journal.root_install_receipt_hash,
        Some(receipt.receipt_hash)
    );
    assert_eq!(
        installed.journal_hash,
        fleet_install_activation_journal_hash(&installed.journal)
    );
    assert_eq!(
        load_fleet_install_activation_journal(
            &root,
            installed.journal.activation.identity.fleet.fleet.network,
            installed.journal.activation.identity.fleet.fleet.fleet_id,
            installed.journal.activation.identity.operation_id,
        )
        .expect("load RootInstalled"),
        installed.journal
    );

    let resumed =
        plan_fleet_install_activation(request(&root, &finalized)).expect("rediscover activation");
    assert!(!resumed.created);
    assert_eq!(
        resumed.journal.phase,
        FleetInstallActivationPhase::RootInstalled
    );
    let receipt_directory = root.join("receipt-recovery");
    fs::create_dir_all(&receipt_directory).expect("create receipt recovery directory");
    fs::copy(&receipt_path, receipt_directory.join("install-root.json"))
        .expect("copy durable root-install receipt");
    assert_eq!(
        recover_activation_root_canister(&resumed, &receipt_directory)
            .expect("recover installed root"),
        Some(receipt.root_canister)
    );
    let repeated = record_root_installed(&root, &resumed, &receipt).expect("repeat RootInstalled");
    assert!(!repeated.advanced);
    assert_eq!(repeated.journal, installed.journal);
    assert_eq!(repeated.journal_hash, installed.journal_hash);

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn canisters_prepared_transition_is_canonical_monotonic_and_idempotent() {
    let root = temp_dir("fleet-install-activation-canisters-prepared");
    let finalized = finalized_release(&root, b"manifest");
    let planned =
        plan_fleet_install_activation(request(&root, &finalized)).expect("plan activation");
    let (receipt_path, _) =
        write_root_install_receipt(&root, [21; 32], &planned.journal.activation.identity);
    let receipt = admit_root_install_receipt(&receipt_path).expect("admit root-install receipt");
    let installed = record_root_installed(&root, &planned, &receipt).expect("record RootInstalled");
    let root_status = prepared_root_status(&installed.journal.activation.identity, 30);
    let evidence = admit_canisters_prepared(
        receipt.root_canister,
        &installed.journal.activation.identity,
        &root_status,
    )
    .expect("admit Prepared status set");

    let prepared =
        record_canisters_prepared(&root, &installed, &evidence).expect("record CanistersPrepared");
    assert!(prepared.advanced);
    assert_eq!(prepared.journal.sequence, 2);
    assert_eq!(
        prepared.journal.phase,
        FleetInstallActivationPhase::CanistersPrepared
    );
    assert_eq!(prepared.journal.activation, evidence.activation);
    assert_eq!(
        prepared.journal_hash,
        fleet_install_activation_journal_hash(&prepared.journal)
    );
    assert_eq!(
        load_fleet_install_activation_journal(
            &root,
            prepared.journal.activation.identity.fleet.fleet.network,
            prepared.journal.activation.identity.fleet.fleet.fleet_id,
            prepared.journal.activation.identity.operation_id,
        )
        .expect("load CanistersPrepared"),
        prepared.journal
    );

    let repeated =
        record_canisters_prepared(&root, &installed, &evidence).expect("repeat CanistersPrepared");
    assert!(!repeated.advanced);
    assert_eq!(repeated.journal, prepared.journal);
    let rediscovered =
        plan_fleet_install_activation(request(&root, &finalized)).expect("rediscover activation");
    let resumed =
        resume_canisters_prepared(&rediscovered).expect("resume CanistersPrepared authority");
    assert!(!resumed.advanced);
    assert_eq!(resumed.journal, prepared.journal);
    assert_eq!(
        recover_activation_root_canister(&rediscovered, &root)
            .expect("recover Prepared root from activation evidence"),
        Some(receipt.root_canister)
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn canisters_activated_transition_is_canonical_monotonic_and_idempotent() {
    let root = temp_dir("fleet-install-activation-canisters-activated");
    let finalized = finalized_release(&root, b"manifest");
    let planned =
        plan_fleet_install_activation(request(&root, &finalized)).expect("plan activation");
    let (receipt_path, _) =
        write_root_install_receipt(&root, [24; 32], &planned.journal.activation.identity);
    let receipt = admit_root_install_receipt(&receipt_path).expect("admit root-install receipt");
    let installed = record_root_installed(&root, &planned, &receipt).expect("record RootInstalled");
    let prepared_status = prepared_root_status(&installed.journal.activation.identity, 30);
    let prepared_evidence = admit_canisters_prepared(
        receipt.root_canister,
        &installed.journal.activation.identity,
        &prepared_status,
    )
    .expect("admit Prepared status set");
    let prepared = record_canisters_prepared(&root, &installed, &prepared_evidence)
        .expect("record CanistersPrepared");

    assert_eq!(
        canisters_prepared_resume_request(&prepared),
        FleetActivationResumeRequest {
            operation_id: prepared.journal.activation.identity.operation_id,
            credential: prepared
                .journal
                .activation
                .credential
                .expect("Prepared credential"),
        }
    );
    let active_status = active_root_status(&prepared_status);
    let active_evidence =
        admit_canisters_activated(receipt.root_canister, &prepared, &active_status)
            .expect("admit exact Active status");
    assert!(
        active_evidence
            .activation
            .canisters
            .iter()
            .all(|entry| entry.activation_evidence_hash.is_some())
    );

    let activated = record_canisters_activated(&root, &prepared, &active_evidence)
        .expect("record CanistersActivated");
    assert!(activated.advanced);
    assert_eq!(activated.journal.sequence, 3);
    assert_eq!(
        activated.journal.phase,
        FleetInstallActivationPhase::CanistersActivated
    );
    assert_eq!(activated.journal.activation, active_evidence.activation);
    assert_eq!(
        activated.journal_hash,
        fleet_install_activation_journal_hash(&activated.journal)
    );
    assert_eq!(
        load_fleet_install_activation_journal(
            &root,
            activated.journal.activation.identity.fleet.fleet.network,
            activated.journal.activation.identity.fleet.fleet.fleet_id,
            activated.journal.activation.identity.operation_id,
        )
        .expect("load CanistersActivated"),
        activated.journal
    );

    let repeated = record_canisters_activated(&root, &prepared, &active_evidence)
        .expect("repeat CanistersActivated");
    assert!(!repeated.advanced);
    assert_eq!(repeated.journal, activated.journal);
    let rediscovered =
        plan_fleet_install_activation(request(&root, &finalized)).expect("rediscover activation");
    let resumed =
        resume_canisters_activated(&rediscovered).expect("resume CanistersActivated authority");
    assert!(!resumed.advanced);
    assert_eq!(resumed.journal, activated.journal);
    assert_eq!(
        recover_activation_root_canister(&rediscovered, &root)
            .expect("recover Active root from activation evidence"),
        Some(receipt.root_canister)
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn canisters_activated_rejects_partial_or_conflicting_evidence_without_mutation() {
    let root = temp_dir("fleet-install-activation-active-conflict");
    let finalized = finalized_release(&root, b"manifest");
    let planned =
        plan_fleet_install_activation(request(&root, &finalized)).expect("plan activation");
    let (receipt_path, _) =
        write_root_install_receipt(&root, [25; 32], &planned.journal.activation.identity);
    let receipt = admit_root_install_receipt(&receipt_path).expect("admit root-install receipt");
    let installed = record_root_installed(&root, &planned, &receipt).expect("record RootInstalled");
    let prepared_status = prepared_root_status(&installed.journal.activation.identity, 30);
    let prepared_evidence = admit_canisters_prepared(
        receipt.root_canister,
        &installed.journal.activation.identity,
        &prepared_status,
    )
    .expect("admit Prepared status set");
    let prepared = record_canisters_prepared(&root, &installed, &prepared_evidence)
        .expect("record CanistersPrepared");
    let prepared_bytes = fs::read(&prepared.path).expect("read Prepared journal");

    let mut incomplete_status = active_root_status(&prepared_status);
    incomplete_status.activated_at_ns = None;
    std::assert_matches!(
        admit_canisters_activated(receipt.root_canister, &prepared, &incomplete_status),
        Err(FleetInstallActivationJournalError::InvalidActivatedActivationEvidence { .. })
    );
    assert_eq!(
        fs::read(&prepared.path).expect("read unchanged Prepared journal"),
        prepared_bytes
    );

    let active_status = active_root_status(&prepared_status);
    let mut active_evidence =
        admit_canisters_activated(receipt.root_canister, &prepared, &active_status)
            .expect("admit exact Active status");
    let child = active_evidence
        .activation
        .canisters
        .iter_mut()
        .find(|entry| entry.principal != receipt.root_canister)
        .expect("child evidence");
    child.activation_evidence_hash = Some([0xff; 32]);
    std::assert_matches!(
        record_canisters_activated(&root, &prepared, &active_evidence),
        Err(FleetInstallActivationJournalError::InvalidActivatedActivationEvidence { .. })
    );
    assert_eq!(
        fs::read(&prepared.path).expect("read unchanged Prepared journal"),
        prepared_bytes
    );

    let mut conflicting_evidence =
        admit_canisters_activated(receipt.root_canister, &prepared, &active_status)
            .expect("admit exact Active status");
    conflicting_evidence
        .activation
        .cascade_manifest
        .as_mut()
        .expect("active cascade manifest")[0]
        .state_snapshot_hash = [0xfe; 32];
    refresh_active_evidence(&mut conflicting_evidence.activation, receipt.root_canister);
    validate_activated_activation_record(&conflicting_evidence.activation)
        .expect("conflicting Active evidence is internally canonical");
    std::assert_matches!(
        record_canisters_activated(&root, &prepared, &conflicting_evidence),
        Err(FleetInstallActivationJournalError::ActivatedActivationEvidenceMismatch)
    );
    assert_eq!(
        fs::read(&prepared.path).expect("read unchanged Prepared journal"),
        prepared_bytes
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn canisters_prepared_rejects_partial_and_conflicting_evidence_without_mutation() {
    let root = temp_dir("fleet-install-activation-prepared-conflict");
    let finalized = finalized_release(&root, b"manifest");
    let planned =
        plan_fleet_install_activation(request(&root, &finalized)).expect("plan activation");
    let (receipt_path, _) =
        write_root_install_receipt(&root, [22; 32], &planned.journal.activation.identity);
    let receipt = admit_root_install_receipt(&receipt_path).expect("admit root-install receipt");
    let installed = record_root_installed(&root, &planned, &receipt).expect("record RootInstalled");
    let mut root_status = prepared_root_status(&installed.journal.activation.identity, 30);
    root_status.cascade = Some(FleetCascadeActivationEvidence::Source {
        cascade_manifest_hash: [0xff; 32],
    });

    std::assert_matches!(
        admit_canisters_prepared(
            receipt.root_canister,
            &installed.journal.activation.identity,
            &root_status,
        ),
        Err(FleetInstallActivationJournalError::InvalidPreparedActivationEvidence { .. })
    );
    assert_eq!(
        fs::read(&installed.path).expect("read unchanged RootInstalled journal"),
        encode_journal(&installed.journal).expect("encode RootInstalled journal")
    );

    let root_status = prepared_root_status(&installed.journal.activation.identity, 30);
    let evidence = admit_canisters_prepared(
        receipt.root_canister,
        &installed.journal.activation.identity,
        &root_status,
    )
    .expect("admit first Prepared status set");
    record_canisters_prepared(&root, &installed, &evidence)
        .expect("record first CanistersPrepared");
    let other_root_status = prepared_root_status(&installed.journal.activation.identity, 31);
    let other = admit_canisters_prepared(
        receipt.root_canister,
        &installed.journal.activation.identity,
        &other_root_status,
    )
    .expect("admit alternate Prepared status set");
    std::assert_matches!(
        record_canisters_prepared(&root, &installed, &other),
        Err(FleetInstallActivationJournalError::PreparedActivationEvidenceMismatch)
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn root_install_receipt_recovery_uses_the_exact_journalled_bytes() {
    let root = temp_dir("fleet-install-activation-recover-receipt");
    let identity = sample_activation_identity();
    let (receipt_path, _) = write_root_install_receipt(&root, [23; 32], &identity);
    let admitted = admit_root_install_receipt(&receipt_path).expect("admit receipt");
    fs::write(root.join("unrelated.json"), b"not the matching receipt")
        .expect("write unrelated receipt");

    assert_eq!(
        recover_root_install_receipt(&root, admitted.receipt_hash).expect("recover exact receipt"),
        admitted
    );
    std::assert_matches!(
        recover_root_install_receipt(&root, [0xff; 32]),
        Err(FleetInstallActivationJournalError::MissingRootInstallReceipt { .. })
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn root_installed_transition_rejects_stale_journal_and_receipt_conflicts() {
    let root = temp_dir("fleet-install-activation-root-conflict");
    let finalized = finalized_release(&root, b"manifest");
    let planned =
        plan_fleet_install_activation(request(&root, &finalized)).expect("plan activation");
    let (receipt_path, _) =
        write_root_install_receipt(&root, [13; 32], &planned.journal.activation.identity);
    let receipt = admit_root_install_receipt(&receipt_path).expect("admit receipt");
    let mut wrong_identity = planned.journal.activation.identity.clone();
    wrong_identity.operation_id = [0xdd; 32];
    let (wrong_identity_path, _) = write_root_install_receipt(&root, [18; 32], &wrong_identity);
    let wrong_identity_receipt =
        admit_root_install_receipt(&wrong_identity_path).expect("admit wrong-identity receipt");

    std::assert_matches!(
        record_root_installed(&root, &planned, &wrong_identity_receipt),
        Err(FleetInstallActivationJournalError::RootInstallReceiptIdentityMismatch)
    );
    assert_eq!(
        fs::read(&planned.path).expect("read unchanged Planned journal"),
        encode_journal(&planned.journal).expect("encode Planned journal")
    );

    let mut changed = planned.journal.clone();
    changed.release_set_manifest_digest = [0xee; 32];
    fs::write(
        &planned.path,
        encode_journal(&changed).expect("encode changed journal"),
    )
    .expect("write changed journal");
    std::assert_matches!(
        record_root_installed(&root, &planned, &receipt),
        Err(FleetInstallActivationJournalError::JournalChanged { .. })
    );
    assert_eq!(
        fs::read(&planned.path).expect("read unchanged conflicting journal"),
        encode_journal(&changed).expect("encode changed journal")
    );

    fs::write(
        &planned.path,
        encode_journal(&planned.journal).expect("encode original journal"),
    )
    .expect("restore planned journal");
    let installed = record_root_installed(&root, &planned, &receipt).expect("record RootInstalled");
    let resumed =
        plan_fleet_install_activation(request(&root, &finalized)).expect("resume RootInstalled");
    let (other_path, _) =
        write_root_install_receipt(&root, [14; 32], &planned.journal.activation.identity);
    let other = admit_root_install_receipt(&other_path).expect("admit other receipt");
    std::assert_matches!(
        record_root_installed(&root, &resumed, &other),
        Err(FleetInstallActivationJournalError::RootInstallReceiptMismatch)
    );
    assert_eq!(
        fs::read(&installed.path).expect("read RootInstalled journal"),
        encode_journal(&installed.journal).expect("encode RootInstalled journal")
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn root_install_receipt_admission_requires_canonical_verified_module_evidence() {
    let root = temp_dir("fleet-install-activation-root-receipt");
    let (path, receipt) =
        write_root_install_receipt(&root, [15; 32], &sample_activation_identity());

    let mut mismatch = receipt.clone();
    mismatch.phase_receipts[0].verified_postcondition.evidence[3] =
        format!("observed_module_hash:{}", hex_digest([16; 32]));
    let mut bytes = serde_json::to_vec_pretty(&mismatch).expect("encode mismatch");
    bytes.push(b'\n');
    fs::write(&path, bytes).expect("write mismatch");
    std::assert_matches!(
        admit_root_install_receipt(&path),
        Err(FleetInstallActivationJournalError::InvalidRootInstallReceipt { .. })
    );

    let mut principal_mismatch = receipt.clone();
    principal_mismatch.root_principal = Some(Principal::from_slice(&[43; 29]).to_text());
    let mut bytes =
        serde_json::to_vec_pretty(&principal_mismatch).expect("encode principal mismatch");
    bytes.push(b'\n');
    fs::write(&path, bytes).expect("write principal mismatch");
    std::assert_matches!(
        admit_root_install_receipt(&path),
        Err(FleetInstallActivationJournalError::InvalidRootInstallReceipt { .. })
    );

    let mut missing_principal = receipt.clone();
    missing_principal.root_principal = None;
    let mut bytes =
        serde_json::to_vec_pretty(&missing_principal).expect("encode missing principal");
    bytes.push(b'\n');
    fs::write(&path, bytes).expect("write missing principal");
    std::assert_matches!(
        admit_root_install_receipt(&path),
        Err(FleetInstallActivationJournalError::InvalidRootInstallReceipt { .. })
    );

    let mut bytes = serde_json::to_vec(&receipt).expect("encode noncanonical");
    bytes.push(b'\n');
    fs::write(&path, bytes).expect("write noncanonical");
    std::assert_matches!(
        admit_root_install_receipt(&path),
        Err(FleetInstallActivationJournalError::InvalidRootInstallReceipt { .. })
    );

    fs::remove_file(&path).expect("remove receipt");
    std::assert_matches!(
        admit_root_install_receipt(&path),
        Err(FleetInstallActivationJournalError::MissingRootInstallReceipt { .. })
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[cfg(unix)]
#[test]
fn root_install_receipt_symlinks_are_rejected() {
    use std::os::unix::fs::symlink;

    let root = temp_dir("fleet-install-activation-root-receipt-symlink");
    let (path, _) = write_root_install_receipt(&root, [17; 32], &sample_activation_identity());
    let real = root.join("real-root-install.json");
    fs::rename(&path, &real).expect("move receipt");
    symlink(&real, &path).expect("link receipt");

    std::assert_matches!(
        admit_root_install_receipt(&path),
        Err(FleetInstallActivationJournalError::UnsafeRootInstallReceipt { .. })
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn exact_repeat_resumes_the_single_existing_planned_authority() {
    let root = temp_dir("fleet-install-activation-resume");
    let finalized = finalized_release(&root, b"manifest");
    let first = plan_fleet_install_activation(request(&root, &finalized)).expect("first plan");
    let repeated =
        plan_fleet_install_activation(request(&root, &finalized)).expect("repeat exact plan");

    assert!(first.created);
    assert!(!repeated.created);
    assert_eq!(repeated.journal, first.journal);
    assert_eq!(repeated.journal_hash, first.journal_hash);
    assert_eq!(repeated.path, first.path);
    assert_eq!(
        fs::read_dir(
            first
                .path
                .parent()
                .and_then(Path::parent)
                .expect("Fleet directory")
        )
        .expect("read Fleet directory")
        .count(),
        1
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn planning_rejects_active_app_and_release_build_contradictions() {
    let root = temp_dir("fleet-install-activation-contradictions");
    let first_release = finalized_release(&root, b"first manifest");
    let first =
        plan_fleet_install_activation(request(&root, &first_release)).expect("first activation");

    std::assert_matches!(
        plan_fleet_install_activation(PlanFleetInstallActivationRequest {
            app: AppId::from("other"),
            ..request(&root, &first_release)
        }),
        Err(FleetInstallActivationJournalError::ActiveAppMismatch {
            path,
            ..
        }) if path == first.path
    );

    let second_release = finalized_release(&root, b"second manifest");
    std::assert_matches!(
        plan_fleet_install_activation(request(&root, &second_release)),
        Err(
            FleetInstallActivationJournalError::ActiveReleaseBuildMismatch {
                path,
                ..
            }
        ) if path == first.path
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn discovery_rejects_competing_name_and_fleet_id_authorities() {
    let root = temp_dir("fleet-install-activation-competing-name");
    let finalized = finalized_release(&root, b"manifest");
    let ReleaseBuildPlanState::Finalized {
        release_set_manifest_digest,
    } = finalized.record.state
    else {
        unreachable!("fixture is finalized");
    };
    let activation_request = request(&root, &finalized);
    plan_fleet_install_activation_with_ids(
        &activation_request,
        &finalized,
        release_set_manifest_digest,
        FleetId::from_generated_bytes([1; 32]),
        [2; 32],
    )
    .expect("first name authority");
    plan_fleet_install_activation_with_ids(
        &activation_request,
        &finalized,
        release_set_manifest_digest,
        FleetId::from_generated_bytes([3; 32]),
        [4; 32],
    )
    .expect("second name authority");

    std::assert_matches!(
        plan_fleet_install_activation(request(&root, &finalized)),
        Err(FleetInstallActivationJournalError::CompetingFleetNameAuthorities { .. })
    );
    fs::remove_dir_all(&root).expect("remove competing-name root");

    let root = temp_dir("fleet-install-activation-competing-id");
    let finalized = finalized_release(&root, b"manifest");
    let ReleaseBuildPlanState::Finalized {
        release_set_manifest_digest,
    } = finalized.record.state
    else {
        unreachable!("fixture is finalized");
    };
    let first_request = request(&root, &finalized);
    let fleet_id = FleetId::from_generated_bytes([5; 32]);
    plan_fleet_install_activation_with_ids(
        &first_request,
        &finalized,
        release_set_manifest_digest,
        fleet_id,
        [6; 32],
    )
    .expect("first ID authority");
    plan_fleet_install_activation_with_ids(
        &PlanFleetInstallActivationRequest {
            fleet_name: "other".parse().expect("Fleet name"),
            ..first_request
        },
        &finalized,
        release_set_manifest_digest,
        fleet_id,
        [7; 32],
    )
    .expect("second ID authority");

    std::assert_matches!(
        plan_fleet_install_activation(request(&root, &finalized)),
        Err(
            FleetInstallActivationJournalError::CompetingFleetIdAuthorities {
                fleet_id: conflicting,
                ..
            }
        ) if conflicting == fleet_id
    );

    fs::remove_dir_all(root).expect("remove competing-ID root");
}

#[test]
fn concurrent_exact_planning_creates_one_authority_and_resumes_it_once() {
    let root = Arc::new(temp_dir("fleet-install-activation-concurrent"));
    let finalized = Arc::new(finalized_release(&root, b"manifest"));
    let barrier = Arc::new(Barrier::new(2));
    let mut workers = Vec::new();

    for _ in 0..2 {
        let root = Arc::clone(&root);
        let finalized = Arc::clone(&finalized);
        let barrier = Arc::clone(&barrier);
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            plan_fleet_install_activation(request(&root, &finalized))
                .expect("concurrent activation plan")
        }));
    }
    let mut planned = workers
        .into_iter()
        .map(|worker| worker.join().expect("join planning worker"))
        .collect::<Vec<_>>();
    planned.sort_by_key(|entry| entry.created);

    assert!(!planned[0].created);
    assert!(planned[1].created);
    assert_eq!(planned[0].journal, planned[1].journal);
    assert_eq!(planned[0].path, planned[1].path);
    assert_eq!(planned[0].journal_hash, planned[1].journal_hash);

    fs::remove_dir_all(root.as_ref()).expect("remove temp root");
}

#[test]
fn unpublished_attempt_directories_are_inert_but_unsafe_entries_fail_closed() {
    let root = temp_dir("fleet-install-activation-inert-attempt");
    let finalized = finalized_release(&root, b"manifest");
    let network = CanonicalNetworkId::public_ic();
    let inert = fleet_install_activation_journal_path(
        &root,
        network,
        FleetId::from_generated_bytes([9; 32]),
        [10; 32],
    );
    fs::create_dir_all(inert.parent().expect("inert operation directory"))
        .expect("create inert attempt");
    let planned = plan_fleet_install_activation(request(&root, &finalized))
        .expect("ignore unpublished attempt");
    assert!(planned.created);

    let stray = fleet_install_activation_network_directory(&root, network).join("stray");
    fs::write(&stray, b"not a directory").expect("write unsafe entry");
    std::assert_matches!(
        plan_fleet_install_activation(request(&root, &finalized)),
        Err(FleetInstallActivationJournalError::UnsafeDirectoryEntry { .. })
    );
    fs::remove_file(&stray).expect("remove unsafe entry");
    fs::create_dir(&stray).expect("create invalid directory");
    std::assert_matches!(
        plan_fleet_install_activation(request(&root, &finalized)),
        Err(FleetInstallActivationJournalError::InvalidDirectory { .. })
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[cfg(unix)]
#[test]
fn discovery_rejects_symlinked_canonical_recovery_directories() {
    use std::os::unix::fs::symlink;

    let root = temp_dir("fleet-install-activation-directory-symlink");
    let finalized = finalized_release(&root, b"manifest");
    let network_directory =
        fleet_install_activation_network_directory(&root, CanonicalNetworkId::public_ic());
    fs::create_dir_all(&network_directory).expect("create network directory");
    let real = root.join("real-fleet-directory");
    fs::create_dir_all(&real).expect("create real Fleet directory");
    symlink(
        &real,
        network_directory.join(FleetId::from_generated_bytes([11; 32]).to_string()),
    )
    .expect("link Fleet directory");

    std::assert_matches!(
        plan_fleet_install_activation(request(&root, &finalized)),
        Err(FleetInstallActivationJournalError::UnsafeDirectoryEntry { .. })
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn planning_requires_unchanged_finalized_release_build_authority() {
    let root = temp_dir("fleet-install-activation-release-authority");
    let finalized = finalized_release(&root, b"manifest");
    let forged = FinalizedReleaseBuild {
        plan_hash: [0xff; 32],
        ..finalized
    };

    std::assert_matches!(
        plan_fleet_install_activation(request(&root, &forged)),
        Err(FleetInstallActivationJournalError::FinalizedReleaseBuildMismatch)
    );

    let other_root = temp_dir("fleet-install-activation-planned-release");
    let planned = plan_release_build(&other_root).expect("plan unfinalized release build");
    let unfinalized = FinalizedReleaseBuild {
        record: ReleaseBuildPlanRecord {
            state: ReleaseBuildPlanState::Planned,
            ..planned.record
        },
        plan_hash: [0; 32],
        path: planned.path,
    };
    std::assert_matches!(
        plan_fleet_install_activation(request(&other_root, &unfinalized)),
        Err(FleetInstallActivationJournalError::ReleaseBuild(
            ReleaseBuildPlanError::InvalidDocument { .. }
        ))
    );

    fs::remove_dir_all(root).expect("remove temp root");
    fs::remove_dir_all(other_root).expect("remove temp root");
}

#[test]
fn duplicate_identity_cannot_replace_the_existing_journal() {
    let root = temp_dir("fleet-install-activation-create-new");
    let finalized = finalized_release(&root, b"manifest");
    let ReleaseBuildPlanState::Finalized {
        release_set_manifest_digest,
    } = finalized.record.state
    else {
        unreachable!("fixture is finalized");
    };
    let network = CanonicalNetworkId::public_ic();
    let fleet_id = FleetId::from_generated_bytes([7; 32]);
    let operation_id = [8; 32];
    let request = PlanFleetInstallActivationRequest {
        root: &root,
        canonical_network_id: network,
        fleet_name: "toko-local".parse().expect("Fleet name"),
        app: AppId::from("toko"),
        finalized_release_build: &finalized,
    };
    let first = plan_fleet_install_activation_with_ids(
        &request,
        &finalized,
        release_set_manifest_digest,
        fleet_id,
        operation_id,
    )
    .expect("first plan");
    let original = fs::read(&first.path).expect("read first journal");

    std::assert_matches!(
        plan_fleet_install_activation_with_ids(
            &PlanFleetInstallActivationRequest {
                fleet_name: "other".parse().expect("Fleet name"),
                app: AppId::from("other"),
                ..request
            },
            &finalized,
            release_set_manifest_digest,
            fleet_id,
            operation_id,
        ),
        Err(FleetInstallActivationJournalError::Io { source, .. })
            if source.kind() == io::ErrorKind::AlreadyExists
    );
    assert_eq!(fs::read(&first.path).expect("reread journal"), original);

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn corrupt_noncanonical_and_path_mismatched_journals_fail_closed() {
    let root = temp_dir("fleet-install-activation-reject");
    let finalized = finalized_release(&root, b"manifest");
    let planned = plan_fleet_install_activation(request(&root, &finalized))
        .expect("plan Fleet install activation");
    let identity = &planned.journal.activation.identity;
    let canonical = fs::read(&planned.path).expect("read journal");

    fs::write(&planned.path, b"not-cbor").expect("corrupt journal");
    std::assert_matches!(
        load_fleet_install_activation_journal(
            &root,
            identity.fleet.fleet.network,
            identity.fleet.fleet.fleet_id,
            identity.operation_id,
        ),
        Err(FleetInstallActivationJournalError::InvalidDocument { .. })
    );

    fs::write(&planned.path, &canonical).expect("restore journal");
    let other_network = "11".repeat(32).parse().expect("canonical network");
    let other_fleet = FleetId::from_generated_bytes([0xee; 32]);
    let other_operation = [0xdd; 32];
    for (network, fleet, operation) in [
        (
            other_network,
            identity.fleet.fleet.fleet_id,
            identity.operation_id,
        ),
        (
            identity.fleet.fleet.network,
            other_fleet,
            identity.operation_id,
        ),
        (
            identity.fleet.fleet.network,
            identity.fleet.fleet.fleet_id,
            other_operation,
        ),
    ] {
        let other_path = fleet_install_activation_journal_path(&root, network, fleet, operation);
        fs::create_dir_all(other_path.parent().expect("journal parent"))
            .expect("create other parent");
        fs::copy(&planned.path, &other_path).expect("copy journal under wrong identity");
        std::assert_matches!(
            load_fleet_install_activation_journal(&root, network, fleet, operation),
            Err(FleetInstallActivationJournalError::InvalidDocument { .. })
        );
    }

    let mut noncanonical = canonical;
    noncanonical.splice(1..2, [0x98, 0x08]);
    fs::write(&planned.path, noncanonical).expect("write noncanonical journal");
    std::assert_matches!(
        load_fleet_install_activation_journal(
            &root,
            identity.fleet.fleet.network,
            identity.fleet.fleet.fleet_id,
            identity.operation_id,
        ),
        Err(FleetInstallActivationJournalError::InvalidDocument { .. })
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn phase_validation_enforces_exact_planned_and_root_installed_evidence() {
    let root = temp_dir("fleet-install-activation-phase");
    let finalized = finalized_release(&root, b"manifest");
    let planned = plan_fleet_install_activation(request(&root, &finalized))
        .expect("plan Fleet install activation");
    let mut invalid = planned.journal.clone();
    invalid.root_install_receipt_hash = Some([1; 32]);

    std::assert_matches!(
        encode_journal(&invalid),
        Err(FleetInstallActivationJournalError::InvalidDocument { .. })
    );

    let mut root_installed = planned.journal.clone();
    root_installed.phase = FleetInstallActivationPhase::RootInstalled;
    root_installed.sequence = 1;
    std::assert_matches!(
        encode_journal(&root_installed),
        Err(FleetInstallActivationJournalError::InvalidDocument { .. })
    );
    root_installed.root_install_receipt_hash = Some([1; 32]);
    assert!(encode_journal(&root_installed).is_ok());

    root_installed.sequence = 0;
    std::assert_matches!(
        encode_journal(&root_installed),
        Err(FleetInstallActivationJournalError::InvalidDocument { .. })
    );

    let mut reserved = planned.journal;
    reserved.phase = FleetInstallActivationPhase::CanistersPrepared;
    reserved.sequence = 2;
    reserved.root_install_receipt_hash = Some([1; 32]);
    std::assert_matches!(
        encode_journal(&reserved),
        Err(FleetInstallActivationJournalError::InvalidDocument { .. })
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[cfg(unix)]
#[test]
fn journal_symlinks_are_rejected() {
    use std::os::unix::fs::symlink;

    let root = temp_dir("fleet-install-activation-symlink");
    let finalized = finalized_release(&root, b"manifest");
    let planned = plan_fleet_install_activation(request(&root, &finalized))
        .expect("plan Fleet install activation");
    let identity = &planned.journal.activation.identity;
    let real = root.join("real-journal.cbor");
    fs::rename(&planned.path, &real).expect("move journal");
    symlink(&real, &planned.path).expect("link journal");

    std::assert_matches!(
        load_fleet_install_activation_journal(
            &root,
            identity.fleet.fleet.network,
            identity.fleet.fleet.fleet_id,
            identity.operation_id,
        ),
        Err(FleetInstallActivationJournalError::UnsafeFile { .. })
    );

    fs::remove_dir_all(root).expect("remove temp root");
}
