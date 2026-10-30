//! Native original ingress/accounting proofs; these fixtures perform no IC effects.

use crate::backup::checkpoint::{
    CheckpointDriver, CheckpointError, CheckpointExecutionError, CheckpointIngress,
    CheckpointProvider, CheckpointUnitRecord, FrozenCheckpoint,
};

use std::{
    collections::BTreeMap,
    fs,
    future::Future,
    io,
    num::NonZeroUsize,
    pin::pin,
    task::{Context, Poll, Waker},
};

use canic_contracts::{
    ids::{
        AppId, CallerInstallation, CallerReceiverAuthority, CallerRootAuthority, CanisterRole,
        CanonicalNetworkId, ComponentBinding, ComponentInstanceId, ComponentSpecId, FleetBinding,
        FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority, ManagedCanisterBinding,
        SubnetId,
    },
    protocol::application_checkpoint as protocol,
};
use ic_backup::{
    model::{
        artifacts::ArtifactChecksumRecord as Digest,
        attempt_journal::AttemptJournalRecord,
        execution_workflow::ExecutionStageBindingRecord,
        operation_plan::{PlanContextRecord, PlanContextRequest},
    },
    ops::persistence::{
        BackupLayoutGuard, create_execution_workflow, read_execution_workflow_progress,
    },
    view::execution_workflow::ExecutionStageProgressState,
    workflow::execution_workflow::{
        ExecutionWorkflowAdvance, ExecutionWorkflowRunError, RetainedStageEvidence,
        advance_execution_workflow, run_new_execution_workflow, run_retained_execution_workflow,
    },
};
use ic_platform::principal::ExternalPrincipal as Principal;

fn ready<T>(future: impl Future<Output = T>) -> T {
    match pin!(future)
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("native fixture future unexpectedly pending"),
    }
}

fn unit() -> CheckpointUnitRecord {
    let registry = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                    fleet_id: FleetId::from_generated_bytes([7; 32]),
                },
                app: AppId::from("fixture"),
            },
            coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[9])),
            coordinator: Principal::from_slice(&[10]),
            recovery_controllers: vec![],
        },
        epoch: 1,
    };
    let root = CallerRootAuthority {
        registry: registry.clone(),
        root: Principal::from_slice(&[13]),
        install_id: [3; 32],
    };
    let members = [14, 15]
        .into_iter()
        .map(|id| CallerReceiverAuthority {
            receiver: CallerInstallation {
                binding: ManagedCanisterBinding::Component(ComponentBinding {
                    authority: registry.clone(),
                    component: ComponentInstanceId::from_generated_bytes([id; 32]),
                    component_spec: ComponentSpecId::try_from("default".to_string()).unwrap(),
                    spec_hash: [12; 32],
                    role: CanisterRole::from("test"),
                    placement_subnet: SubnetId::from_principal(Principal::from_slice(&[8])),
                    fleet_subnet_root: root.root,
                    canister_id: Principal::from_slice(&[id]),
                }),
                install_id: [1; 32],
                component_install_id: [2; 32],
            },
            issuer: root.clone(),
            policy_digest: [4; 32],
        })
        .collect();
    let context = PlanContextRecord::new(&PlanContextRequest {
        network: registry
            .binding
            .fleet
            .fleet
            .canonical_network_id
            .to_string(),
        caller: Principal::anonymous().to_text(),
        release: "ab".repeat(32),
    })
    .unwrap();
    CheckpointUnitRecord::new(context, root, members, [6; 32], 0).unwrap()
}

