//! PocketIC proof of an application-owned row/checkpoint commit using published IcyDB.

mod store;
mod transport;

use super::{configure_runtime, drive_icydb_startup, upgrade, wait_for_canic_install_callback};
use std::time::Duration;

use candid::{CandidType, Deserialize, Principal};
use canic::Error;
use canic_testing_internal::pic::{
    CanicIcydbLifecycleFixture, install_canic_icydb_lifecycle_fixture,
};
use ic_testkit::pic::{CandidCallExt, CanisterInstallExt, ErrorCode};
use sha2::{Digest, Sha256};

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct ImportBinding {
    target: Principal,
    installation: [u8; 32],
    content: [u8; 32],
}

#[derive(CandidType, Clone, Copy, Debug)]
enum CommitFault {
    None,
    ErrorBeforeRows,
    TrapBeforeRows,
    TrapAfterRows,
    TrapAfterCheckpoint,
    ReturnErrorAfterRows,
}

#[derive(CandidType, Debug, Deserialize)]
struct ImportSnapshot {
    binding: ImportBinding,
    next: u64,
    validated: u64,
    receipt: Option<ImportBinding>,
    instructions: u64,
}

#[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
enum ImportError {
    Binding,
    Bounds,
    Busy,
    Conflict,
    Database,
    Injected,
    NotBegun,
    NotReady,
    Sequence,
    Source,
    Validation,
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one managed installation retains the exact commit, restart, validation and receipt replay journey"
)]
fn published_icydb_rows_and_checkpoint_share_a_synchronous_message_commit() {
    let fixture = install_canic_icydb_lifecycle_fixture();
    prove_returned_error_is_not_rollback(&fixture);
    let (target, directory) = fixture.install_composed_canister();
    let chunks: Vec<Vec<u8>> = (0..8).map(row_bytes).collect();
    let digests: Vec<[u8; 32]> = chunks
        .iter()
        .map(|bytes| Sha256::digest(bytes).into())
        .collect();
    let binding = binding(target, &digests, directory.operation_id);
    let prepared = fixture
        .pic
        .update_candid::<Result<Result<ImportSnapshot, ImportError>, Error>, _>(
            target,
            "fixture_begin",
            (binding.clone(), digests.clone(), None::<Principal>),
        );
    assert_eq!(
        prepared.unwrap_err().reject_response.unwrap().error_code,
        ErrorCode::CanisterCalledTrap
    );
    configure_runtime(&fixture.pic, target, fixture.root, directory);
    wait_for_canic_install_callback(&fixture.pic, target);
    drive_icydb_startup(&fixture.pic, target);
    begin(&fixture, &binding, &digests);

    let outsider = Principal::from_slice(&[91; 29]);
    let denied: Result<Result<ImportSnapshot, ImportError>, Error> = fixture
        .pic
        .update_candid_as(
            target,
            outsider,
            "fixture_commit",
            (binding.clone(), 0_u64, chunks[0].clone(), CommitFault::None),
        )
        .unwrap();
    assert!(denied.is_err());
    assert_eq!(progress(&fixture, target).next, 0);
    let rejected = commit(
        &fixture,
        &binding,
        0,
        chunks[0].clone(),
        CommitFault::ErrorBeforeRows,
    );
    assert!(matches!(rejected, Err(ImportError::Injected)));
    assert!(first_row(&fixture, target).is_empty());

    for fault in [
        CommitFault::TrapBeforeRows,
        CommitFault::TrapAfterRows,
        CommitFault::TrapAfterCheckpoint,
    ] {
        let trapped = fixture
            .pic
            .update_candid::<Result<Result<ImportSnapshot, ImportError>, Error>, _>(
                target,
                "fixture_commit",
                (binding.clone(), 0_u64, chunks[0].clone(), fault),
            );
        assert_eq!(
            trapped.unwrap_err().reject_response.unwrap().error_code,
            ErrorCode::CanisterCalledTrap
        );
        assert_eq!(progress(&fixture, target).next, 0);
        assert!(first_row(&fixture, target).is_empty());
    }
    assert!(matches!(
        commit(&fixture, &binding, 1, chunks[1].clone(), CommitFault::None),
        Err(ImportError::Sequence)
    ));
    assert!(matches!(
        commit(&fixture, &binding, 0, vec![0; 17], CommitFault::None),
        Err(ImportError::Bounds)
    ));
    let mut stale = binding.clone();
    stale.installation = [99; 32];
    assert!(matches!(
        commit(&fixture, &stale, 0, chunks[0].clone(), CommitFault::None),
        Err(ImportError::Binding)
    ));
    let first = commit(&fixture, &binding, 0, chunks[0].clone(), CommitFault::None).unwrap();
    assert_eq!(first.next, 1);
    restart(&fixture, target);
    begin(&fixture, &binding, &digests);
    let replay = commit(&fixture, &binding, 0, chunks[0].clone(), CommitFault::None).unwrap();
    assert_eq!(replay.next, 1);
    assert_eq!(first_row(&fixture, target), [(0, 10)]);
    assert!(matches!(
        commit(&fixture, &binding, 0, row_bytes(44), CommitFault::None),
        Err(ImportError::Conflict)
    ));

    let mut maximum_commit = first.instructions;
    for (index, bytes) in chunks.iter().enumerate().skip(1) {
        let result = commit(
            &fixture,
            &binding,
            index as u64,
            bytes.clone(),
            CommitFault::None,
        )
        .unwrap();
        assert_eq!(result.next, index as u64 + 1);
        maximum_commit = maximum_commit.max(result.instructions);
    }
    let mut maximum_validation = 0;
    let mut previous = 0;
    for step in 0..=chunks.len() {
        let result = validate(&fixture, &binding).unwrap();
        assert!(result.validated >= previous && result.validated <= previous + 1);
        maximum_validation = maximum_validation.max(result.instructions);
        previous = result.validated;
        if step == 0 {
            restart(&fixture, target);
        }
        if result.receipt.is_some() {
            break;
        }
    }
    let complete = progress(&fixture, target);
    assert_eq!(complete.binding, binding);
    assert_eq!(complete.next, chunks.len() as u64);
    assert_eq!(complete.validated, complete.next);
    assert_eq!(complete.receipt, Some(binding.clone()));
    restart(&fixture, target);
    let replay = validate(&fixture, &binding).unwrap();
    assert_eq!(replay.receipt, complete.receipt);
    assert_eq!(replay.validated, complete.validated);
    assert!(
        replay.instructions < maximum_validation,
        "receipt replay must avoid scanning rows"
    );
    prove_reinstall_fences_old_binding(&fixture, &binding, &digests);
    println!(
        "CANIC-165 commit={maximum_commit} validation={maximum_validation} receipt_replay={} instructions; {} rows, 16-byte chunks",
        replay.instructions,
        chunks.len()
    );
}

