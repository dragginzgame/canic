use super::*;
use crate::{
    CanisterConfig, Network,
    tests::{directory, input},
};
use std::collections::BTreeMap;

fn selection() -> SelectedFleet {
    crate::tests::selected_fleet(
        "assets",
        "mainnet",
        Network::Mainnet,
        BTreeMap::from([
            (
                "root".into(),
                CanisterConfig {
                    id: Some("rrkah-fqaaa-aaaaa-aaaaq-cai".into()),
                    role: Some("root".into()),
                    ..Default::default()
                },
            ),
            (
                "store".into(),
                CanisterConfig {
                    role: Some("wasm-store".into()),
                    parent: Some("root".into()),
                    ..Default::default()
                },
            ),
            (
                "app".into(),
                CanisterConfig {
                    parent: Some("root".into()),
                    ..Default::default()
                },
            ),
        ]),
    )
}

#[test]
fn store_bootstrap_retains_distinct_authority_and_original_approval() {
    let dir = directory();
    let wasm = dir.join("store.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let journal = dir.join("journal");
    let config = selection();
    let (id, plan) =
        create_store_plan(&journal, &wasm, input(), &config, "store", &"11".repeat(32)).unwrap();
    assert!(plan.management().is_none());
    assert_eq!(plan.application_controller(), "rrkah-fqaaa-aaaaa-aaaaq-cai");
    assert_eq!(plan.creation_cycles(), input().creation_cycles);
    let decoded: Plan = serde_json::from_slice(&serde_json::to_vec(&plan).unwrap()).unwrap();
    assert_eq!(decoded, plan);
    assert_eq!(decoded.digest().unwrap(), id);
    let projection = crate::fleet::local_operation(&journal, &id, plan.created_at_ns)
        .unwrap()
        .0;
    assert_eq!(
        projection.store_root.as_deref(),
        Some(plan.application_controller())
    );
    assert!(projection.canister.is_none()); // Unknown creation never selects an arbitrary Store.
    assert_eq!(
        create_store_plan(&journal, &wasm, input(), &config, "store", &"11".repeat(32)).unwrap(),
        (id, plan.clone())
    );
    assert!(
        create_store_plan(&journal, &wasm, input(), &config, "store", &"22".repeat(32)).is_err()
    );
    // A direct install cannot reuse the Store approval just because its bytes/budget match.
    let mut direct = plan.clone();
    direct.action = PlanAction::Create {
        cycles: plan.creation_cycles(),
    };
    assert!(
        plan::retain_locked(
            &store::Store::open(&journal).unwrap(),
            direct,
            b"\0asm\x01\0\0\0"
        )
        .is_err()
    );
    let mut record = serde_json::to_value(&plan).unwrap();
    record["target"] = serde_json::json!("ryjl3-tyaaa-aaaaa-aaaba-cai");
    assert!(serde_json::from_value::<Plan>(record).is_err());
    // Store infrastructure does not consume the only application slot.
    let (_, child) = crate::create_managed_plan(
        &journal,
        &wasm,
        input(),
        &config,
        "app",
        &"11".repeat(32),
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert!(child.management().is_some());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn store_bootstrap_refuses_incomplete_topology_and_effect_authority() {
    let dir = directory();
    let wasm = dir.join("store.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    for case in 0..6 {
        let mut config = selection();
        let mut authority = input();
        match case {
            0 => config.config.canisters.get_mut("store").unwrap().role = Some("worker".into()),
            1 => {
                config.config.canisters.get_mut("store").unwrap().id =
                    Some("ryjl3-tyaaa-aaaaa-aaaba-cai".into())
            }
            2 => config.config.canisters.get_mut("root").unwrap().id = None,
            3 => authority.target = Some("ryjl3-tyaaa-aaaaa-aaaba-cai".into()),
            4 => authority.creation_cycles = 0,
            _ => config.config.canisters.get_mut("root").unwrap().id = Some("aaaaa-aa".into()),
        }
        let journal = dir.join(format!("case-{case}"));
        assert!(
            create_store_plan(
                &journal,
                &wasm,
                authority,
                &config,
                "store",
                &"11".repeat(32)
            )
            .is_err()
        );
        if journal.exists() {
            assert_eq!(std::fs::read_dir(journal).unwrap().count(), 1);
        }
    }
    std::fs::remove_dir_all(dir).unwrap();
}
