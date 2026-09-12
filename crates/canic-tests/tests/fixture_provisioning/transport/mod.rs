//! Real inter-canister delivery, stale callback and durable receipt qualification.

use super::{
    CommitFault, ImportBinding, ImportError, ImportSnapshot, binding, configure_runtime,
    drive_icydb_startup, first_row, progress, restart, row_bytes, validate,
    wait_for_canic_install_callback,
};
use std::time::Duration;

use candid::{CandidType, Principal, decode_one, encode_one};
use canic::Error;
use canic_testing_internal::pic::{
    CanicIcydbLifecycleFixture, install_canic_icydb_lifecycle_fixture,
};
use ic_testkit::pic::{CandidCallExt, CanisterInstallExt, ErrorCode};
use sha2::{Digest, Sha256};

#[derive(CandidType)]
enum ReplyMode {
    Held,
    Immediate,
    Reject,
}

#[test]
fn transport_rechecks_import_authority_and_requires_durable_validation_receipt() {
    let fixture = install_canic_icydb_lifecycle_fixture();
    let (target, installation) = activate(&fixture);
    let (source, _) = activate(&fixture);
    let bytes = row_bytes(0);
    let original = binding(target, &[Sha256::digest(&bytes).into()], installation);
    begin(&fixture, &original, source, &bytes).unwrap();
    assert!(matches!(
        ready(&fixture, &original),
        Err(ImportError::NotReady)
    ));

    prove_source_rejections(&fixture, &original, source, &bytes);
    let replacement = prove_late_reply_is_fenced(&fixture, &original, source, &bytes);
    let replacement_bytes = [0_u64.to_le_bytes(), 77_u64.to_le_bytes()].concat();
    prove_lost_reply_and_receipt(&fixture, &replacement, source, &replacement_bytes);
    prove_reinstall_during_fetch(&fixture, source, &bytes);
}

fn prove_reinstall_during_fetch(
    fixture: &CanicIcydbLifecycleFixture,
    source: Principal,
    bytes: &[u8],
) {
    let (target, installation) = activate(fixture);
    let digests = [Sha256::digest(bytes).into()];
    let original = binding(target, &digests, installation);
    begin(fixture, &original, source, bytes).unwrap();
    fixture
        .pic
        .wait_out_install_code_rate_limit(Duration::from_mins(5));
    prepare(fixture, source, &original, bytes, ReplyMode::Held);
    let message = fixture
        .pic
        .submit_call(
            target,
            Principal::anonymous(),
            "fixture_pull",
            encode_one(&original).unwrap(),
        )
        .unwrap();
    for _ in 0..16 {
        fixture.pic.tick();
        if reads(fixture, source) == 1 {
            break;
        }
    }
    assert_eq!(reads(fixture, source), 1);
    assert!(fixture.pic.ingress_status(message.clone()).is_none());
    let directory = fixture.reinstall_composed_canister(target, [98; 32]);
    let current = binding(target, &digests, directory.operation_id);
    configure_runtime(&fixture.pic, target, fixture.root, directory);
    wait_for_canic_install_callback(&fixture.pic, target);
    drive_icydb_startup(&fixture.pic, target);
    begin(fixture, &current, source, bytes).unwrap();
    release(fixture, source);
    // The IC discards the old installation's suspended call context. The
    // replacement must retain neither its row nor its import acknowledgement.
    assert!(fixture.pic.await_call(message).is_err());
    assert_eq!(progress(fixture, target).next, 0);
    assert!(first_row(fixture, target).is_empty());
    assert!(matches!(
        ready(fixture, &current),
        Err(ImportError::NotReady)
    ));
    assert!(matches!(
        pull(fixture, &original),
        Err(ImportError::Binding)
    ));
    prepare(fixture, source, &current, bytes, ReplyMode::Immediate);
    assert_eq!(pull(fixture, &current).unwrap().next, 1);
}

fn activate(fixture: &CanicIcydbLifecycleFixture) -> (Principal, [u8; 32]) {
    let (target, directory) = fixture.install_composed_canister();
    let installation = directory.operation_id;
    configure_runtime(&fixture.pic, target, fixture.root, directory);
    wait_for_canic_install_callback(&fixture.pic, target);
    drive_icydb_startup(&fixture.pic, target);
    (target, installation)
}

