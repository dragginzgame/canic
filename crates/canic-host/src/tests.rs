use crate::{Error, Network, PlanInput, create_plan, plan, store};
use std::path::PathBuf;

fn directory() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("target/host-tests")
        .join(format!("{}-{}", std::process::id(), crate::now().unwrap()));
    std::fs::create_dir_all(&root).unwrap();
    root
}

pub(crate) fn input() -> PlanInput {
    PlanInput {
        network: Network::Mainnet,
        controller: "o5trf-oqyg7-cawjp-xs4pw-aomb3-iwki5-hyezf-qahfz-j3ffd-jh4fc-oqe".into(),
        target: None,
        previous_module: None,
        creation_cycles: 1_000_000_000_000,
    }
}

#[test]
fn plan_identity_admission_and_artifact_custody() {
    let root = directory();
    let wasm = root.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let (id, plan) = create_plan(&root, &wasm, input()).unwrap();
    assert_eq!(plan::load(&root.join(&id), &id).unwrap().0, plan);
    let json = serde_json::to_string_pretty(&plan).unwrap();
    let reread: crate::Plan = serde_json::from_str(&json).unwrap();
    assert_eq!(reread.digest().unwrap(), id);
    let mut changed = plan;
    changed.creation_cycles += 1;
    assert_ne!(changed.digest().unwrap(), id);
    std::fs::write(root.join(&id).join("module.wasm"), b"\0asm\x01\0\0\0\0").unwrap();
    assert!(matches!(
        plan::load(&root.join(&id), &id),
        Err(Error::Invalid(_))
    ));
    let mut invalid = input();
    invalid.creation_cycles = 0;
    assert!(matches!(
        create_plan(&root, &wasm, invalid),
        Err(Error::Invalid(_))
    ));
    let mut anonymous = input();
    anonymous.controller = "2vxsx-fae".into();
    assert!(matches!(
        create_plan(&root, &wasm, anonymous),
        Err(Error::Invalid(_))
    ));
    let mut partial_upgrade = input();
    partial_upgrade.target = Some("aaaaa-aa".into());
    assert!(matches!(
        create_plan(&root, &wasm, partial_upgrade),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn retained_evidence_never_overwrites_and_store_excludes_concurrent_writer() {
    let root = directory();
    let first = store::Store::open(&root).unwrap();
    assert!(store::Store::open(&root).is_err());
    assert!(first.operation("../../escape").is_err());
    let file = root.join("signed.json");
    store::put(&file, b"original signed bytes").unwrap();
    assert!(store::put(&file, b"replacement").is_err());
    assert_eq!(store::read(&file, 100).unwrap(), b"original signed bytes");
    drop(first);
    assert!(store::Store::open(&root).is_ok());
}

#[test]
fn local_network_cannot_disable_mainnet_trust() {
    let network = Network::Local {
        url: "https://icp-api.io/".into(),
        root_key: "00".repeat(133),
        routing_canister: "aaaaa-aa".into(),
    };
    assert!(matches!(network.validate(), Err(Error::Invalid(_))));
}
