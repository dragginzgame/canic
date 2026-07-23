use super::*;
use crate::test_support::temp_dir;
use std::{fs, sync::Arc};

#[test]
fn planned_record_is_durable_canonical_and_bound_to_its_path() {
    let root = temp_dir("release-build-plan");
    let nonce = ReleaseBuildNonce::from_random_bytes([7; 32]);
    let plan = plan_release_build_with_nonce(&root, nonce).expect("plan release build");

    assert_eq!(
        plan.record.release_build_id,
        ReleaseBuildId::from_nonce(nonce)
    );
    assert_eq!(
        load_release_build_plan(&root, plan.record.release_build_id).expect("load plan"),
        plan.record
    );
    std::assert_matches!(
        load_finalized_release_build(&root, plan.record.release_build_id),
        Err(ReleaseBuildPlanError::InvalidDocument { .. })
    );
    let bytes = fs::read(&plan.path).expect("read plan");
    assert_eq!(bytes[0], 0x83);
    assert_eq!(bytes[1..3], [0x58, 0x20]);
    assert!(bytes.ends_with(&[0x81, 0x00]));

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn finalization_is_one_way_idempotent_and_hashes_exact_canonical_bytes() {
    let root = temp_dir("release-build-finalize");
    let nonce = ReleaseBuildNonce::from_random_bytes([11; 32]);
    let plan = plan_release_build_with_nonce(&root, nonce).expect("plan release build");
    let manifest_path = root.join("release-set.json");
    fs::write(&manifest_path, b"{\"exact\":\"manifest\"}").expect("write manifest");

    let finalized =
        finalize_release_build_from_manifest(&root, plan.record.release_build_id, &manifest_path)
            .expect("finalize");
    let repeated =
        finalize_release_build_from_manifest(&root, plan.record.release_build_id, &manifest_path)
            .expect("repeat exact finalization");

    assert_eq!(repeated, finalized);
    assert_eq!(
        load_finalized_release_build(&root, plan.record.release_build_id)
            .expect("load finalized plan"),
        finalized
    );
    assert_eq!(
        finalized.record.state,
        ReleaseBuildPlanState::Finalized {
            release_set_manifest_digest: Sha256::digest(b"{\"exact\":\"manifest\"}").into(),
        }
    );
    let expected_hash = domain_hash(PLAN_HASH_DOMAIN, &encode_record(finalized.record));
    assert_eq!(finalized.plan_hash, expected_hash);

    fs::write(&manifest_path, b"{\"different\":true}").expect("replace manifest");
    std::assert_matches!(
        finalize_release_build_from_manifest(&root, plan.record.release_build_id, &manifest_path,),
        Err(ReleaseBuildPlanError::ConflictingFinalization { .. })
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn competing_finalization_has_one_durable_winner() {
    let root = Arc::new(temp_dir("release-build-finalize-race"));
    let plan = plan_release_build_with_nonce(&root, ReleaseBuildNonce::from_random_bytes([12; 32]))
        .expect("plan release build");

    let workers = [[1; 32], [2; 32]].map(|digest| {
        let root = Arc::clone(&root);
        std::thread::spawn(move || {
            finalize_release_build(&root, plan.record.release_build_id, digest)
        })
    });
    let outcomes = workers.map(|worker| worker.join().expect("join finalizer"));

    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(
                outcome,
                Err(ReleaseBuildPlanError::ConflictingFinalization { .. })
            ))
            .count(),
        1
    );
    let durable = load_finalized_release_build(&root, plan.record.release_build_id)
        .expect("load winning finalization");
    assert!(outcomes.iter().flatten().any(|outcome| outcome == &durable));

    fs::remove_dir_all(root.as_ref()).expect("remove temp root");
}

#[test]
fn malformed_noncanonical_and_identity_mismatched_plans_fail_closed() {
    let root = temp_dir("release-build-reject");
    let nonce = ReleaseBuildNonce::from_random_bytes([13; 32]);
    let plan = plan_release_build_with_nonce(&root, nonce).expect("plan release build");
    let canonical = fs::read(&plan.path).expect("read canonical plan");
    let other_id = ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([17; 32]));

    fs::write(
        &plan.path,
        encode_record(ReleaseBuildPlanRecord {
            release_build_id: other_id,
            ..plan.record
        }),
    )
    .expect("write nonce-mismatched plan");
    std::assert_matches!(
        load_release_build_plan(&root, plan.record.release_build_id),
        Err(ReleaseBuildPlanError::NonceIdentityMismatch { .. })
    );

    let mut noncanonical = canonical.clone();
    let state_offset = noncanonical.len() - 1;
    noncanonical.splice(state_offset.., [0x18, 0x00]);
    fs::write(&plan.path, noncanonical).expect("write noncanonical plan");
    std::assert_matches!(
        load_release_build_plan(&root, plan.record.release_build_id),
        Err(ReleaseBuildPlanError::InvalidDocument { .. })
    );

    fs::write(&plan.path, canonical).expect("restore canonical plan");
    let other_path = release_build_plan_path(&root, other_id);
    fs::create_dir_all(other_path.parent().expect("other parent")).expect("create other parent");
    fs::copy(&plan.path, &other_path).expect("copy plan under wrong identity");
    std::assert_matches!(
        load_release_build_plan(&root, other_id),
        Err(ReleaseBuildPlanError::PathIdentityMismatch { .. })
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[cfg(unix)]
#[test]
fn plan_and_manifest_symlinks_are_rejected() {
    use std::os::unix::fs::symlink;

    let root = temp_dir("release-build-symlink");
    let nonce = ReleaseBuildNonce::from_random_bytes([19; 32]);
    let plan = plan_release_build_with_nonce(&root, nonce).expect("plan release build");
    let real_manifest = root.join("real-release-set.json");
    let linked_manifest = root.join("release-set.json");
    fs::write(&real_manifest, b"{}").expect("write real manifest");
    symlink(&real_manifest, &linked_manifest).expect("link manifest");

    std::assert_matches!(
        finalize_release_build_from_manifest(&root, plan.record.release_build_id, &linked_manifest,),
        Err(ReleaseBuildPlanError::UnsafeFile { .. })
    );

    fs::remove_dir_all(root).expect("remove temp root");
}
