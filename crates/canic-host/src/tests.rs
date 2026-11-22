use crate::{Error, Network, PlanInput, create_plan, plan, store};
use std::path::PathBuf;

pub(crate) fn directory() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("target/host-tests");
    std::fs::create_dir_all(&root).unwrap();
    root.join(
        crate::staging_directory::allocate(
            &crate::output_publication::prepare(&root, 0o700).unwrap(),
            "case",
        )
        .unwrap()
        .0,
    )
}

pub(crate) fn selected_fleet(
    app: &str,
    fleet: &str,
    network: Network,
    canisters: std::collections::BTreeMap<String, crate::CanisterConfig>,
) -> crate::SelectedFleet {
    crate::SelectedFleet {
        app: app.into(),
        fleet: fleet.into(),
        config: crate::FleetConfig {
            network,
            canisters,
            journal: None,
            receiver_policy: None,
            top_up_children: vec![],
            role_receivers: vec![],
            subnet_child_limit: None,
            registry: None,
            standby_pool: None,
            worker_pools: Vec::new(),
            shard_pools: Vec::new(),
            auth: Vec::new(),
        },
    }
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
fn network_binding_preserves_fleet_encoding_and_separates_trust_inputs() {
    let original = Network::Local {
        url: "http://localhost:8000".into(),
        root_key: "00".repeat(133),
        root_subnet: None,
        routing_canister: "aaaaa-aa".into(),
    };
    for network in [Network::Mainnet, original.clone()] {
        assert_eq!(
            network.binding_id().unwrap(),
            crate::artifact::digest(&serde_json::to_vec(&network).unwrap())
        );
    }
    for changed in [
        Network::Local {
            url: "http://localhost:8000".into(),
            root_key: "01".repeat(133),
            root_subnet: None,
            routing_canister: "aaaaa-aa".into(),
        },
        Network::Local {
            url: "http://localhost:8001".into(),
            root_key: "00".repeat(133),
            root_subnet: None,
            routing_canister: "aaaaa-aa".into(),
        },
        Network::Local {
            url: "http://localhost:8000".into(),
            root_key: "00".repeat(133),
            root_subnet: None,
            routing_canister: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
        },
    ] {
        assert_ne!(
            original.binding_id().unwrap(),
            changed.binding_id().unwrap()
        );
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
    let crate::PlanAction::Create { cycles } = &mut changed.action else {
        panic!("creation")
    };
    *cycles += 1;
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
        root_subnet: None,
        routing_canister: "aaaaa-aa".into(),
    };
    assert!(matches!(network.validate(), Err(Error::Invalid(_))));
}

#[test]
fn artifact_inspection_uses_store_capacity_without_expanding_direct_install_authority() {
    let root = directory();
    let wasm = root.join("large.wasm");
    let mut bytes = vec![42; crate::artifact::MAX_ARTIFACT_BYTES];
    bytes[..8].copy_from_slice(b"\0asm\x01\0\0\0");
    std::fs::write(&wasm, &bytes).unwrap();
    let artifact = crate::inspect(&wasm).unwrap();
    assert_eq!(artifact.bytes, bytes.len());
    assert_eq!(artifact.sha256, crate::artifact::digest(&bytes));
    assert!(create_plan(&root.join("journal"), &wasm, input()).is_err());
    assert!(!root.join("journal").exists());
    bytes.push(0);
    std::fs::write(&wasm, bytes).unwrap();
    assert!(crate::inspect(&wasm).is_err());
    std::fs::write(&wasm, b"not wasm").unwrap();
    assert!(crate::inspect(&wasm).is_err());
}

pub(crate) fn wire_fixture(name: &str) -> Vec<u8> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/platform-0.33.7");
    crate::unhex(
        std::fs::read_to_string(dir.join(format!("{name}.hex")))
            .unwrap()
            .trim(),
    )
    .unwrap()
}
