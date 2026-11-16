//! Admission after earliest dispatch retains the original bounded approval and timer owner.
use super::*;

async fn stage_upgrade(
    root: &Client<'_>,
    input: &stored::Stage<upgrade::Approval>,
) -> Result<upgrade::Status, management::Refusal> {
    root.update(
        "stage_stored_upgrade",
        stored::STAGE_UPGRADE,
        &Request {
            slot: ChildId::FIRST,
            value: input,
        },
        stored::LIMITS,
    )
    .await
}

#[tokio::test(flavor = "current_thread")]
async fn elapsed_earliest_dispatch_creates_and_upgrades_without_renewing_expiry() {
    let (_, binary) = support::inputs();
    let owner = support::instance(&binary).await;
    let pic = owner.instance();
    let (root, supplier, mut input, second) = fixture(pic, false).await;
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    input.approval.execute_at_ns = now - 1_000_000_000;
    input.approval.expires_at_ns = now + 120_000_000_000;
    let mut expired = input.clone();
    expired.approval.expires_at_ns = now;
    let before = pic.get_stable_memory(root.id).await;
    assert_eq!(stage(&root, &expired).await, Err(management::Refusal::Time));
    assert_eq!(pic.get_stable_memory(root.id).await, before);
    let admitted = stage(&root, &input).await.unwrap();
    assert_eq!(admitted.approval.as_ref(), Some(&input.approval));
    assert_eq!(admitted.attempts, [0; 3]);
    // Only replica rounds/time pass; the operator does not advance or renew the operation.
    settle(pic).await;
    let created = custody(&root).await;
    let creation = &created.children[0].creation;
    assert_eq!(creation.phase, management::Phase::Ready, "{created:?}");
    assert_eq!(creation.attempts, [1; 3]);
    assert_eq!(creation.approval.as_ref(), Some(&input.approval));
    let worker = CanisterId::from_text(creation.worker.as_ref().unwrap()).unwrap();
    assert_eq!(
        pic.canister_status(worker, Some(root.id))
            .await
            .unwrap()
            .module_hash,
        Some(input.approval.module_hash.to_vec())
    );
    maintain(&root).await;
    assert_eq!(custody(&root).await, created);

    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let change = stored::Stage {
        approval: upgrade::Approval {
            manager: root.id.to_text(),
            child: worker.to_text(),
            previous_module: input.approval.module_hash,
            module_hash: second.hash,
            execute_at_ns: now - 1_000_000_000,
            expires_at_ns: now + 120_000_000_000,
        },
        reference: stored::Reference {
            store: supplier.id.to_text(),
            manifest: second,
        },
    };
    let mut expired = change.clone();
    expired.approval.expires_at_ns = now;
    assert_eq!(
        stage_upgrade(&root, &expired).await,
        Err(management::Refusal::Time)
    );
    assert_eq!(custody(&root).await, created);
    let admitted = stage_upgrade(&root, &change).await.unwrap();
    assert_eq!(admitted.approval.as_ref(), Some(&change.approval));
    settle(pic).await;
    let completed = custody(&root).await;
    assert_eq!(
        completed.children[0].upgrades[0].phase,
        upgrade::Phase::Ready,
        "{completed:?}"
    );
    assert_eq!(completed.children[0].upgrades[0].attempts, [1; 2]);
    assert_eq!(
        completed.children[0].upgrades[0].approval.as_ref(),
        Some(&change.approval)
    );
    assert_eq!(
        pic.canister_status(worker, Some(root.id))
            .await
            .unwrap()
            .module_hash,
        Some(change.approval.module_hash.to_vec())
    );
    maintain(&root).await;
    assert_eq!(custody(&root).await, completed);
    pic.advance_time(Duration::from_secs(121)).await;
    assert_eq!(stage(&root, &input).await.unwrap().attempts, [1; 3]);
    assert_eq!(
        stage_upgrade(&root, &change).await.unwrap(),
        completed.children[0].upgrades[0]
    );
    assert_eq!(custody(&root).await, completed);
}

#[tokio::test(flavor = "current_thread")]
async fn future_dispatch_still_waits_and_expired_unstarted_work_does_not_create() {
    let (_, binary) = support::inputs();
    let owner = support::instance(&binary).await;
    let pic = owner.instance();
    let (root, _, mut input, _) = fixture(pic, false).await;
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    input.approval.execute_at_ns = now + 60_000_000_000;
    input.approval.expires_at_ns = now + 120_000_000_000;
    stage(&root, &input).await.unwrap();
    settle(pic).await;
    let pending = custody(&root).await;
    assert_eq!(
        pending.children[0].creation.phase,
        management::Phase::Scheduled
    );
    assert_eq!(pending.children[0].creation.attempts, [0; 3]);
    assert!(pending.children[0].creation.worker.is_none());
    // Passing the original expiry before the timer runs never grants a fresh window.
    pic.advance_time(Duration::from_secs(121)).await;
    settle(pic).await;
    let expired = custody(&root).await;
    assert_eq!(
        expired.children[0].creation.phase,
        management::Phase::Expired
    );
    assert_eq!(expired.children[0].creation.attempts, [0; 3]);
    assert_eq!(
        expired.children[0].creation.approval.as_ref(),
        Some(&input.approval)
    );
    assert!(expired.children[0].creation.worker.is_none());
    assert_eq!(
        stage(&root, &input).await.unwrap().phase,
        management::Phase::Expired
    );
    assert_eq!(custody(&root).await, expired);
}