struct NativeProvider {
    calls: usize,
    lose_second: bool,
    replies: Vec<Vec<u8>>,
    reply_override: Option<Vec<u8>>,
}
impl CheckpointProvider for NativeProvider {
    type Error = io::Error;
    fn admit(
        &mut self,
        _: &CheckpointIngress,
        _: &CheckpointUnitRecord,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        std::future::ready(Ok(()))
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
            if let Some(reply) = self.reply_override.take() {
                return Ok(reply);
            }
            if self.lose_second && self.calls == 2 {
                return Err(io::ErrorKind::TimedOut.into());
            }
            let id = if request.method() == "canic_root_checkpoint_begin" {
                protocol::BEGIN
            } else {
                protocol::ADVANCE
            };
            let reply = ic_wire::native::encode_response(
                id,
                &Ok::<_, canic_contracts::dto::error::Error>(request.expected_status().clone()),
                protocol::LIMITS,
            )
            .unwrap();
            self.replies.push(reply.clone());
            Ok(reply)
        })())
    }
    fn qualify(
        &mut self,
        _: &CheckpointIngress,
        reply: &[u8],
    ) -> impl Future<Output = Result<Digest, Self::Error>> {
        std::future::ready(Ok(Digest::from_bytes(reply)))
    }
}

fn fixture() -> (std::path::PathBuf, FrozenCheckpoint) {
    let path = std::env::temp_dir().join(format!(
        "canic-original-checkpoint-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&path).unwrap();
    (path, FrozenCheckpoint::new(unit()).unwrap())
}

fn provider(lose_second: bool) -> NativeProvider {
    NativeProvider {
        calls: 0,
        lose_second,
        replies: vec![],
        reply_override: None,
    }
}

#[test]
fn complete_original_checkpoint_and_cold_replay_do_not_touch_root_paid_evidence() {
    let (path, intent) = fixture();
    let layout = BackupLayoutGuard::acquire(&path).unwrap();
    intent.retain(&layout).unwrap();
    fs::write(
        path.join("separate-root-paid-evidence.json"),
        br#"{"pending":true,"original_limit":1}"#,
    )
    .unwrap();
    let paid = fs::read(path.join("separate-root-paid-evidence.json")).unwrap();
    let mut driver = CheckpointDriver::new(&layout, intent.clone(), provider(false)).unwrap();
    ready(run_new_execution_workflow(
        &layout,
        intent.workflow(),
        &mut driver,
    ))
    .unwrap();
    let retained = driver
        .provider()
        .replies
        .iter()
        .enumerate()
        .map(|(index, reply)| {
            let binding: ExecutionStageBindingRecord = serde_json::from_slice(
                &fs::read(path.join(format!("execution-stage-{index}/stage-binding.json")))
                    .unwrap(),
            )
            .unwrap();
            (
                index as u64,
                RetainedStageEvidence::new(binding.digest(), Digest::from_bytes(reply)),
            )
        })
        .collect();
    let mut cold = CheckpointDriver::new(
        &layout,
        FrozenCheckpoint::read(&layout).unwrap(),
        provider(false),
    )
    .unwrap();
    ready(run_retained_execution_workflow(
        &layout,
        &intent.workflow().digest(),
        &retained,
        &mut cold,
    ))
    .unwrap();
    assert_eq!(cold.provider().calls, 0);
    assert_eq!(
        fs::read(path.join("separate-root-paid-evidence.json")).unwrap(),
        paid
    );
}

#[test]
fn lost_ingress_keeps_original_pending_and_never_redispatches() {
    let (path, intent) = fixture();
    let layout = BackupLayoutGuard::acquire(&path).unwrap();
    intent.retain(&layout).unwrap();
    let mut driver = CheckpointDriver::new(&layout, intent.clone(), provider(true)).unwrap();
    assert!(
        ready(run_new_execution_workflow(
            &layout,
            intent.workflow(),
            &mut driver
        ))
        .is_err()
    );
    assert_eq!(driver.provider().calls, 2);
    let original = fs::read(path.join("execution-stage-1/attempt-1.json")).unwrap();
    let mut cold = CheckpointDriver::new(
        &layout,
        FrozenCheckpoint::read(&layout).unwrap(),
        provider(false),
    )
    .unwrap();
    assert!(
        ready(run_retained_execution_workflow(
            &layout,
            &intent.workflow().digest(),
            &BTreeMap::new(),
            &mut cold
        ))
        .is_err()
    );
    assert!(
        ready(run_new_execution_workflow(
            &layout,
            intent.workflow(),
            &mut cold
        ))
        .is_err()
    );
    assert_eq!(cold.provider().calls, 0);
    assert_eq!(
        fs::read(path.join("execution-stage-1/attempt-1.json")).unwrap(),
        original
    );
}

#[test]
fn wrong_original_reply_preserves_bytes_and_pending_spending() {
    let (path, intent) = fixture();
    let layout = BackupLayoutGuard::acquire(&path).unwrap();
    intent.retain(&layout).unwrap();
    let mut wrong = intent.request(0).unwrap().expected_status().clone();
    wrong.operation_id[0] ^= 1;
    let raw = ic_wire::native::encode_response(
        protocol::BEGIN,
        &Ok::<_, canic_contracts::dto::error::Error>(wrong),
        protocol::LIMITS,
    )
    .unwrap();
    let mut source = provider(false);
    source.reply_override = Some(raw.clone());
    let mut driver = CheckpointDriver::new(&layout, intent.clone(), source).unwrap();
    let error = ready(run_new_execution_workflow(
        &layout,
        intent.workflow(),
        &mut driver,
    ))
    .unwrap_err();
    match error {
        ExecutionWorkflowRunError::Driver(CheckpointExecutionError::AfterReply {
            reply, ..
        }) => {
            assert_eq!(reply, raw);
        }
        other => panic!("unexpected error: {other}"),
    }
    assert_eq!(driver.provider().calls, 1);
    let original = fs::read(path.join("execution-stage-0/attempt-0.json")).unwrap();
    let mut cold = CheckpointDriver::new(
        &layout,
        FrozenCheckpoint::read(&layout).unwrap(),
        provider(false),
    )
    .unwrap();
    assert!(
        ready(run_retained_execution_workflow(
            &layout,
            &intent.workflow().digest(),
            &BTreeMap::new(),
            &mut cold,
        ))
        .is_err()
    );
    assert_eq!(cold.provider().calls, 0);
    assert_eq!(
        fs::read(path.join("execution-stage-0/attempt-0.json")).unwrap(),
        original
    );
    assert!(!path.join("execution-stage-1").exists());
}

#[test]
fn changed_retained_unit_or_context_cannot_rebind_an_original_workflow() {
    for field in [
        "context",
        "root",
        "members",
        "operation_id",
        "expected_sequence",
    ] {
        let (path, intent) = fixture();
        let layout = BackupLayoutGuard::acquire(&path).unwrap();
        intent.retain(&layout).unwrap();
        let mut value = serde_json::to_value(&intent.record).unwrap();
        match field {
            "context" => value[field]["release"] = serde_json::json!("cd".repeat(32)),
            "root" => value[field]["install_id"][0] = serde_json::json!(99),
            "members" => {
                value[field].as_array_mut().unwrap().pop();
            }
            "operation_id" => value[field][0] = serde_json::json!(88),
            "expected_sequence" => value[field] = serde_json::json!(9),
            _ => unreachable!(),
        }
        fs::write(
            path.join("checkpoint-intent.json"),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            CheckpointDriver::new(&layout, intent, provider(false)),
            Err(CheckpointError::OriginalMismatch | CheckpointError::UnitMismatch)
        ));
    }
}

#[test]
fn reordered_members_normalize_and_wrong_root_identity_or_duplicate_members_reject() {
    let original = unit();
    let mut reversed = original.members().to_vec();
    reversed.reverse();
    let context = original.context().clone();
    assert_eq!(
        CheckpointUnitRecord::new(
            context.clone(),
            original.root().clone(),
            reversed,
            [6; 32],
            0
        )
        .unwrap(),
        original
    );
    let duplicate = vec![original.members()[0].clone(); 2];
    assert!(
        CheckpointUnitRecord::new(
            context.clone(),
            original.root().clone(),
            duplicate,
            [6; 32],
            0
        )
        .is_err()
    );
    let mut wrong = original.members().to_vec();
    wrong[0].issuer.install_id[0] ^= 1;
    assert!(
        CheckpointUnitRecord::new(context, original.root().clone(), wrong, [6; 32], 0).is_err()
    );
}

#[cfg(unix)]
#[test]
fn retained_intent_rejects_symlink_substitution_and_create_only_overwrite() {
    let (path, intent) = fixture();
    let layout = BackupLayoutGuard::acquire(&path).unwrap();
    intent.retain(&layout).unwrap();
    assert!(matches!(
        intent.retain(&layout),
        Err(CheckpointError::Persistence(_))
    ));
    let retained = path.join("checkpoint-intent.json");
    let bytes = fs::read(&retained).unwrap();
    let moved = path.join("substituted.json");
    fs::rename(&retained, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &retained).unwrap();
    assert!(FrozenCheckpoint::read(&layout).is_err());
    assert_eq!(fs::read(moved).unwrap(), bytes);
    drop(layout);
    fs::remove_dir_all(path).unwrap();
}

struct RecoveryProvider {
    observed: usize,
    admitted: usize,
    reply: Vec<u8>,
    fail_qualification: bool,
    lose_reply: bool,
}
impl super::CheckpointRecoveryProvider for RecoveryProvider {
    type Error = io::Error;
    fn admit(
        &mut self,
        _: &super::CheckpointObservation,
        _: &CheckpointUnitRecord,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        self.admitted += 1;
        std::future::ready(Ok(()))
    }
    fn observe(
        &mut self,
        request: &super::CheckpointObservation,
        reserved: &ic_backup::model::application_ingress::ApplicationObservationRequest<'_>,
        journal: &AttemptJournalRecord,
    ) -> impl Future<Output = Result<Vec<u8>, Self::Error>> {
        reserved.validate_journal(journal).unwrap();
        self.observed += 1;
        assert_eq!(
            journal.pending_observation_request(),
            Some(request.binding().hash())
        );
        assert_eq!(
            journal.view().pending_mutation,
            Some(request.mutation_attempt())
        );
        std::future::ready(if self.lose_reply {
            Err(io::Error::other("fixture lost observation"))
        } else {
            Ok(self.reply.clone())
        })
    }
    fn qualify(
        &mut self,
        _: &super::CheckpointObservation,
        reply: &[u8],
    ) -> impl Future<Output = Result<Digest, Self::Error>> {
        std::future::ready(if self.fail_qualification {
            Err(io::Error::other("fixture qualification refusal"))
        } else {
            Ok(Digest::from_bytes(reply))
        })
    }
}

#[test]
fn original_recovery_observation_settles_once_and_continues_without_mutation_replay() {
    use super::reconcile_checkpoint;
    use ic_backup::ops::persistence::{AttemptJournalGuard, ExecutionStageGuard};
    let (path, intent) = fixture();
    let layout = BackupLayoutGuard::acquire(&path).unwrap();
    intent.retain(&layout).unwrap();
    let mut original = CheckpointDriver::new(&layout, intent.clone(), provider(true)).unwrap();
    assert!(
        ready(run_new_execution_workflow(
            &layout,
            intent.workflow(),
            &mut original
        ))
        .is_err()
    );
    let binding: ExecutionStageBindingRecord = serde_json::from_slice(
        &fs::read(path.join("execution-stage-1/stage-binding.json")).unwrap(),
    )
    .unwrap();
    let stage =
        ExecutionStageGuard::open(&layout, &intent.workflow().digest(), 1, &binding.digest())
            .unwrap();
    let reply = ic_wire::native::encode_response(
        protocol::STATUS,
        &Ok::<_, canic_contracts::dto::error::Error>(Some(
            intent.request(1).unwrap().expected_status().clone(),
        )),
        protocol::LIMITS,
    )
    .unwrap();
    let mut observer = RecoveryProvider {
        observed: 0,
        admitted: 0,
        reply,
        fail_qualification: false,
        lose_reply: false,
    };
    let recovered = ready(reconcile_checkpoint(
        &layout,
        &intent,
        &stage,
        &mut observer,
    ))
    .unwrap();
    let authority = stage.plan().attempt_authority(1).unwrap();
    let journal = AttemptJournalGuard::open(stage.layout().unwrap(), &authority).unwrap();
    let view = journal.record().unwrap().view();
    assert!(view.applied);
    assert_eq!((view.mutations_used, view.observations_used), (1, 1));
    assert_eq!(
        (view.mutations_remaining, view.observations_remaining),
        (0, 0)
    );
    drop(journal);
    assert!(
        ready(reconcile_checkpoint(
            &layout,
            &intent,
            &stage,
            &mut observer
        ))
        .is_err()
    );
    assert_eq!((observer.admitted, observer.observed), (1, 1));
    drop(stage);
    let first: ExecutionStageBindingRecord = serde_json::from_slice(
        &fs::read(path.join("execution-stage-0/stage-binding.json")).unwrap(),
    )
    .unwrap();
    let retained = BTreeMap::from([
        (
            0,
            RetainedStageEvidence::new(
                first.digest(),
                Digest::from_bytes(&original.provider().replies[0]),
            ),
        ),
        (1, recovered),
    ]);
    let mut cold = CheckpointDriver::new(
        &layout,
        FrozenCheckpoint::read(&layout).unwrap(),
        provider(false),
    )
    .unwrap();
    ready(run_retained_execution_workflow(
        &layout,
        &intent.workflow().digest(),
        &retained,
        &mut cold,
    ))
    .unwrap();
    assert_eq!(cold.provider().calls, 1);
}

#[test]
fn failed_recovery_keeps_original_observation_occupied_without_retry_or_refund() {
    use super::{CheckpointRecoveryError, reconcile_checkpoint};
    use ic_backup::ops::persistence::{AttemptJournalGuard, ExecutionStageGuard};
    for failure in [
        "wrong_status",
        "stale_status",
        "foreign_root",
        "qualification",
        "lost_reply",
    ] {
        let (path, intent) = fixture();
        let layout = BackupLayoutGuard::acquire(&path).unwrap();
        intent.retain(&layout).unwrap();
        let mut original = CheckpointDriver::new(&layout, intent.clone(), provider(true)).unwrap();
        assert!(
            ready(run_new_execution_workflow(
                &layout,
                intent.workflow(),
                &mut original
            ))
            .is_err()
        );
        let binding: ExecutionStageBindingRecord = serde_json::from_slice(
            &fs::read(path.join("execution-stage-1/stage-binding.json")).unwrap(),
        )
        .unwrap();
        let stage =
            ExecutionStageGuard::open(&layout, &intent.workflow().digest(), 1, &binding.digest())
                .unwrap();
        let status = match failure {
            "wrong_status" => None,
            "stale_status" => Some(intent.request(0).unwrap().expected_status().clone()),
            "foreign_root" => {
                let mut status = intent.request(1).unwrap().expected_status().clone();
                status.root_install_id = [0xee; 32];
                Some(status)
            }
            _ => Some(intent.request(1).unwrap().expected_status().clone()),
        };
        let reply = ic_wire::native::encode_response(
            protocol::STATUS,
            &Ok::<_, canic_contracts::dto::error::Error>(status),
            protocol::LIMITS,
        )
        .unwrap();
        let mut observer = RecoveryProvider {
            observed: 0,
            admitted: 0,
            reply: reply.clone(),
            fail_qualification: failure == "qualification",
            lose_reply: failure == "lost_reply",
        };
        let error = ready(reconcile_checkpoint(
            &layout,
            &intent,
            &stage,
            &mut observer,
        ))
        .unwrap_err();
        match error {
            CheckpointRecoveryError::AfterReply {
                reply: retained, ..
            } => assert_eq!(retained, reply),
            CheckpointRecoveryError::Observation(_) => assert_eq!(failure, "lost_reply"),
            other => panic!("unexpected recovery result: {other:?}"),
        }
        let authority = stage.plan().attempt_authority(1).unwrap();
        let journal = AttemptJournalGuard::open(stage.layout().unwrap(), &authority).unwrap();
        let view = journal.record().unwrap().view();
        assert_eq!(view.pending_mutation, Some(1));
        // Mutation and observation share Backup's attempt-ID sequence, not separate counters.
        assert!(view.pending_observation.is_some());
        assert_ne!(view.pending_observation, view.pending_mutation);
        assert_eq!((view.mutations_used, view.observations_used), (1, 1));
        assert!(!view.applied);
        drop(journal);
        let before = fs::read(path.join("execution-stage-1/attempt-1.json")).unwrap();
        assert!(
            ready(reconcile_checkpoint(
                &layout,
                &intent,
                &stage,
                &mut observer
            ))
            .is_err()
        );
        assert_eq!((observer.admitted, observer.observed), (1, 1));
        assert_eq!(
            fs::read(path.join("execution-stage-1/attempt-1.json")).unwrap(),
            before
        );
    }
}

struct FixtureAdmission(CheckpointUnitRecord);
impl super::CheckpointAdmission for FixtureAdmission {
    type Error = io::Error;
    fn admit(
        &mut self,
        unit: &CheckpointUnitRecord,
        payload: &ic_backup::model::application_ingress::ApplicationIngressPayload,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        assert_eq!(unit, &self.0);
        assert_eq!(payload.receiver(), unit.root().root.to_text());
        std::future::ready(Ok(()))
    }
}

#[tokio::test]
async fn agent_retention_refusal_preserves_original_spending_without_network_or_resend() {
    use ic_backup_agent::{
        AgentTransport, ApplicationIngressProviderError, PreparedUpdate, ReservedUpdate,
    };
    use ic_platform::agent::{AnonymousIdentity, RequestId};
    use std::{cell::Cell, rc::Rc, sync::Arc, time::Duration};
    let (path, intent) = fixture();
    let layout = BackupLayoutGuard::acquire(&path).unwrap();
    intent.retain(&layout).unwrap();
    let transport = AgentTransport::new(
        intent.record.context().clone(),
        "http://127.0.0.1:1",
        Arc::new(AnonymousIdentity),
        vec![1],
        Duration::from_secs(1),
    )
    .unwrap();
    let retained = Rc::new(Cell::new(0));
    let calls = Rc::clone(&retained);
    let provider = super::AgentCheckpointProvider::new(
        &transport,
        FixtureAdmission(intent.record.clone()),
        move |request: &ReservedUpdate<'_>, prepared: &PreparedUpdate<'_>| {
            assert!(matches!(request, ReservedUpdate::ApplicationMutation(_)));
            assert!(!prepared.envelope().is_empty());
            calls.set(calls.get() + 1);
            Err(ApplicationIngressProviderError::Unavailable)
        },
        |_: &ReservedUpdate<'_>, _: &RequestId, _: &[u8]| {
            panic!("retention refusal must precede all network submission")
        },
    );
    let mut driver = CheckpointDriver::new(&layout, intent.clone(), provider).unwrap();
    let result = run_new_execution_workflow(&layout, intent.workflow(), &mut driver).await;
    assert!(matches!(
        result,
        Err(ExecutionWorkflowRunError::Driver(
            CheckpointExecutionError::Submit(super::AgentCheckpointError::Transport(
                ApplicationIngressProviderError::Unavailable
            ))
        ))
    ));
    assert_eq!(retained.get(), 1);
    let journal_path = path.join("execution-stage-0/attempt-0.json");
    let original = fs::read(&journal_path).unwrap();
    assert!(
        run_retained_execution_workflow(
            &layout,
            &intent.workflow().digest(),
            &BTreeMap::new(),
            &mut driver
        )
        .await
        .is_err()
    );
    assert_eq!(retained.get(), 1);
    assert_eq!(fs::read(journal_path).unwrap(), original);
}

