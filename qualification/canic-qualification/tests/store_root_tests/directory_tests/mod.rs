//! Root derives the shared directory from its own ready service installations.
use super::*;
use canic::sharding::standby::StandbyPoolError;
use canic::{
    method::Contract,
    topology::directory::{
        Delivery, Directory, Hop, ReceiveDirectory, RedeliverDirectoryRevision, RedeliveryError,
        Service, ServiceInstall,
    },
};

async fn publish(root: &Client<'_>, name: &str, revision: u64) -> Result<u64, RedeliveryError> {
    root.update(
        RedeliverDirectoryRevision::NAME,
        RedeliverDirectoryRevision::ID,
        &(name, revision),
        canic::method::LIMITS,
    )
    .await
}

async fn resolve(pic: &PocketIc, hub: CanisterId) -> Option<(u64, Vec<CanisterId>)> {
    query_service(pic, hub, "resolve_service", "hub").await
}

async fn query_service<T: serde::de::DeserializeOwned>(
    pic: &PocketIc,
    hub: CanisterId,
    method: &str,
    role: impl serde::Serialize,
) -> T {
    let frame = |direction| ic_wire::Encoding::Frame {
        method: ic_wire::method_id(method),
        direction,
    };
    let reply = pic
        .query_call(
            hub,
            CanisterId::anonymous(),
            method,
            ic_wire::encode(
                &role,
                frame(ic_wire::Direction::Request),
                canic::method::LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    ic_wire::decode(
        &reply,
        frame(ic_wire::Direction::Response),
        canic::method::LIMITS,
    )
    .unwrap()
}

async fn refused_delivery(
    pic: &PocketIc,
    target: CanisterId,
    caller: CanisterId,
    role: &str,
    directory: Directory,
) -> canic::topology::directory::DirectoryError {
    let update = Delivery {
        backup_timing: None,
        directory,
        path: vec![Hop {
            shard_pool: None,
            canister: target.to_text().parse().unwrap(),
            role: role.into(),
            generation: [0; 32],
        }],
    };
    let reply = pic
        .update_call(
            target,
            caller,
            ReceiveDirectory::NAME,
            ReceiveDirectory::encode_request(&update).unwrap(),
        )
        .await
        .unwrap();
    ReceiveDirectory::decode_reply(&reply).unwrap().unwrap_err()
}

async fn snapshot_readers(root: &Client<'_>, child: CanisterId) {
    use ic_platform_protocol::management_snapshots as snapshots;
    use ic_testkit::pocket_ic::common::rest::RawEffectivePrincipal;
    let pic = root.pic;
    let alice = CanisterId::self_authenticating(b"first snapshot reader");
    let bob = CanisterId::self_authenticating(b"second snapshot reader");
    pic.stop_canister(child, Some(root.id)).await.unwrap();
    let snapshot = pic
        .take_canister_snapshot(child, Some(root.id), None)
        .await
        .unwrap();
    pic.start_canister(child, Some(root.id)).await.unwrap();
    let target = child.to_text().parse().unwrap();
    let read = async |viewer| {
        pic.update_call_with_effective_principal(
            CanisterId::management_canister(),
            RawEffectivePrincipal::CanisterId(child.as_slice().to_vec()),
            viewer,
            "read_canister_snapshot_data",
            snapshots::ReadDataArgs {
                canister_id: target,
                snapshot_id: snapshot.id.clone(),
                kind: snapshots::ReadKind::WasmModule(snapshots::Extent { offset: 0, size: 4 }),
            }
            .encode(4096)
            .unwrap(),
        )
        .await
    };
    let grant = stored::SnapshotReaders {
        root: root.id.to_text().parse().unwrap(),
        child: target,
        readers: [alice, bob]
            .into_iter()
            .map(|p| p.to_text().parse().unwrap())
            .collect(),
    };
    let configure = async |input: &stored::SnapshotReaders| -> Result<(), management::Failure> {
        root.update(
            stored::ConfigureSnapshotReaders::NAME,
            stored::ConfigureSnapshotReaders::ID,
            input,
            stored::ConfigureSnapshotReaders::REQUEST,
        )
        .await
    };
    assert!(read(alice).await.is_err());
    assert!(
        pic.update_call(
            root.id,
            alice,
            stored::ConfigureSnapshotReaders::NAME,
            stored::ConfigureSnapshotReaders::encode_request(&grant).unwrap()
        )
        .await
        .is_err()
    );
    configure(&grant).await.unwrap();
    for viewer in [alice, bob] {
        assert_eq!(
            pic.list_canister_snapshots(child, Some(viewer))
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(read(viewer).await.is_ok());
    }
    assert!(
        pic.delete_canister_snapshot(child, Some(alice), snapshot.id.clone())
            .await
            .is_err()
    );
    configure(&stored::SnapshotReaders {
        readers: vec![],
        ..grant
    })
    .await
    .unwrap();
    assert!(read(alice).await.is_err());
    assert!(read(root.id).await.is_ok());
}

mod capacity;
mod funding;
mod peer;
mod retirement;
mod role_proof;
mod singleton;

enum Scenario {
    Capacity(u8),
    ThreeServices,
    ProjectLedgers,
    Funding,
    RoleProof,
    Retirement { from_spare: bool },
    BothHubs,
    Peer,
    StoppedHub,
    OperatorReplay,
    StoppedChild,
    Descendant,
}

#[tokio::test]
async fn receiver_guards_named_service_and_rechecks_writes_after_await() {
    installed_publication(Scenario::Peer).await;
}

#[tokio::test]
async fn readiness_publishes_to_existing_and_new_services() {
    installed_publication(Scenario::BothHubs).await;
}

#[tokio::test]
async fn root_reopen_catches_up_a_service_that_missed_publication() {
    installed_publication(Scenario::StoppedHub).await;
}

#[tokio::test]
async fn replay_catches_up_a_reopened_hub_without_restarting_root() {
    installed_publication(Scenario::OperatorReplay).await;
}

#[tokio::test]
async fn stopped_child_lags_while_hub_advances_and_recovers_after_hub_upgrade() {
    installed_publication(Scenario::StoppedChild).await;
}

#[tokio::test]
async fn root_installs_descendant_with_hub_as_its_directory_publisher() {
    installed_publication(Scenario::Descendant).await;
}

#[tokio::test]
async fn root_installs_separate_ledgers_for_two_registry_projects() {
    installed_publication(Scenario::ProjectLedgers).await;
}

#[tokio::test]
async fn two_hubs_compete_for_the_last_subnet_slot() {
    installed_publication(Scenario::Capacity(3)).await;
}

#[tokio::test]
async fn one_full_pool_leaves_capacity_for_the_other_hub() {
    installed_publication(Scenario::Capacity(4)).await;
}

#[tokio::test]
async fn one_hub_assigns_the_same_key_in_independent_named_pools() {
    installed_publication(Scenario::Capacity(5)).await;
}

#[tokio::test]
async fn third_permanent_service_and_two_named_hub_pools_keep_their_custody() {
    installed_publication(Scenario::ThreeServices).await;
}

#[tokio::test]
async fn scheduled_permanent_child_blocks_a_second_direct_creation() {
    installed_publication(Scenario::Capacity(1)).await;
}

#[tokio::test]
async fn shard_renews_and_uses_role_until_retirement() {
    installed_publication(Scenario::RoleProof).await;
}

#[tokio::test]
async fn drained_shard_returns_as_a_worker_after_proof_expiry() {
    installed_publication(Scenario::Retirement { from_spare: false }).await;
}

#[tokio::test]
async fn standby_backed_shard_returns_to_its_original_inventory_row() {
    installed_publication(Scenario::Retirement { from_spare: true }).await;
}

#[tokio::test]
async fn only_the_approved_direct_child_receives_recurring_funding() {
    installed_publication(Scenario::Funding).await;
}

async fn installed_publication(scenario: Scenario) {
    let capacity = match scenario {
        Scenario::Capacity(limit) => Some(limit),
        Scenario::RoleProof => Some(4),
        Scenario::ThreeServices => Some(5),
        Scenario::ProjectLedgers => Some(6),
        Scenario::Funding => Some(2),
        Scenario::Retirement { from_spare } => Some(if from_spare { 4 } else { 3 }),
        _ => None,
    };
    let module = "root.wasm";
    let (mut server, instance) = support::instance(&canic_qualification::server_directory()).await;
    let pic = instance.client();
    let (root, supplier) =
        readiness_tests::fixture_with_root(pic, std::fs::read(artifacts().join(module)).unwrap())
            .await;
    let manifest = supplier
        .publish_named(
            "hub",
            "1",
            &std::fs::read(artifacts().join("directory.wasm")).unwrap(),
        )
        .await;
    let retirement_releases = if matches!(scenario, Scenario::Retirement { .. }) {
        let publish = async |name: &str| {
            supplier
                .publish_named(
                    name,
                    "1",
                    &std::fs::read(artifacts().join(format!("{name}.wasm"))).unwrap(),
                )
                .await
        };
        Some((publish("dormant").await, publish("recycled-miner").await))
    } else {
        None
    };
    pic.set_controllers(supplier.id, Some(root.operator), vec![root.id])
        .await
        .unwrap();
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let source = stored::Stage {
        approval: management::Approval {
            manager: root.id.to_text(),
            module_hash: manifest.hash,
            creation_cycles: management::CREATION_CYCLES,
            execute_at_ns: now + 10_000_000_000,
            expires_at_ns: now + 240_000_000_000,
        },
        reference: stored::Reference {
            store: supplier.id.to_text(),
            manifest,
        },
    };
    let stage = async |slot, args: Vec<u8>| -> Result<management::Status, management::Refusal> {
        root.update(
            "stage_child_creation",
            canic_management_protocol::application::STAGE_CHILD_CREATION,
            &Request {
                slot,
                value: canic_management_protocol::application::Deployment::<(), _>::Application(
                    management::application::Stage {
                        source: source.clone(),
                        arguments: management::application::Arguments::new(args).unwrap(),
                    },
                ),
            },
            stored::LIMITS,
        )
        .await
    };
    let pin = if matches!(scenario, Scenario::RoleProof | Scenario::Retirement { .. }) {
        Some(role_proof::prepare(&root).await)
    } else {
        None
    };
    let named = |name: &str| {
        ServiceInstall {
            funding_operator: Some(root.operator.to_text().parse().unwrap()),
            hub_operator: Some(root.operator.to_text().parse().unwrap()),
            name: name.into(),
            role: "hub".into(),
            application: if name == "accounts_hub" {
                pin.clone().unwrap_or_default()
            } else {
                vec![]
            },
        }
        .encode()
        .unwrap()
    };
    let ready = async |slot| {
        settle(pic).await;
        let status: management::Status = root.observe_creation(slot).await;
        assert_eq!(status.phase, management::Phase::Ready, "{status:?}");
        let id = CanisterId::from_text(status.worker.unwrap()).unwrap();
        pic.add_cycles(id, 20_000_000_000_000).await;
        id
    };
    let prefilled = matches!(scenario, Scenario::Capacity(4));
    if prefilled {
        capacity::approve(&root, 4).await;
        let policy = stored::StandbyPolicy {
            root: root.id.to_text(),
            minimum_available: 4,
            maximum_creations: 4,
            expires_at_ns: source.approval.expires_at_ns,
            minimum_import_cycles: 1u64.try_into().unwrap(),
            imports: vec![],
        };
        root.update::<_, Result<(), StandbyPoolError>>(
            "configure_standby",
            stored::CONFIGURE_STANDBY,
            &policy,
            stored::LIMITS,
        )
        .await
        .unwrap();
        settle(pic).await;
        assert_eq!(capacity::counts(&root).await.ready, 4);
    }
    stage(ChildId::FIRST, named("accounts_hub")).await.unwrap();
    if matches!(scenario, Scenario::Capacity(1)) {
        capacity::approve(&root, 1).await;
        assert_eq!(
            stage(ChildId::SECOND, named("project_hub")).await,
            Err(management::Refusal::Occupied)
        );
        stage(ChildId::FIRST, named("accounts_hub")).await.unwrap();
        let status: management::Status = root.observe_creation(ChildId::SECOND).await;
        assert_eq!(status.phase, management::Phase::Empty);
        assert_eq!(
            instance.close(Duration::from_secs(10)).await,
            ic_testkit::server::Deletion::Attempted
        );
        assert!(server.stop().unwrap().process.cleanup.is_empty());
        return;
    }
    assert_eq!(
        publish(&root, "accounts_hub", 1).await,
        Err(RedeliveryError::NoReadyServices)
    );
    if let Some(limit) = capacity {
        // First hub already has an outstanding install. Second hub will stage under the ceiling.
        capacity::approve(&root, limit).await;
    }
    pic.advance_time(Duration::from_secs(11)).await;
    let user = ready(ChildId::FIRST).await;
    assert_eq!(resolve(pic, user).await, Some((1, vec![user])));
    use canic::topology::directory::{BackupTiming, ReadBackupTiming};
    let reply = pic
        .query_call(
            user,
            CanisterId::anonymous(),
            ReadBackupTiming::NAME,
            ReadBackupTiming::encode_request(&()).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        ReadBackupTiming::decode_reply(&reply).unwrap(),
        Some(BackupTiming {
            revision: 1,
            daily_at_utc_secs: None,
        })
    );
    if matches!(scenario, Scenario::BothHubs) {
        snapshot_readers(&root, user).await;
    }
    type Single = Option<(u64, Result<CanisterId, String>)>;
    let unique =
        async |hub, role: &str| -> Single { query_service(pic, hub, "unique_service", role).await };
    assert_eq!(unique(user, "hub").await, Some((1, Ok(user))));
    assert_eq!(
        unique(user, "unknown").await,
        Some((1, Err("service role missing".into())))
    );
    let outsider = CanisterId::self_authenticating(b"directory observer");
    let request = ic_wire::encode(
        &("accounts_hub", 1_u64),
        ic_wire::Encoding::Frame {
            method: RedeliverDirectoryRevision::ID,
            direction: ic_wire::Direction::Request,
        },
        canic::method::LIMITS,
    )
    .unwrap();
    let denial = pic
        .update_call(root.id, outsider, RedeliverDirectoryRevision::NAME, request)
        .await
        .unwrap_err();
    assert!(
        denial.reject_message.contains("controller required"),
        "{denial:?}"
    );
    assert_eq!(resolve(pic, user).await, Some((1, vec![user])));

    assert_eq!(publish(&root, "accounts_hub", 1).await, Ok(1));
    assert_eq!(resolve(pic, user).await, Some((1, vec![user])));
    if matches!(scenario, Scenario::BothHubs) {
        // Ordinary growth cannot rename an installed service or claim its name
        // for a different installation, even with a valid controller approval.
        let before = pic.get_stable_memory(root.id).await;
        assert_eq!(
            stage(ChildId::FIRST, named("project_hub")).await,
            Err(management::Refusal::Occupied)
        );
        assert_eq!(
            stage(ChildId::SECOND, named("accounts_hub")).await,
            Err(management::Refusal::Binding)
        );
        let retry = stage(ChildId::FIRST, named("accounts_hub")).await.unwrap();
        assert_eq!(retry.worker, Some(user.to_text()));
        assert_eq!(retry.attempts, [1; 3]);
        assert_eq!(pic.get_stable_memory(root.id).await, before);
        assert_eq!(unique(user, "hub").await, Some((1, Ok(user))));
    }
    if matches!(scenario, Scenario::Descendant) {
        let install = canic::topology::Install {
            role: "worker".into(),
            root: root.id.to_text().parse().unwrap(),
            parent: user.to_text().parse().unwrap(),
            application: vec![],
        };
        // A well-formed topology for another installing Root cannot spend this Root's slot.
        let wrong_root = canic::topology::Install {
            role: "worker".into(),
            root: install.parent,
            parent: install.root,
            application: vec![],
        };
        assert_eq!(
            stage(ChildId::SECOND, wrong_root.encode().unwrap()).await,
            Err(management::Refusal::Binding)
        );
        // Even a real canister under Root controller authority is not an
        // application parent unless Root has its ready installation record.
        let foreign_parent = canic::topology::Install {
            parent: supplier.id.to_text().parse().unwrap(),
            root: install.root,
            role: install.role.clone(),
            application: vec![],
        };
        assert_eq!(
            stage(ChildId::SECOND, foreign_parent.encode().unwrap()).await,
            Err(management::Refusal::Binding)
        );
        let untouched: management::Status = root.observe_creation(ChildId::SECOND).await;
        assert_eq!(untouched.phase, management::Phase::Empty);
        assert_eq!(untouched.attempts, [0; 3]);
        stage(ChildId::SECOND, install.encode().unwrap())
            .await
            .unwrap();
        let child = ready(ChildId::SECOND).await;
        assert_eq!(resolve(pic, child).await, Some((1, vec![user])));
        let status: (String, Option<Option<u64>>) =
            query_service(pic, user, "topology_status", child).await;
        assert_eq!(status, ("hub".into(), Some(Some(1))));
        assert_eq!(pic.get_controllers(child).await, [root.id]);
        let wasm = std::fs::read(artifacts().join("directory.wasm")).unwrap();
        for caller in [user, root.operator] {
            let refused = pic
                .upgrade_canister(child, wasm.clone(), vec![], Some(caller))
                .await
                .expect_err("only Root may replace a descendant's Wasm");
            assert_eq!(
                refused.error_code,
                ic_testkit::pocket_ic::ErrorCode::CanisterInvalidController
            );
        }
        canic_qualification::upgrade_stopped(pic, child, wasm, vec![], Some(root.id))
            .await
            .unwrap();
        let status: (String, Option<Option<u64>>) =
            query_service(pic, child, "topology_status", user).await;
        assert_eq!(status, ("worker".into(), None));
        funding::application_parent(&root, user, child, "worker").await;
        let before = pic.get_stable_memory(child).await;
        for (sender, role, expected) in [
            (
                outsider,
                "worker",
                canic::topology::directory::DirectoryError::Unauthorized,
            ),
            (
                user,
                "hub",
                canic::topology::directory::DirectoryError::InvalidDelivery,
            ),
            (
                user,
                "worker",
                canic::topology::directory::DirectoryError::GenerationChanged,
            ),
        ] {
            assert_eq!(
                refused_delivery(
                    pic,
                    child,
                    sender,
                    role,
                    Directory {
                        revision: 2,
                        services: vec![]
                    }
                )
                .await,
                expected
            );
        }
        assert_eq!(pic.get_stable_memory(child).await, before);
        assert_eq!(resolve(pic, child).await, Some((1, vec![user])));
        assert_eq!(
            instance.close(Duration::from_secs(10)).await,
            ic_testkit::server::Deletion::Attempted
        );
        assert!(server.stop().unwrap().process.cleanup.is_empty());
        return;
    }
    let stopped_child = if matches!(scenario, Scenario::StoppedChild) {
        capacity::approve(&root, 3).await;
        capacity::approve_projects(&root, user, &source, 1).await;
        let child = capacity::ready_project(&root, user, 1).await;
        assert_eq!(resolve(pic, child).await, Some((1, vec![user])));
        pic.stop_canister(child, Some(root.id)).await.unwrap();
        Some(child)
    } else {
        None
    };
    let miss_delivery = matches!(
        scenario,
        Scenario::StoppedHub | Scenario::OperatorReplay | Scenario::Peer
    );
    if miss_delivery {
        pic.stop_canister(user, Some(root.id)).await.unwrap();
    }
    stage(ChildId::SECOND, named("project_hub")).await.unwrap();
    let project = ready(ChildId::SECOND).await;
    if matches!(scenario, Scenario::ProjectLedgers) {
        let parent = singleton::exercise(&root, project, &source, module).await;
        let duplicate = canic::topology::Install {
            role: "project_ledger".into(),
            root: root.id.to_text().parse().unwrap(),
            parent: parent.to_text().parse().unwrap(),
            application: vec![],
        };
        assert_eq!(
            stage(ChildId::HISTORICAL_SERVICES[2], duplicate.encode().unwrap()).await,
            Err(management::Refusal::Binding)
        );
        assert_eq!(
            instance.close(Duration::from_secs(10)).await,
            ic_testkit::server::Deletion::Attempted
        );
        assert!(server.stop().unwrap().process.cleanup.is_empty());
        return;
    }
    if matches!(scenario, Scenario::Funding) {
        funding::exercise(&root, project, user).await;
        assert_eq!(
            instance.close(Duration::from_secs(10)).await,
            ic_testkit::server::Deletion::Attempted
        );
        assert!(server.stop().unwrap().process.cleanup.is_empty());
        return;
    }
    if matches!(scenario, Scenario::ThreeServices) {
        let third = ChildId::HISTORICAL_SERVICES[2];
        stage(third, named("discovery_hub")).await.unwrap();
        let discovery = ready(third).await;
        for (slot, name, original) in [
            (ChildId::FIRST, "accounts_hub", user),
            (ChildId::SECOND, "project_hub", project),
            (third, "discovery_hub", discovery),
        ] {
            assert_eq!(
                stage(slot, named(name)).await.unwrap().worker,
                Some(original.to_text())
            );
            assert_eq!(
                resolve(pic, original).await,
                Some((3, vec![user, discovery, project]))
            );
        }
        capacity::separate_hub_pools(&root, &source, user, discovery).await;
        assert_eq!(
            instance.close(Duration::from_secs(10)).await,
            ic_testkit::server::Deletion::Attempted
        );
        assert!(server.stop().unwrap().process.cleanup.is_empty());
        return;
    }
    if let Some((dormant, miner)) = retirement_releases {
        retirement::exercise(
            &root,
            user,
            project,
            &source,
            dormant,
            miner,
            matches!(scenario, Scenario::Retirement { from_spare: true }),
        )
        .await;
        assert_eq!(
            instance.close(Duration::from_secs(10)).await,
            ic_testkit::server::Deletion::Attempted
        );
        assert!(server.stop().unwrap().process.cleanup.is_empty());
        return;
    }
    if matches!(scenario, Scenario::BothHubs) {
        let sibling = Client {
            pic,
            id: project,
            operator: root.operator,
        };
        let before = pic.get_stable_memory(user).await;
        assert_eq!(
            sibling
                .update::<_, CanisterId>(
                    "attempt_delivery",
                    ic_wire::method_id("attempt_delivery"),
                    &(),
                    canic::method::LIMITS
                )
                .await,
            user
        );
        assert_eq!(pic.get_stable_memory(user).await, before);
    }

    let latest = Some((2, vec![user, project]));
    assert_eq!(resolve(pic, project).await, latest);
    assert_eq!(
        unique(project, "hub").await,
        Some((2, Err("service role ambiguous".into())))
    );
    assert_eq!(
        publish(&root, "project_hub", 1).await,
        Err(RedeliveryError::RevisionChanged)
    );
    if miss_delivery {
        assert_eq!(
            publish(&root, "accounts_hub", 2).await,
            Err(RedeliveryError::DeliveryUnconfirmed)
        );
        pic.start_canister(user, Some(root.id)).await.unwrap();
        assert_eq!(resolve(pic, user).await, Some((1, vec![user])));
    } else {
        assert_eq!(resolve(pic, user).await, latest);
    }
    if matches!(scenario, Scenario::Peer) {
        peer::exercise(&root, user, project).await;
    }
    if matches!(scenario, Scenario::StoppedHub) {
        capacity::approve(&root, 3).await;
        capacity::approve_projects(&root, project, &source, 1).await;
        let child = capacity::ready_project(&root, project, 1).await;
        assert_eq!(resolve(pic, child).await, latest);
        // The child joined the project branch. Its readiness cannot replay the
        // missed publication to the independently restarted accounts hub.
        assert_eq!(resolve(pic, user).await, Some((1, vec![user])));
    }
    if let Some(child) = stopped_child {
        assert_eq!(
            publish(&root, "accounts_hub", 2).await,
            Err(RedeliveryError::DeliveryUnconfirmed)
        );
        canic_qualification::upgrade_stopped(
            pic,
            user,
            std::fs::read(artifacts().join("directory.wasm")).unwrap(),
            vec![],
            Some(root.id),
        )
        .await
        .unwrap();
        let status: (String, Option<Option<u64>>) =
            query_service(pic, user, "topology_status", child).await;
        assert_eq!(status.1, Some(Some(1)));
        assert_eq!(resolve(pic, user).await, latest);
        pic.start_canister(child, Some(root.id)).await.unwrap();
        assert_eq!(resolve(pic, child).await, Some((1, vec![user])));
        assert_eq!(publish(&root, "accounts_hub", 2).await, Ok(2));
        assert_eq!(resolve(pic, child).await, latest);
        let status: (String, Option<Option<u64>>) =
            query_service(pic, user, "topology_status", child).await;
        assert_eq!(status.1, Some(Some(2)));
    }
    if matches!(scenario, Scenario::OperatorReplay) {
        canic_qualification::upgrade_stopped(
            pic,
            user,
            std::fs::read(artifacts().join("directory.wasm")).unwrap(),
            vec![],
            Some(root.id),
        )
        .await
        .unwrap();
        assert_eq!(unique(user, "hub").await, Some((1, Ok(user))));
        assert_eq!(publish(&root, "accounts_hub", 2).await, Ok(2));
        // Inject delayed publisher frames at the installed receiver. The normal
        // Root command above only emits the current, complete projection.
        let retained = pic.get_stable_memory(user).await;
        for revision in [1, 2] {
            let old_members = Directory {
                revision,
                services: vec![Service {
                    name: "accounts_hub".into(),
                    role: "hub".into(),
                    canister: user.to_text().parse().unwrap(),
                }],
            };
            assert_eq!(
                refused_delivery(pic, user, root.id, "hub", old_members).await,
                canic::topology::directory::DirectoryError::RevisionConflict
            );
            assert_eq!(pic.get_stable_memory(user).await, retained);
        }
        assert_eq!(resolve(pic, user).await, latest);
        assert_eq!(
            unique(user, "hub").await,
            Some((2, Err("service role ambiguous".into())))
        );
    }
    if let Some(limit) = capacity {
        let key_limit = if matches!(scenario, Scenario::RoleProof | Scenario::Retirement { .. }) {
            3
        } else {
            1
        };
        capacity::exercise(&root, &source, user, project, limit, key_limit).await;
        if prefilled {
            let counts = capacity::counts(&root).await;
            assert_eq!((counts.ready, counts.creations_used), (0, 4));
            assert_eq!(counts.refill, stored::RefillStatus::AllowanceSpent);
            let rows = root
                .update::<_, <canic::sharding::standby::ObserveStandby as Contract>::Output>(
                    "observe_standby",
                    canic::sharding::standby::ObserveStandby::ID,
                    &(),
                    canic::sharding::standby::ObserveStandby::REPLY,
                )
                .await;
            assert_eq!(rows.len(), 4);
            for (_, asset) in rows {
                let canic::sharding::standby::Asset::Claimed(id, allocation) = asset else {
                    panic!("one spare per installed child")
                };
                let slot = ChildId::placement(allocation.slot as usize).unwrap();
                let status: management::Status = root.observe_creation(slot).await;
                assert_eq!(status.worker, Some(id.to_text()));
                assert_eq!(status.attempts, [0, 1, 1]);
            }
        }
    }
    canic_qualification::upgrade_stopped(
        pic,
        root.id,
        std::fs::read(artifacts().join(module)).unwrap(),
        vec![],
        Some(root.operator),
    )
    .await
    .unwrap();
    settle(pic).await;
    assert_eq!(resolve(pic, user).await, latest);
    assert_eq!(resolve(pic, project).await, latest);
    let bytes = pic.get_stable_memory(user).await;
    assert_eq!(publish(&root, "accounts_hub", 2).await, Ok(2));
    assert_eq!(pic.get_stable_memory(user).await, bytes);
    if let Some(ref pin) = pin {
        let generation = management::application::Stage {
            source,
            arguments: management::application::Arguments::new(named("project_hub")).unwrap(),
        }
        .pin(management::application::STAGE, ChildId::SECOND)
        .operation;
        role_proof::exercise(&root, user, project, pin.clone(), generation).await;
    }
    assert_eq!(
        instance.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}