fn prove_source_rejections(
    fixture: &CanicIcydbLifecycleFixture,
    binding: &ImportBinding,
    source: Principal,
    bytes: &[u8],
) {
    assert!(matches!(
        begin(fixture, binding, Principal::anonymous(), bytes),
        Err(ImportError::Conflict)
    ));
    prepare(fixture, source, binding, bytes, ReplyMode::Immediate);
    let denied: Result<Result<Vec<u8>, ImportError>, Error> = fixture
        .pic
        .update_candid(source, "fixture_source_read", (binding.clone(),))
        .unwrap();
    assert!(matches!(denied.unwrap(), Err(ImportError::Binding)));
    assert_eq!(reads(fixture, source), 0);

    let mut wrong_installation = binding.clone();
    wrong_installation.installation = [91; 32];
    prepare(
        fixture,
        source,
        &wrong_installation,
        bytes,
        ReplyMode::Immediate,
    );
    assert!(matches!(pull(fixture, binding), Err(ImportError::Binding)));
    assert_eq!(reads(fixture, source), 0);
    prepare(fixture, source, binding, bytes, ReplyMode::Reject);
    assert!(matches!(pull(fixture, binding), Err(ImportError::Source)));
    prepare(
        fixture,
        source,
        binding,
        &row_bytes(88),
        ReplyMode::Immediate,
    );
    assert!(matches!(pull(fixture, binding), Err(ImportError::Conflict)));
    assert_eq!(progress(fixture, binding.target).next, 0);
    assert!(first_row(fixture, binding.target).is_empty());
    assert!(matches!(
        ready(fixture, binding),
        Err(ImportError::NotReady)
    ));
}

fn prove_late_reply_is_fenced(
    fixture: &CanicIcydbLifecycleFixture,
    original: &ImportBinding,
    source: Principal,
    bytes: &[u8],
) -> ImportBinding {
    prepare(fixture, source, original, bytes, ReplyMode::Held);
    let message = fixture
        .pic
        .submit_call(
            original.target,
            Principal::anonymous(),
            "fixture_pull",
            encode_one(original).unwrap(),
        )
        .unwrap();
    for _ in 0..16 {
        fixture.pic.tick();
        if reads(fixture, source) == 1 {
            break;
        }
    }
    assert_eq!(reads(fixture, source), 1);
    assert!(fixture.pic.ingress_status(message.clone()).is_none());
    assert!(matches!(pull(fixture, original), Err(ImportError::Busy)));
    assert_eq!(reads(fixture, source), 1);
    let invalidated: Result<Result<(), ImportError>, Error> = fixture
        .pic
        .update_candid(
            original.target,
            "fixture_invalidate_empty_import",
            (original.clone(),),
        )
        .unwrap();
    invalidated.unwrap().unwrap();

    let bytes = [0_u64.to_le_bytes(), 77_u64.to_le_bytes()].concat();
    let replacement = binding(
        original.target,
        &[Sha256::digest(&bytes).into()],
        original.installation,
    );
    begin(fixture, &replacement, source, &bytes).unwrap();
    release(fixture, source);
    let reply = fixture.pic.await_call(message).unwrap();
    let result: Result<Result<ImportSnapshot, ImportError>, Error> = decode_one(&reply).unwrap();
    assert!(matches!(result.unwrap(), Err(ImportError::Binding)));
    let observed = progress(fixture, replacement.target);
    assert_eq!(observed.binding, replacement);
    assert_eq!(observed.next, 0);
    assert_eq!(observed.receipt, None);
    assert!(first_row(fixture, replacement.target).is_empty());
    replacement
}

