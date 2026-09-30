//! An uncertified management refusal cannot authorize new ingress after source mutation.

use super::*;
use canic_host::fleet_ensure::ops::capacity_import::observation::CapacityImportObserver;

pub(super) fn uncertified_rejection_keeps_original_request(
    pic: &PocketIc,
    paths: &EnsurePaths,
    store: &CapacityImportJournalStore,
    icp: &IcpCli,
    runtime: &tokio::runtime::Runtime,
    transport: &CapacityImportTransport,
) {
    let previous = store.read().unwrap().unwrap();
    let root = previous.plan.authority.root;
    let operator = previous.plan.authority.operator;
    let previous_owner = Principal::self_authenticating(b"competing capacity owner");
    let source = pic.create_canister_on_subnet(None, None, pic.get_subnet(root).unwrap());
    pic.install_canister(source, b"\0asm\x01\0\0\0".to_vec(), Vec::new(), None);
    pic.stop_canister(source, None).unwrap();
    pic.set_controllers(source, None, vec![operator, previous_owner])
        .unwrap();
    let context = runtime.block_on(transport.root_context(root)).unwrap();
    let plan = admission::review(pic, review(pic, root, &[source], operator, &context));
    let staged = publication::bind(
        paths,
        &journal::reviewed(plan.clone()).unwrap(),
        Path::new("policy.toml"),
        Path::new("seed.toml"),
    )
    .unwrap();
    store.stage_review(staged).unwrap();
    let mut reader = CapacityImportLiveObserver::from_icp(icp).unwrap();
    let staged = store.read().unwrap().unwrap();
    let staged = publication::reserve_inspection(&staged, root).unwrap();
    store.save(&staged).unwrap();
    let destination = runtime.block_on(reader.destination(&plan)).unwrap();
    let staged = publication::reserve_inspection(&staged, source).unwrap();
    store.save(&staged).unwrap();
    let observed = runtime.block_on(reader.source(&plan, source)).unwrap();
    let approved = journal::approve(
        &staged,
        plan.plan_sha256,
        &destination,
        std::slice::from_ref(&observed),
    )
    .unwrap();
    store.save(&approved).unwrap();
    let approved = publication::reserve_submission(&approved, "reserve").unwrap();
    store.save(&approved).unwrap();
    let status = runtime.block_on(transport.reserve_root(&approved)).unwrap();
    let reserved =
        journal::reserve(&approved, reservation_evidence(&plan, &status).unwrap()).unwrap();
    store.save(&reserved).unwrap();
    let request = transport.prepare(&plan, source).unwrap();
    let intent = journal::prepare_handoff(&reserved, &observed, request).unwrap();
    store.save(&intent).unwrap();
    let intent = publication::reserve_submission(&intent, "0:handoff").unwrap();
    store.save(&intent).unwrap();
    let issued = journal::issue_handoff(&intent, source).unwrap();
    store.save(&issued).unwrap();

    // Management authorization rejects before ingress admission. No certified request
    // outcome exists, so this HTTP refusal cannot grant a different request identity.
    pic.set_controllers(source, Some(previous_owner), vec![previous_owner])
        .unwrap();
    assert!(matches!(
        runtime.block_on(transport.submit(&issued, source)),
        Err(CapacityImportJournalError::Unresolved)
    ));
    assert!(matches!(
        runtime.block_on(transport.completion(&issued, source)),
        Err(CapacityImportJournalError::Unresolved)
    ));
    assert_eq!(store.read().unwrap().unwrap(), issued);
    pic.set_controllers(
        source,
        Some(previous_owner),
        observed.binding.controllers.clone(),
    )
    .unwrap();
    let restored = runtime.block_on(reader.source(&plan, source)).unwrap();
    assert_eq!(restored.binding.controllers, observed.binding.controllers);
    assert_eq!(
        restored.binding.canister_version,
        observed.binding.canister_version + 2
    );
    let new_request = transport.prepare(&plan, source).unwrap();
    assert!(matches!(
        journal::rejection::renew(&issued, &restored, new_request),
        Err(CapacityImportJournalError::Unresolved)
    ));
    assert_eq!(store.read().unwrap().unwrap(), issued);
    let status = runtime.block_on(transport.root_status(&plan)).unwrap();
    assert!(matches!(
        status.progress[0],
        PoolImportSourceProgress::AwaitingHandoff
    ));
}
