use super::*;
use crate::{
    PlanInput, create_plan,
    tests::{directory, input},
};

#[tokio::test(flavor = "current_thread")]
async fn partial_inventory_keeps_uncertain_work_without_exporting_attempts() {
    let root = directory();
    let wasm = root.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let (id, _) = create_plan(&root, &wasm, input()).unwrap();
    std::fs::create_dir(root.join(&id).join("create")).unwrap();
    let signed = root.join(&id).join("create/signed.json");
    std::fs::write(&signed, "PRIVATE_SIGNED_ATTEMPT_SENTINEL").unwrap();
    std::fs::create_dir(root.join("f".repeat(64))).unwrap();
    let snapshot = fleet(&root).await.unwrap();
    assert!(snapshot.canisters.is_empty());
    assert_eq!(snapshot.operations.len(), 1);
    assert_eq!(snapshot.issues.len(), 1);
    assert_eq!(
        snapshot.operations[0].effects["create"],
        "signed_attempt_retained_outcome_unknown"
    );
    assert!(snapshot.operations[0].canister.is_none());
    assert!(
        !serde_json::to_string(&snapshot)
            .unwrap()
            .contains("PRIVATE_SIGNED_ATTEMPT_SENTINEL")
    );
    assert_eq!(
        std::fs::read_to_string(signed).unwrap(),
        "PRIVATE_SIGNED_ATTEMPT_SENTINEL"
    );
    let forged = Receipt {
        operation: id.clone(),
        canister: "aaaaa-aa".into(),
        module_hash: snapshot.operations[0].approved_module.clone(),
        controller: snapshot.operations[0].approved_controller.clone(),
    };
    store::put_json(&root.join(&id).join("receipt.json"), &forged).unwrap();
    let snapshot = fleet(&root).await.unwrap();
    assert!(snapshot.operations.is_empty());
    assert_eq!(snapshot.issues.len(), 2);
}

#[test]
fn replicas_with_equal_principals_keep_separate_trust_bindings() {
    let root = directory();
    let wasm = root.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let mut projections = Vec::new();
    for byte in ["aa", "bb"] {
        let (id, plan) = create_plan(
            &root,
            &wasm,
            PlanInput {
                network: Network::Local {
                    url: "http://127.0.0.1:8080/".into(),
                    root_key: byte.repeat(133),
                    routing_canister: "aaaaa-aa".into(),
                },
                target: Some("aaaaa-aa".into()),
                previous_module: Some("00".repeat(32)),
                creation_cycles: 0,
                ..input()
            },
        )
        .unwrap();
        projections.push(
            local_operation(&root, &id, plan.expires_at_ns + 1)
                .unwrap()
                .0,
        );
    }
    assert_eq!(projections[0].canister, projections[1].canister);
    assert_ne!(projections[0].network_id, projections[1].network_id);
    assert!(projections.iter().all(|item| item.approval_expired));
}

fn root_operation(network: Network) -> FleetOperation {
    let directory = directory();
    let wasm = directory.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let (id, _) = crate::plan::retain(
        &directory,
        &wasm,
        PlanInput { network, ..input() },
        Some(crate::DeploymentName {
            app: "starter".into(),
            fleet: "demo".into(),
            canister: "app".into(),
        }),
        Some(crate::ManagedCreation {
            root: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
            root_module: "01".repeat(32),
            execute_at_ns: crate::now().unwrap() + 600_000_000_000,
        }),
    )
    .unwrap();
    let operation = local_operation(&directory, &id, crate::now().unwrap())
        .unwrap()
        .0;
    std::fs::remove_dir_all(directory).unwrap();
    operation
}

#[test]
fn automatic_root_observations_keep_exact_networks_and_deduplicate_explicit_selection() {
    let first = Network::Mainnet;
    let second = Network::Local {
        url: "http://localhost:8000/".into(),
        root_key: "aa".repeat(133),
        routing_canister: "aaaaa-aa".into(),
    };
    let operations = [
        root_operation(first.clone()),
        root_operation(second.clone()),
    ];
    let root = operations[0].management.as_ref().unwrap().manager.clone();
    let targets = BTreeMap::from([
        ((operations[0].network_id.clone(), root.clone()), first),
        ((operations[1].network_id.clone(), root.clone()), second),
    ]);
    let explicit = std::collections::BTreeSet::from([root]);
    assert_eq!(
        select_managers(&operations, &targets, &Default::default()).unwrap(),
        targets
    );
    assert!(matches!(
        select_managers(&operations, &targets, &explicit),
        Err(Error::Invalid(_))
    ));
    let one = BTreeMap::from([targets
        .first_key_value()
        .map(|(key, value)| (key.clone(), value.clone()))
        .unwrap()]);
    let matching: Vec<_> = operations
        .into_iter()
        .filter(|operation| {
            one.keys()
                .any(|(network, _)| *network == operation.network_id)
        })
        .collect();
    assert_eq!(select_managers(&matching, &one, &explicit).unwrap(), one);
}

#[test]
fn root_reports_cannot_substitute_another_approval_or_a_self_child() {
    let operation = root_operation(Network::Mainnet);
    let network = operation.network_id.clone();
    let approval = operation.management.clone().unwrap();
    let root = approval.manager.clone();
    let operations = [operation];
    let before = serde_json::to_value(&operations).unwrap();
    let mut report = canic_management_protocol::Status::default();
    assert!(check_report(&operations, &network, &root, &report).is_ok());
    report.phase = canic_management_protocol::Phase::Scheduled;
    assert!(check_report(&operations, &network, &root, &report).is_err());
    report.approval = Some(approval.clone());
    assert!(check_report(&operations, &network, &root, &report).is_ok());
    report.approval.as_mut().unwrap().module_hash = [9; 32];
    assert!(check_report(&operations, &network, &root, &report).is_err());
    report.approval = Some(approval);
    report.worker = Some(root.clone());
    assert!(check_report(&operations, &network, &root, &report).is_err());
    report.worker = Some("ryjl3-tyaaa-aaaaa-aaaba-cai".into());
    assert!(check_report(&operations, &network, &root, &report).is_ok());
    assert_eq!(serde_json::to_value(&operations).unwrap(), before);
    assert!(operations[0].canister.is_none());
    assert!(!operations[0].receipt_retained);
}

#[test]
fn manager_observation_budget_includes_every_network_bound_owner() {
    let targets: BTreeMap<_, _> = (1..=17)
        .map(|value| {
            (
                (
                    Network::Mainnet.binding_id().unwrap(),
                    ic_platform::principal::Principal::from_slice(&[value]).to_text(),
                ),
                Network::Mainnet,
            )
        })
        .collect();
    let explicit = targets
        .keys()
        .map(|(_, principal)| principal.clone())
        .collect();
    assert!(matches!(
        select_managers(&[], &targets, &explicit),
        Err(Error::Invalid(_))
    ));
}
