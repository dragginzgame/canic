//! Executed by the installed Store harness in a separate process to keep supplier graphs separate.
use crate::{DeploymentName, Network, Plan, PlanAction, StoreMaintenance, UpgradeTarget};
use canic_management_protocol::{custody, digest};
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(serde::Deserialize)]
struct Input {
    network: Network,
    root: String,
    store: String,
    expected: custody::Root,
    artifacts: PathBuf,
}

fn retain(journal: &Path, plan: &Plan, wasm: &[u8]) -> String {
    let store = crate::store::Store::open(journal).unwrap();
    crate::plan::publish_locked(&store, plan.clone(), wasm)
        .unwrap()
        .0
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "run scripts/dev/test-store.sh root-maintenance; requires its live local simulator and input"]
async fn root_maintenance_preserves_store_operation() {
    let input_path = PathBuf::from(std::env::var_os("CANIC_ROOT_CUSTODY_INPUT").unwrap());
    let input: Input = crate::store::json(&input_path).unwrap();
    let directory = input_path.parent().unwrap();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let signer = repo.join(".tools/ic/bin/icp");
    let wasm = std::fs::read(input.artifacts.join("root-maintenance.wasm")).unwrap();
    let now = u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    )
    .unwrap();
    let operation = input.expected.store_operation.as_ref().unwrap();
    let plan = Plan {
        version: crate::plan::APPROVAL_VERSION,
        network: input.network,
        controller: "2vxsx-fae".into(),
        artifact: crate::inspect(&input.artifacts.join("root-maintenance.wasm")).unwrap(),
        action: PlanAction::MaintainRoot {
            target: UpgradeTarget {
                canister: input.root,
                module: crate::hex(&digest(
                    &std::fs::read(input.artifacts.join("root-faults.wasm")).unwrap(),
                )),
            },
            children: vec![input.expected.stored.children[0].clone()],
            store: Some(Box::new(StoreMaintenance {
                history: Vec::new(),
                store: UpgradeTarget {
                    canister: input.store,
                    module: crate::hex(&operation.approval.previous_module),
                },
                sources: input.expected.stored.sources.to_vec(),
                operation: Some(operation.clone()),
            })),
        },
        created_at_ns: now,
        expires_at_ns: now + 3_600_000_000_000,
        deployment: Some(DeploymentName {
            app: "custody".into(),
            fleet: "local".into(),
            canister: "root".into(),
        }),
    };
    let journal = directory.join("journal");
    let mut altered = plan.clone();
    let PlanAction::MaintainRoot {
        store: Some(store), ..
    } = &mut altered.action
    else {
        unreachable!()
    };
    store.operation.as_mut().unwrap().approval.attempt.operation[0] ^= 1;
    let bad = retain(&journal, &altered, &wasm);
    assert!(
        crate::apply(&journal, &bad, "anonymous", &signer)
            .await
            .is_err()
    );
    assert!(!journal.join(&bad).join("install/signed.json").exists());

    let id = retain(&journal, &plan, &wasm);
    let pause = directory.join("pause-after-install");
    let quoted_signer = signer.to_str().unwrap().replace('\'', "'\\''");
    std::fs::write(&pause, format!("#!/bin/sh\nfor arg in \"$@\"; do\n case \"$arg\" in */custody-after/arguments.bin) exit 73;; esac\ndone\nexec '{quoted_signer}' \"$@\"\n")).unwrap();
    std::fs::set_permissions(&pause, std::fs::Permissions::from_mode(0o700)).unwrap();
    let interrupted = crate::apply(&journal, &id, "anonymous", &pause).await;
    assert!(interrupted.is_err());
    let path = journal.join(&id);
    assert!(path.join("install/reply.bin").exists(), "{interrupted:?}");
    assert!(!path.join("receipt.json").exists());
    let signed = std::fs::read(path.join("install/signed.json")).unwrap();
    std::fs::remove_file(path.join("install/reply.bin")).unwrap();
    assert!(matches!(
        crate::apply(&journal, &id, "anonymous", &signer)
            .await
            .unwrap(),
        crate::ApplyOutcome::Complete(_)
    ));
    assert_eq!(
        std::fs::read(path.join("install/signed.json")).unwrap(),
        signed
    );
    assert!(
        serde_json::to_value(crate::status(&journal, &id).unwrap()).unwrap()["receipt"].is_object()
    );
    for side in ["custody-before", "custody-after"] {
        let reply = std::fs::read(path.join(side).join("reply.bin")).unwrap();
        assert_eq!(
            ic_wire::decode::<canic_management_protocol::application::Custody>(
                &reply,
                ic_wire::Encoding::Frame {
                    method: canic_management_protocol::application::CUSTODY,
                    direction: ic_wire::Direction::Response
                },
                canic_management_protocol::application::LIMITS
            )
            .unwrap()
            .root
            .current,
            input.expected
        );
    }
}
