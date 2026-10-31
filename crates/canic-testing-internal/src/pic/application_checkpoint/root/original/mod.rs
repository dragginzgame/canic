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

use canic_contracts::{
    dto::application_checkpoint::RootCheckpointRequest, ids::ApplicationCheckpointPhase,
};
use canic_host::backup::checkpoint::{
    CheckpointDriver, CheckpointIngress, CheckpointObservation, CheckpointProvider,
    CheckpointRecoveryProvider, CheckpointUnitRecord, FrozenCheckpoint, reconcile_checkpoint,
};
use ic_backup::{
    model::{
        artifacts::ArtifactChecksumRecord as Digest,
        attempt_journal::AttemptJournalRecord,
        execution_workflow::ExecutionStageBindingRecord,
        operation_plan::{PlanContextRecord, PlanContextRequest},
    },
    ops::persistence::{
        BackupLayoutGuard, ExecutionStageGuard, create_execution_workflow,
        read_execution_workflow_progress,
    },
    view::execution_workflow::ExecutionStageProgressState,
    workflow::execution_workflow::{
        ExecutionWorkflowAdvance, ExecutionWorkflowRunError, RetainedStageEvidence,
        advance_execution_workflow, run_retained_execution_workflow,
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
    pending_evidence: Option<String>,
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
        reserved: &ic_backup::model::application_ingress::ApplicationMutationRequest<'_>,
        journal: &AttemptJournalRecord,
    ) -> impl Future<Output = Result<Vec<u8>, Self::Error>> {
        reserved.validate_journal(journal).unwrap();
        std::future::ready((|| {
            assert_eq!(
                journal.authority().binding().request(),
                request.binding().hash()
            );
            self.calls += 1;
            let key = format!(
                "{}-mutation-{}",
                reserved.authority().digest().hash(),
                reserved.mutation_attempt()
            );
            self.pending_evidence = Some(key.clone());
            retain(
                &self.evidence.join(format!("{key}-request.wire")),
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
                &self.evidence.join(format!(
                    "{}-reply.wire",
                    self.pending_evidence
                        .take()
                        .expect("original submitted reservation")
                )),
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
    let before_observation = members
        .iter()
        .map(|member| super::view(pic, *member))
        .collect::<Vec<_>>();
    assert!(
        super::status_as(
            pic,
            root,
            Principal::self_authenticating(b"checkpoint-outsider")
        )
        .is_err()
    );
    assert_eq!(super::status(pic, root), Some(prepared.clone()));
    assert_eq!(super::status(pic, root).as_ref(), Some(&prepared));
    assert_eq!(
        members
            .iter()
            .map(|member| super::view(pic, *member))
            .collect::<Vec<_>>(),
        before_observation
    );
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
            pending_evidence: None,
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
    // Exclusive simulator/controller custody qualifies this provider, not a live trust source.
    assert_eq!(root.to_text(), intent.request(1).unwrap().receiver());
    let recovered = ready(reconcile_checkpoint(
        layout,
        intent,
        &stage,
        &mut InstalledRecovery {
            pic,
            evidence: layout.root(),
        },
    ))
    .unwrap();
    let first = read_binding(layout.root(), 0);
    let reply = read_mutation_reply(layout.root(), 0);
    BTreeMap::from([
        (
            0,
            RetainedStageEvidence::new(first.digest(), Digest::from_bytes(&reply)),
        ),
        (1, recovered),
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
            pending_evidence: None,
            lose_second: true,
        },
    )
    .unwrap();
    create_execution_workflow(layout, intent.workflow()).unwrap();
    assert!(
        ready(advance_execution_workflow(
            layout,
            &intent.workflow().digest(),
            &BTreeMap::new(),
            std::num::NonZeroUsize::new(2).unwrap(),
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
            pending_evidence: None,
            lose_second: false,
        },
    )
    .unwrap();
    let progress = read_execution_workflow_progress(layout, &intent.workflow().digest()).unwrap();
    assert_eq!(
        progress
            .stages
            .iter()
            .filter(|stage| matches!(stage.state, ExecutionStageProgressState::Unprepared))
            .count(),
        member_count - 1
    );
    assert!(matches!(
        ready(advance_execution_workflow(
            layout,
            &intent.workflow().digest(),
            &retained,
            std::num::NonZeroUsize::new(member_count - 1).unwrap(),
            &mut cold,
        ))
        .unwrap(),
        ExecutionWorkflowAdvance::Complete(_)
    ));
    assert_eq!(cold.provider().calls, member_count - 1);
    let final_sequence = member_count as u64;
    let binding = read_binding(&directory, final_sequence);
    let raw = read_mutation_reply(&directory, final_sequence);
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
                Digest::from_bytes(&read_mutation_reply(layout.root(), 0))
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
            pending_evidence: None,
            lose_second: false,
        },
    )
    .unwrap();
    assert!(matches!(
        ready(advance_execution_workflow(
            layout,
            &intent.workflow().digest(),
            &retained,
            std::num::NonZeroUsize::new(1).unwrap(),
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

struct InstalledRecovery<'a> {
    pic: &'a PocketIc,
    evidence: &'a Path,
}

#[expect(
    clippy::future_not_send,
    reason = "Backup providers retain typed non-Send qualification errors and impose no executor contract"
)]
impl CheckpointRecoveryProvider for InstalledRecovery<'_> {
    type Error = InstalledRecoveryError;

    fn admit(
        &mut self,
        request: &CheckpointObservation,
        unit: &CheckpointUnitRecord,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        // This test retains exclusive simulator/controller custody across the entire operation.
        assert_eq!(request.receiver(), unit.root().root.to_text());
        std::future::ready(Ok(()))
    }

    fn observe(
        &mut self,
        request: &CheckpointObservation,
        reserved: &ic_backup::model::application_ingress::ApplicationObservationRequest<'_>,
        journal: &AttemptJournalRecord,
    ) -> impl Future<Output = Result<Vec<u8>, Self::Error>> {
        reserved.validate_journal(journal).unwrap();
        std::future::ready((|| {
            assert_eq!(
                journal.pending_observation_request(),
                Some(request.binding().hash())
            );
            assert_eq!(
                journal.view().pending_mutation,
                Some(request.mutation_attempt())
            );
            retain(
                &self.evidence.join("original-status-request.wire"),
                request.arguments(),
            );
            let raw = self
                .pic
                .update_call(
                    Principal::from_text(request.receiver()).unwrap(),
                    Principal::anonymous(),
                    request.method(),
                    request.arguments().to_vec(),
                )
                .map_err(InstalledRecoveryError::Observation)?;
            retain(&self.evidence.join("original-status-reply.wire"), &raw);
            Ok(raw)
        })())
    }

    fn qualify(
        &mut self,
        request: &CheckpointObservation,
        reply: &[u8],
    ) -> impl Future<Output = Result<Digest, Self::Error>> {
        // Exact installed-unit state from the exclusively held simulator is fixture evidence.
        std::future::ready(
            request
                .decode(reply)
                .map(|_| Digest::from_bytes(reply))
                .map_err(InstalledRecoveryError::Qualification),
        )
    }
}

#[derive(Debug)]
enum InstalledRecoveryError {
    Observation(ic_platform::pocket_ic::RejectResponse),
    Qualification(canic_host::backup::checkpoint::CheckpointError),
}

impl std::fmt::Display for InstalledRecoveryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Observation(error) => {
                write!(formatter, "checkpoint observation refused: {error:?}")
            }
            Self::Qualification(error) => {
                write!(formatter, "checkpoint qualification refused: {error}")
            }
        }
    }
}

impl std::error::Error for InstalledRecoveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Observation(_) => None,
            Self::Qualification(error) => Some(error),
        }
    }
}

fn read_mutation_reply(root: &Path, sequence: u64) -> Vec<u8> {
    use ic_backup::model::attempt_journal::AppliedReceiptEvidence;
    let journal: AttemptJournalRecord = serde_json::from_slice(
        &fs::read(root.join(format!(
            "execution-stage-{sequence}/attempt-{sequence}.json"
        )))
        .unwrap(),
    )
    .unwrap();
    let Some(AppliedReceiptEvidence::Mutation { attempt, .. }) = journal.applied_receipt() else {
        panic!("direct original mutation receipt required");
    };
    fs::read(root.join(format!(
        "{}-mutation-{attempt}-reply.wire",
        journal.authority().digest().hash()
    )))
    .unwrap()
}