fn prove_lost_reply_and_receipt(
    fixture: &CanicIcydbLifecycleFixture,
    binding: &ImportBinding,
    source: Principal,
    bytes: &[u8],
) {
    for fault in [CommitFault::TrapAfterRows, CommitFault::TrapAfterCheckpoint] {
        prepare(fixture, source, binding, bytes, ReplyMode::Immediate);
        let result = fixture
            .pic
            .update_candid::<Result<Result<ImportSnapshot, ImportError>, Error>, _>(
                binding.target,
                "fixture_pull_with_fault",
                (binding.clone(), fault),
            );
        assert_eq!(
            result.unwrap_err().reject_response.unwrap().error_code,
            ErrorCode::CanisterCalledTrap
        );
        assert_eq!(progress(fixture, binding.target).next, 0);
        assert!(first_row(fixture, binding.target).is_empty());
    }
    // The callback's trap cleanup must also release the in-flight lease so the
    // next ordinary pull can retry the same application-owned cursor.
    prepare(fixture, source, binding, bytes, ReplyMode::Immediate);
    let message = fixture
        .pic
        .submit_call(
            binding.target,
            Principal::anonymous(),
            "fixture_pull",
            encode_one(binding).unwrap(),
        )
        .unwrap();
    // Drain but deliberately discard the successful caller response. Recovery
    // must use stable progress, not the lost acknowledgement.
    fixture.pic.await_call(message).unwrap();
    restart(fixture, binding.target);
    assert_eq!(progress(fixture, binding.target).next, 1);
    assert_eq!(first_row(fixture, binding.target), [(0, 77)]);
    assert_eq!(pull(fixture, binding).unwrap().next, 1);
    assert_eq!(reads(fixture, source), 1);
    assert!(matches!(
        ready(fixture, binding),
        Err(ImportError::NotReady)
    ));
    assert!(validate(fixture, binding).unwrap().receipt.is_none());
    assert!(matches!(
        ready(fixture, binding),
        Err(ImportError::NotReady)
    ));
    // The final response is likewise not the authority: restart before observing it.
    validate(fixture, binding).unwrap();
    restart(fixture, binding.target);
    assert_eq!(ready(fixture, binding).unwrap(), *binding);
    assert_eq!(
        pull(fixture, binding).unwrap().receipt,
        Some(binding.clone())
    );
    assert_eq!(reads(fixture, source), 1);
}

fn begin(
    fixture: &CanicIcydbLifecycleFixture,
    binding: &ImportBinding,
    source: Principal,
    bytes: &[u8],
) -> Result<ImportSnapshot, ImportError> {
    let digests: Vec<[u8; 32]> = vec![Sha256::digest(bytes).into()];
    fixture
        .pic
        .update_candid::<Result<Result<ImportSnapshot, ImportError>, Error>, _>(
            binding.target,
            "fixture_begin",
            (binding.clone(), digests, Some(source)),
        )
        .unwrap()
        .unwrap()
}

fn prepare(
    fixture: &CanicIcydbLifecycleFixture,
    source: Principal,
    binding: &ImportBinding,
    bytes: &[u8],
    mode: ReplyMode,
) {
    fixture
        .pic
        .update_candid::<Result<Result<(), ImportError>, Error>, _>(
            source,
            "fixture_source_prepare",
            (binding.clone(), bytes.to_vec(), mode),
        )
        .unwrap()
        .unwrap()
        .unwrap();
}

fn release(fixture: &CanicIcydbLifecycleFixture, source: Principal) {
    fixture
        .pic
        .update_candid::<Result<(), Error>, _>(source, "fixture_source_release", ())
        .unwrap()
        .unwrap();
}

fn reads(fixture: &CanicIcydbLifecycleFixture, source: Principal) -> u64 {
    fixture
        .pic
        .query_candid::<Result<u64, Error>, _>(source, "fixture_source_reads", ())
        .unwrap()
        .unwrap()
}

fn pull(
    fixture: &CanicIcydbLifecycleFixture,
    binding: &ImportBinding,
) -> Result<ImportSnapshot, ImportError> {
    fixture
        .pic
        .update_candid::<Result<Result<ImportSnapshot, ImportError>, Error>, _>(
            binding.target,
            "fixture_pull",
            (binding.clone(),),
        )
        .unwrap()
        .unwrap()
}

fn ready(
    fixture: &CanicIcydbLifecycleFixture,
    binding: &ImportBinding,
) -> Result<ImportBinding, ImportError> {
    fixture
        .pic
        .query_candid::<Result<Result<ImportBinding, ImportError>, Error>, _>(
            binding.target,
            "fixture_application_ready",
            (binding.clone(),),
        )
        .unwrap()
        .unwrap()
}
