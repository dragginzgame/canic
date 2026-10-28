//! Native original ingress/accounting proofs; these fixtures perform no IC effects.

use crate::checkpoint::{
    CheckpointDriver, CheckpointError, CheckpointExecutionError, CheckpointIngress,
    CheckpointProvider, CheckpointUnitRecord, FrozenCheckpoint,
};

use std::{
    collections::BTreeMap,
    fs,
    future::Future,
    io,
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
    ops::persistence::BackupLayoutGuard,
    workflow::execution_workflow::{
        ExecutionWorkflowRunError, RetainedStageEvidence, run_new_execution_workflow,
        run_retained_execution_workflow,
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
        journal: &AttemptJournalRecord,
    ) -> impl Future<Output = Result<Vec<u8>, Self::Error>> {
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
    let path = crate::test_support::temp_dir("canic-original-checkpoint");
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