#[test]
fn opaque_begin_payload_commits_complete_unit_and_reply_ceiling() {
    let original = unit();
    let intent = FrozenCheckpoint::new(original.clone()).unwrap();
    let ingress = intent.request(0).unwrap();
    let begin: canic_contracts::dto::application_checkpoint::RootCheckpointBeginRequest =
        ic_wire::native::decode_request(protocol::BEGIN, ingress.arguments(), protocol::LIMITS)
            .unwrap();
    assert_eq!(
        begin.unit_digest,
        protocol::unit_digest(original.members()).unwrap()
    );
    assert_eq!(ingress.binding(), &ingress.payload().digest());
    assert_eq!(ingress.payload().reply_limit(), protocol::LIMITS.bytes);
    let mut changed: serde_json::Value = serde_json::to_value(&original).unwrap();
    changed["members"][0]["policy_digest"][0] = serde_json::json!(99);
    let changed = FrozenCheckpoint::new(serde_json::from_value(changed).unwrap()).unwrap();
    assert_ne!(changed.workflow().digest(), intent.workflow().digest());
    assert_ne!(changed.request(0).unwrap().binding(), ingress.binding());
}

#[test]
fn identical_advance_payloads_keep_distinct_original_authorities() {
    let intent = FrozenCheckpoint::new(unit()).unwrap();
    assert_eq!(
        intent.request(1).unwrap().binding(),
        intent.request(2).unwrap().binding()
    );
    let first = intent.child(1).unwrap().attempt_authority(1).unwrap();
    let second = intent.child(2).unwrap().attempt_authority(2).unwrap();
    assert_ne!(first.digest(), second.digest());
}

