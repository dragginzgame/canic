//! Original host-ingress accounting over installed Root checkpoint effects.
//!
//! The fixture separately owns simulator snapshot/load authority. It does not
//! qualify a production Root-paid snapshot dispatcher or live CLI activation.

use crate::pic::application_checkpoint::{caller_authority_status, increment, view};

use std::{
    collections::BTreeMap,
    fs,
    future::Future,
    io,
    path::{Path, PathBuf},
    pin::pin,
    task::{Context, Poll, Waker},
};

use canic_backup::checkpoint::{
    CheckpointDriver, CheckpointIngress, CheckpointProvider, CheckpointUnitRecord, FrozenCheckpoint,
};
use canic_contracts::{
    dto::application_checkpoint::{RootCheckpointRequest, RootCheckpointStatus},
    ids::ApplicationCheckpointPhase,
    protocol::application_checkpoint as protocol,
};
use ic_backup::{
    model::{
        artifacts::ArtifactChecksumRecord as Digest,
        attempt_journal::{
            AttemptJournalRecord, ObservationOutcomeRecord, ObservationReceiptRequest,
        },
        execution_workflow::ExecutionStageBindingRecord,
        operation_plan::{PlanContextRecord, PlanContextRequest},
    },
    ops::persistence::{AttemptJournalGuard, BackupLayoutGuard, ExecutionStageGuard},
    workflow::execution_workflow::{
        ExecutionWorkflowRunError, RetainedStageEvidence, run_new_execution_workflow,
        run_retained_execution_workflow,
    },
};
use ic_host_fs::durable::{PublicationMode, WriteOptions, write_bytes};
use ic_platform::{pocket_ic::PocketIc, principal::ExternalPrincipal as Principal};
use ic_testkit::pic::{CanisterSnapshotTarget, PocketIcSnapshotExt, SnapshotRestoreFunding};

struct InstalledProvider<'a> {
    pic: &'a PocketIc,
    root: Principal,
    evidence: PathBuf,
    calls: usize,
    lose_second: bool,
    authority: CheckpointUnitRecord,
}
impl CheckpointProvider for InstalledProvider<'_> {
    type Error = io::Error;
    fn admit(
        &mut self,
        request: &CheckpointIngress,
        unit: &CheckpointUnitRecord,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        std::future::ready({
            // One exclusive fixture controller owns the backend throughout this phase.
            // Only these fixed checkpoint commands can run; installation, membership,
            // release and controller authority cannot change. No hidden observations run.
            assert_eq!(unit, &self.authority);
            assert_eq!(request.receiver(), self.root.to_text());
            Ok(())
        })
    }
    fn submit(
        &mut self,
        request: &CheckpointIngress,
        journal: &AttemptJournalRecord,
    ) -> impl Future<Output = Result<Vec<u8>, Self::Error>> {
        std::future::ready((|| {
            assert_eq!(
                journal.authority().binding().request(),
                request.binding().hash()
            );
            self.calls += 1;
            retain(
                &self
                    .evidence
                    .join(format!("{}-request.wire", request.binding().hash())),
                request.arguments(),
            );
            let reply = self
                .pic
                .update_call(
                    self.root,
                    Principal::anonymous(),
                    request.method(),
                    request.arguments().to_vec(),
                )
                .unwrap();
            if self.lose_second && self.calls == 2 {
                return Err(io::ErrorKind::TimedOut.into());
            }
            Ok(reply)
        })())
    }
    fn qualify(
        &mut self,
        request: &CheckpointIngress,
        reply: &[u8],
    ) -> impl Future<Output = Result<Digest, Self::Error>> {
        std::future::ready({
            request.decode(reply).unwrap();
            retain(
                &self
                    .evidence
                    .join(format!("{}-reply.wire", request.binding().hash())),
                reply,
            );
            Ok(Digest::from_bytes(reply))
        })
    }
}

fn retain(path: &Path, bytes: &[u8]) {
    write_bytes(
        path,
        WriteOptions {
            mode: PublicationMode::CreateNew,
            permissions: 0o600,
        },
        bytes,
    )
    .unwrap();
}
fn ready<T>(future: impl Future<Output = T>) -> T {
    match pin!(future)
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("blocking simulator fixture unexpectedly pending"),
    }
}

