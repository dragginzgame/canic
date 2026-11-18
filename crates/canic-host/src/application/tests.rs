use super::*;
use crate::{CanisterBinding, Network};

fn setup() -> (std::path::PathBuf, SelectedFleet) {
    let dir = crate::tests::directory();
    let config = dir.join("canic.toml");
    crate::init_app(&config, "demo").unwrap();
    crate::add_app_canister(&config, "demo", "app", crate::CanisterDefinition::default()).unwrap();
    crate::init_fleet(&config, "demo", "mainnet", Network::Mainnet).unwrap();
    crate::bind_canister(
        &config,
        "demo",
        "mainnet",
        "app",
        CanisterBinding {
            id: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let selected = crate::load_config(&config)
        .unwrap()
        .select(None, None)
        .unwrap();
    (dir, selected)
}

#[test]
fn request_authority_roundtrips_and_uncertainty_blocks_replacement() {
    let (dir, fleet) = setup();
    let journal = fleet.journal_path(&dir.join("canic.toml"));
    let request = || Request {
        method: "counter".into(),
        arguments: vec![1, 2, 3],
    };
    let caller = "ryjl3-tyaaa-aaaaa-aaaba-cai";
    let (id, plan) = prepare(
        &journal,
        &fleet,
        "app",
        caller.into(),
        vec!["increment".into()],
        request(),
    )
    .unwrap();
    assert_eq!(plan.creation_cycles(), 0);
    assert!(plan.target().is_none());
    assert_eq!(
        plan.application_call().unwrap().canister,
        "rrkah-fqaaa-aaaaa-aaaaq-cai"
    );
    assert_eq!(
        plan::load(&journal.join(&id), &id).unwrap(),
        (plan.clone(), vec![1, 2, 3])
    );
    assert!(!journal.join(&id).join("module.wasm").exists());
    fleet.check_operation(&journal, &id).unwrap();
    assert!(
        crate::fleet::inventory(&journal, false, crate::now().unwrap())
            .unwrap()
            .records
            .is_empty()
    );
    assert!(matches!(
        prepare(
            &journal,
            &fleet,
            "app",
            caller.into(),
            vec!["increment".into()],
            request()
        ),
        Err(Error::Blocked(_))
    ));
    // A changed command, expiry or missing signed file cannot erase uncertainty.
    assert!(matches!(
        prepare(
            &journal,
            &fleet,
            "app",
            caller.into(),
            vec!["read".into()],
            request()
        ),
        Err(Error::Blocked(_))
    ));
    let mut conflicting = serde_json::to_value(&plan).unwrap();
    conflicting["creation_cycles"] = 1.into();
    assert!(serde_json::from_value::<Plan>(conflicting).is_err());
    std::fs::write(journal.join(&id).join("request.bin"), [9]).unwrap();
    assert!(plan::load(&journal.join(&id), &id).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn target_and_bounds_fail_before_retaining_authority() {
    let (dir, fleet) = setup();
    let journal = fleet.journal_path(&dir.join("canic.toml"));
    assert!(target(&journal, &fleet, "missing").is_err());
    for request in [
        Request {
            method: "".into(),
            arguments: vec![],
        },
        Request {
            method: "counter".into(),
            arguments: vec![0; MAX_REQUEST_BYTES + 1],
        },
    ] {
        assert!(
            prepare(
                &journal,
                &fleet,
                "app",
                "ryjl3-tyaaa-aaaaa-aaaba-cai".into(),
                vec!["read".into()],
                request
            )
            .is_err()
        );
    }
    assert!(!journal.exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn expired_call_never_signs_again_and_retained_reply_never_reexecutes() {
    let (dir, fleet) = setup();
    let journal = fleet.journal_path(&dir.join("canic.toml"));
    let (id, plan) = prepare(
        &journal,
        &fleet,
        "app",
        crate::signed::tests::controller(),
        vec!["increment".into()],
        Request {
            method: "counter".into(),
            arguments: vec![1],
        },
    )
    .unwrap();
    let path = journal.join(&id).join("call");
    std::fs::create_dir(&path).unwrap();
    let bytes = crate::signed::tests::signed_fixture(
        &plan,
        &call(&plan, &[1]).unwrap(),
        crate::now().unwrap() - 1,
    );
    std::fs::write(path.join("signed.json"), &bytes).unwrap();
    let absent = dir.join("no-signer");
    assert!(matches!(
        crate::reconcile(&journal, &id, "unused", &absent).await,
        Err(Error::Blocked(_))
    ));
    assert_eq!(std::fs::read(path.join("signed.json")).unwrap(), bytes);
    assert!(!path.join("reply.bin").exists());
    // A retained reply is terminal even if its app codec would refuse it.
    std::fs::write(path.join("reply.bin"), [255]).unwrap();
    let crate::ApplyOutcome::Call { call } = crate::apply(&journal, &id, "unused", &absent)
        .await
        .unwrap()
    else {
        panic!("call reply");
    };
    assert_eq!(call.bytes, [255]);
    assert_eq!(std::fs::read(path.join("signed.json")).unwrap(), bytes);
    let mut forged = plan.clone();
    forged.controller = "rrkah-fqaaa-aaaaa-aaaaq-cai".into();
    assert!(receipt(&journal.join(&id), &forged, &[1]).is_err());
    let (next, _) = prepare(
        &journal,
        &fleet,
        "app",
        crate::signed::tests::controller(),
        vec!["increment".into()],
        Request {
            method: "counter".into(),
            arguments: vec![1],
        },
    )
    .unwrap();
    assert_ne!(
        next, id,
        "a new explicit invocation follows a retained terminal reply"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn codec_refuses_claimed_callers_and_bad_output_and_publishes_executable_mode() {
    let dir = crate::tests::directory();
    for (index, output) in [
        r#"{"method":"counter","arguments":[1],"caller":"forged"}"#,
        r#"{"method":"counter","arguments":[256]}"#,
        "not JSON",
    ]
    .into_iter()
    .enumerate()
    {
        let executable = dir.join(format!("codec-{index}"));
        crate::output_publication::publish(
            &executable,
            format!("#!/bin/sh\nprintf '%s' '{output}'\n").as_bytes(),
            ic_host_fs::Destination::CreateNew,
            0o700,
            0o700,
        )
        .unwrap();
        assert!(codec::encode(&executable, &["read".into()]).is_err());
    }
    let executable = dir.join("echo-reply");
    crate::output_publication::publish(
        &executable,
        b"#!/bin/sh\ncat\n",
        ic_host_fs::Destination::CreateNew,
        0o700,
        0o700,
    )
    .unwrap();
    assert_eq!(
        codec::decode(&executable, br#"{"value":7}"#).unwrap(),
        serde_json::json!({"value":7})
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn resumed_call_issues_read_state_only() {
    use std::io::{Read as _, Write as _};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let (dir, mut fleet) = setup();
    fleet.config.network = Network::Local {
        url: format!("http://{}/", listener.local_addr().unwrap()),
        root_key: crate::hex(include_bytes!("../network/mainnet-root.der")),
        root_subnet: Some(crate::network::MAINNET_ROOT_SUBNET.into()),
        routing_canister: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
    };
    let journal = fleet.journal_path(&dir.join("canic.toml"));
    let (id, plan) = prepare(
        &journal,
        &fleet,
        "app",
        crate::signed::tests::controller(),
        vec!["increment".into()],
        Request {
            method: "counter".into(),
            arguments: vec![1],
        },
    )
    .unwrap();
    let path = journal.join(&id).join("call");
    std::fs::create_dir(&path).unwrap();
    let signed = crate::signed::tests::signed_fixture(
        &plan,
        &call(&plan, &[1]).unwrap(),
        crate::now().unwrap() + 300_000_000_000,
    );
    std::fs::write(path.join("signed.json"), &signed).unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut bytes = [0; 8192];
        let count = stream.read(&mut bytes).unwrap();
        let request = std::str::from_utf8(bytes[..count].split(|b| *b == b'\n').next().unwrap())
            .unwrap()
            .to_owned();
        stream
            .write_all(b"HTTP/1.1 500 Test\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .unwrap();
        request
    });
    assert!(
        crate::apply(&journal, &id, "unused", &dir.join("absent-signer"))
            .await
            .is_err()
    );
    let request = server.join().unwrap();
    assert!(request.starts_with("POST /api/"), "{request}");
    assert!(request.contains("/read_state "), "{request}");
    assert_eq!(std::fs::read(path.join("signed.json")).unwrap(), signed);
    assert!(!path.join("reply.bin").exists());
    std::fs::remove_dir_all(dir).unwrap();
}