#[test]
fn bounded_checkpoint_pauses_and_cold_continues_with_original_evidence() {
    let (path, intent) = fixture();
    let layout = BackupLayoutGuard::acquire(&path).unwrap();
    intent.retain(&layout).unwrap();
    create_execution_workflow(&layout, intent.workflow()).unwrap();
    let digest = intent.workflow().digest();
    let initial = read_execution_workflow_progress(&layout, &digest).unwrap();
    assert!(
        initial
            .stages
            .iter()
            .all(|stage| matches!(stage.state, ExecutionStageProgressState::Unprepared))
    );
    let mut retained = BTreeMap::new();
    for sequence in 0..intent.workflow().stages().len() {
        let mut driver = CheckpointDriver::new(
            &layout,
            FrozenCheckpoint::read(&layout).unwrap(),
            provider(false),
        )
        .unwrap();
        let result = ready(advance_execution_workflow(
            &layout,
            &digest,
            &retained,
            NonZeroUsize::new(1).unwrap(),
            &mut driver,
        ))
        .unwrap();
        assert_eq!(driver.provider().calls, 1);
        assert_eq!(
            matches!(result, ExecutionWorkflowAdvance::Complete(_)),
            sequence + 1 == intent.workflow().stages().len()
        );
        let binding: ExecutionStageBindingRecord = serde_json::from_slice(
            &fs::read(path.join(format!("execution-stage-{sequence}/stage-binding.json"))).unwrap(),
        )
        .unwrap();
        retained.insert(
            sequence as u64,
            RetainedStageEvidence::new(
                binding.digest(),
                Digest::from_bytes(&driver.provider().replies[0]),
            ),
        );
        let progress = read_execution_workflow_progress(&layout, &digest).unwrap();
        assert_eq!(progress.stages.iter().filter(|stage|
            matches!(&stage.state, ExecutionStageProgressState::Prepared { progress, .. }
                if progress.applied_operations == 1)).count(), sequence + 1);
        assert_eq!(driver.provider().calls, 1);
    }
    let mut cold = CheckpointDriver::new(&layout, intent, provider(false)).unwrap();
    assert!(matches!(
        ready(advance_execution_workflow(
            &layout,
            &digest,
            &retained,
            NonZeroUsize::new(1).unwrap(),
            &mut cold
        ))
        .unwrap(),
        ExecutionWorkflowAdvance::Complete(_)
    ));
    assert_eq!(cold.provider().calls, 0);
}