#[test]
pub(in crate::pic) fn qualify_original_host_workflow() {
    let (fixture, _) = super::install();
    let pic = &fixture.pic;
    let root = fixture.metadata.root_id;
    let mut members: Vec<_> = fixture
        .metadata
        .snapshot_pids
        .iter()
        .copied()
        .filter(|pid| !fixture.metadata.managed_store_pids.contains(pid))
        .collect();
    members.sort();
    assert_eq!(
        members.len(),
        2,
        "this acceptance unit has two participants"
    );
    let (intent, unit, layout) = prepare(pic, root, &members);
    let directory = layout.root().to_path_buf();
    let retained = run_prepare(pic, root, &unit, &intent, &layout, members.len());
    let prepared = super::status(pic, root).unwrap();
    assert_eq!(prepared.phase, ApplicationCheckpointPhase::Prepared);
    assert_eq!(prepared.acknowledged as usize, members.len());
    for member in &members {
        assert!(increment(pic, *member).is_err());
    }
    qualify_capture_restore(pic, root, &members);
    let before = super::status(pic, root);
    let mut replay = CheckpointDriver::new(
        &layout,
        FrozenCheckpoint::read(&layout).unwrap(),
        InstalledProvider {
            pic,
            root,
            authority: unit,
            evidence: directory,
            calls: 0,
            lose_second: false,
        },
    )
    .unwrap();
    ready(run_retained_execution_workflow(
        &layout,
        &intent.workflow().digest(),
        &retained,
        &mut replay,
    ))
    .unwrap();
    assert_eq!(replay.provider().calls, 0);
    assert_eq!(super::status(pic, root), before);
    super::qualify_release(
        pic,
        root,
        &members,
        &RootCheckpointRequest {
            root_install_id: prepared.root_install_id,
            operation_id: prepared.operation_id,
            sequence: prepared.sequence,
        },
    );
}

