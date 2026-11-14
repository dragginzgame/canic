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