fn prove_returned_error_is_not_rollback(fixture: &CanicIcydbLifecycleFixture) {
    let (target, directory) = fixture.install_composed_canister();
    let installation = directory.operation_id;
    configure_runtime(&fixture.pic, target, fixture.root, directory);
    wait_for_canic_install_callback(&fixture.pic, target);
    drive_icydb_startup(&fixture.pic, target);
    let bytes = row_bytes(0);
    let digests = [Sha256::digest(&bytes).into()];
    let binding = binding(target, &digests, installation);
    begin(fixture, &binding, &digests);
    assert!(matches!(
        commit(
            fixture,
            &binding,
            0,
            bytes,
            CommitFault::ReturnErrorAfterRows
        ),
        Err(ImportError::Injected)
    ));
    assert_eq!(progress(fixture, target).next, 0);
    assert_eq!(first_row(fixture, target), [(0, 10)]);
    restart(fixture, target);
    assert_eq!(progress(fixture, target).next, 0);
    assert_eq!(first_row(fixture, target), [(0, 10)]);
}

fn binding(target: Principal, digests: &[[u8; 32]], installation: [u8; 32]) -> ImportBinding {
    ImportBinding {
        target,
        installation,
        content: Sha256::digest(digests.iter().flatten().copied().collect::<Vec<_>>()).into(),
    }
}