fn read_binding(root: &Path, sequence: u64) -> ExecutionStageBindingRecord {
    let bytes = ic_host_fs::read::read_file(
        &root.join(format!("execution-stage-{sequence}/stage-binding.json")),
        512 * 1024,
    )
    .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn reconcile_original(
    pic: &PocketIc,
    root: Principal,
    layout: &BackupLayoutGuard,
    intent: &FrozenCheckpoint,
) -> BTreeMap<u64, RetainedStageEvidence> {
    let binding = read_binding(layout.root(), 1);
    let stage =
        ExecutionStageGuard::open(layout, &intent.workflow().digest(), 1, &binding.digest())
            .unwrap();
    let authority = stage.plan().attempt_authority(1).unwrap();
    let mut journal = AttemptJournalGuard::open(stage.layout().unwrap(), &authority).unwrap();
    let arguments =
        ic_wire::native::encode_request(protocol::STATUS, &(), protocol::LIMITS).unwrap();
    let request = Digest::from_bytes(
        &ic_wire::native::encode(
            &(
                "canic.fixture.original-root-status",
                intent.workflow().digest(),
                root,
                "canic_root_checkpoint_status",
                &arguments,
            ),
            protocol::LIMITS,
        )
        .unwrap(),
    );
    let observation = journal
        .reserve_planned_observation(&stage.plan().digest(), 1, request.hash())
        .unwrap();
    retain(
        &layout.root().join("original-status-request.wire"),
        &arguments,
    );
    let raw = pic
        .query_call(
            root,
            Principal::anonymous(),
            "canic_root_checkpoint_status",
            arguments,
        )
        .unwrap();
    let status: Result<Option<RootCheckpointStatus>, canic::Error> =
        ic_wire::native::decode_response(protocol::STATUS, &raw, protocol::LIMITS).unwrap();
    // Exclusive controller custody and the exact installed unit qualify this
    // original transition. No request or raw reply is reconstructed from a digest.
    assert_eq!(
        status.unwrap().as_ref(),
        Some(intent.request(1).unwrap().expected_status())
    );
    retain(&layout.root().join("original-status-reply.wire"), &raw);
    let learned = Digest::from_bytes(&raw);
    journal
        .record_observation(ObservationReceiptRequest {
            attempt: observation,
            request: request.hash().into(),
            outcome: ObservationOutcomeRecord::Applied,
            evidence: learned.hash().into(),
        })
        .unwrap();
    drop(journal);
    stage.reconcile_checkpoint(learned.clone()).unwrap();
    let first = read_binding(layout.root(), 0);
    let reply = fs::read(layout.root().join(format!(
        "{}-reply.wire",
        intent.request(0).unwrap().binding().hash()
    )))
    .unwrap();
    BTreeMap::from([
        (
            0,
            RetainedStageEvidence::new(first.digest(), Digest::from_bytes(&reply)),
        ),
        (1, RetainedStageEvidence::new(binding.digest(), learned)),
    ])
}

fn qualify_capture_restore(pic: &PocketIc, root: Principal, members: &[Principal]) {
    let before: Vec<_> = members.iter().map(|member| view(pic, *member)).collect();
    // These direct simulator management effects have separate fixture authority;
    // the adapter's host receipts never authorize or settle Root-paid calls.
    let snapshots = pic
        .capture_snapshots(members.iter().map(|member| CanisterSnapshotTarget {
            canister_id: *member,
            sender: Some(root),
        }))
        .unwrap();
    for member in members {
        pic.stop_canister(*member, Some(root)).unwrap();
    }
    pic.restore_snapshots(&snapshots, SnapshotRestoreFunding::Preserve)
        .unwrap();
    for member in members {
        pic.start_canister(*member, Some(root)).unwrap();
    }
    for (member, expected) in members.iter().zip(&before) {
        assert_eq!(&view(pic, *member), expected);
    }
}

fn prepare(
    pic: &PocketIc,
    root: Principal,
    members: &[Principal],
) -> (FrozenCheckpoint, CheckpointUnitRecord, BackupLayoutGuard) {
    let authorities: Vec<_> = members
        .iter()
        .map(|member| {
            caller_authority_status(pic, *member, root)
                .unwrap()
                .authority
        })
        .collect();
    let root_identity = authorities[0].issuer.clone();
    let actual = pic
        .canister_status(root, Some(Principal::anonymous()))
        .unwrap();
    let context = PlanContextRecord::new(&PlanContextRequest {
        // This baseline installs an explicit qualification network identity. It
        // has no NNS root key and supplies no live trust-anchor admission proof.
        network: root_identity
            .registry
            .binding
            .fleet
            .fleet
            .canonical_network_id
            .to_string(),
        caller: Principal::anonymous().to_text(),
        release: Digest::from_digest(actual.module_hash.unwrap().try_into().unwrap())
            .hash()
            .into(),
    })
    .unwrap();
    assert!(
        actual
            .settings
            .controllers
            .contains(&Principal::anonymous())
    );
    let unit =
        CheckpointUnitRecord::new(context, root_identity, authorities, [0xd1; 32], 0).unwrap();
    let intent = FrozenCheckpoint::new(unit.clone()).unwrap();
    let parent = crate::workspace::discover().unwrap().join("target");
    fs::create_dir_all(&parent).unwrap();
    let directory = ic_host_fs::directory::create_unique_directory(
        &parent,
        "original-root-checkpoint-",
        0o700,
        None,
    )
    .unwrap();
    let layout = BackupLayoutGuard::acquire(&directory).unwrap();
    intent.retain(&layout).unwrap();
    (intent, unit, layout)
}

fn run_prepare(
    pic: &PocketIc,
    root: Principal,
    unit: &CheckpointUnitRecord,
    intent: &FrozenCheckpoint,
    layout: &BackupLayoutGuard,
    member_count: usize,
) -> BTreeMap<u64, RetainedStageEvidence> {
    let directory = layout.root().to_path_buf();
    let mut driver = CheckpointDriver::new(
        layout,
        intent.clone(),
        InstalledProvider {
            pic,
            root,
            authority: unit.clone(),
            evidence: directory.clone(),
            calls: 0,
            lose_second: true,
        },
    )
    .unwrap();
    assert!(
        ready(run_new_execution_workflow(
            layout,
            intent.workflow(),
            &mut driver
        ))
        .is_err()
    );
    assert_eq!(driver.provider().calls, 2);
    refuse_pending(pic, root, unit, intent, layout);
    let mut retained = reconcile_original(pic, root, layout, intent);
    let mut cold = CheckpointDriver::new(
        layout,
        FrozenCheckpoint::read(layout).unwrap(),
        InstalledProvider {
            pic,
            root,
            authority: unit.clone(),
            evidence: directory.clone(),
            calls: 0,
            lose_second: false,
        },
    )
    .unwrap();
    ready(run_retained_execution_workflow(
        layout,
        &intent.workflow().digest(),
        &retained,
        &mut cold,
    ))
    .unwrap();
    assert_eq!(cold.provider().calls, member_count - 1);
    let final_sequence = member_count as u64;
    let binding = read_binding(&directory, final_sequence);
    let raw = fs::read(directory.join(format!(
        "{}-reply.wire",
        intent.request(final_sequence).unwrap().binding().hash()
    )))
    .unwrap();
    retained.insert(
        final_sequence,
        RetainedStageEvidence::new(binding.digest(), Digest::from_bytes(&raw)),
    );
    retained
}

fn refuse_pending(
    pic: &PocketIc,
    root: Principal,
    unit: &CheckpointUnitRecord,
    intent: &FrozenCheckpoint,
    layout: &BackupLayoutGuard,
) {
    let original = fs::read(layout.root().join("execution-stage-1/attempt-1.json")).unwrap();
    let retained = (0..=1)
        .map(|sequence| {
            let binding = read_binding(layout.root(), sequence);
            let learned = if sequence == 0 {
                Digest::from_bytes(
                    &fs::read(layout.root().join(format!(
                        "{}-reply.wire",
                        intent.request(0).unwrap().binding().hash(),
                    )))
                    .unwrap(),
                )
            } else {
                // Supplying a digest never settles the original pending attempt.
                Digest::from_bytes(b"unsettled")
            };
            (
                sequence,
                RetainedStageEvidence::new(binding.digest(), learned),
            )
        })
        .collect();
    let mut cold = CheckpointDriver::new(
        layout,
        intent.clone(),
        InstalledProvider {
            pic,
            root,
            authority: unit.clone(),
            evidence: layout.root().to_path_buf(),
            calls: 0,
            lose_second: false,
        },
    )
    .unwrap();
    assert!(matches!(
        ready(run_retained_execution_workflow(
            layout,
            &intent.workflow().digest(),
            &retained,
            &mut cold,
        )),
        Err(ExecutionWorkflowRunError::Checkpoint(_))
    ));
    assert_eq!(cold.provider().calls, 0);
    assert_eq!(
        fs::read(layout.root().join("execution-stage-1/attempt-1.json")).unwrap(),
        original
    );
}