fn row_bytes(id: u64) -> Vec<u8> {
    [id.to_le_bytes(), (id + 10).to_le_bytes()].concat()
}

fn begin(fixture: &CanicIcydbLifecycleFixture, binding: &ImportBinding, digests: &[[u8; 32]]) {
    let result: Result<Result<ImportSnapshot, ImportError>, Error> = fixture
        .pic
        .update_candid(
            binding.target,
            "fixture_begin",
            (binding.clone(), digests.to_vec(), None::<Principal>),
        )
        .unwrap();
    result.unwrap().unwrap();
}

fn commit(
    fixture: &CanicIcydbLifecycleFixture,
    binding: &ImportBinding,
    index: u64,
    bytes: Vec<u8>,
    fault: CommitFault,
) -> Result<ImportSnapshot, ImportError> {
    fixture
        .pic
        .update_candid::<Result<Result<ImportSnapshot, ImportError>, Error>, _>(
            binding.target,
            "fixture_commit",
            (binding.clone(), index, bytes, fault),
        )
        .unwrap()
        .unwrap()
}

fn validate(
    fixture: &CanicIcydbLifecycleFixture,
    binding: &ImportBinding,
) -> Result<ImportSnapshot, ImportError> {
    fixture
        .pic
        .update_candid::<Result<Result<ImportSnapshot, ImportError>, Error>, _>(
            binding.target,
            "fixture_validate",
            (binding.clone(),),
        )
        .unwrap()
        .unwrap()
}

fn progress(fixture: &CanicIcydbLifecycleFixture, target: Principal) -> ImportSnapshot {
    fixture
        .pic
        .query_candid::<Result<Result<ImportSnapshot, ImportError>, Error>, _>(
            target,
            "fixture_progress",
            (),
        )
        .unwrap()
        .unwrap()
        .unwrap()
}

fn first_row(fixture: &CanicIcydbLifecycleFixture, target: Principal) -> Vec<(u64, u64)> {
    fixture
        .pic
        .query_candid::<Result<Result<Vec<(u64, u64)>, ImportError>, Error>, _>(
            target,
            "fixture_first_row",
            (),
        )
        .unwrap()
        .unwrap()
        .unwrap()
}

fn restart(fixture: &CanicIcydbLifecycleFixture, target: Principal) {
    fixture
        .pic
        .wait_out_install_code_rate_limit(Duration::from_mins(5));
    upgrade(&fixture.pic, target, &fixture.wasm);
    drive_icydb_startup(&fixture.pic, target);
}

fn prove_reinstall_fences_old_binding(
    fixture: &CanicIcydbLifecycleFixture,
    old: &ImportBinding,
    digests: &[[u8; 32]],
) {
    fixture
        .pic
        .wait_out_install_code_rate_limit(Duration::from_mins(5));
    let directory = fixture.reinstall_composed_canister(old.target, [98; 32]);
    let current = binding(old.target, digests, directory.operation_id);
    configure_runtime(&fixture.pic, old.target, fixture.root, directory);
    wait_for_canic_install_callback(&fixture.pic, old.target);
    drive_icydb_startup(&fixture.pic, old.target);
    assert!(first_row(fixture, old.target).is_empty());
    begin(fixture, &current, digests);
    assert!(matches!(
        commit(fixture, old, 0, row_bytes(0), CommitFault::None),
        Err(ImportError::Binding)
    ));
    assert!(matches!(validate(fixture, old), Err(ImportError::Binding)));
    let observed = progress(fixture, old.target);
    assert_eq!(observed.next, 0);
    assert_eq!(observed.receipt, None);
    assert_eq!(
        commit(fixture, &current, 0, row_bytes(0), CommitFault::None)
            .unwrap()
            .next,
        1
    );
}
