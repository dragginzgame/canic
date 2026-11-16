use super::*;
use crate::{Network, tests::directory};

fn source() -> (std::path::PathBuf, SelectedFleet, String, Plan, Vec<u8>) {
    let dir = directory();
    let bytes = b"\0asm\x01\0\0\0".to_vec();
    let wasm = dir.join("app.wasm");
    std::fs::write(&wasm, &bytes).unwrap();
    let mut selection = super::super::tests::selection();
    selection.config.network = Network::Local {
        url: "http://127.0.0.1:8000/".into(),
        root_key: "00".repeat(133),
        routing_canister: "aaaaa-aa".into(),
    };
    let (id, plan) = super::super::create_publication_plan(
        &dir,
        &wasm,
        &selection,
        "2vxsx-fae",
        super::super::tests::publication("v1"),
    )
    .unwrap();
    (dir, selection, id, plan, bytes)
}
fn retain_reply(dir: &Path, plan: &Plan, bytes: &[u8], deadline: u64, reply: &wire::RemovalReply) {
    let path = dir.join(plan.digest().unwrap()).join("removal");
    std::fs::create_dir_all(&path).unwrap();
    let call = call(plan, bytes, deadline).unwrap();
    std::fs::write(
        path.join("dispatch-deadline.json"),
        serde_json::to_vec(&deadline).unwrap(),
    )
    .unwrap();
    std::fs::write(
        path.join("signed.json"),
        crate::signed::tests::signed_fixture(plan, &call, plan.expires_at_ns),
    )
    .unwrap();
    std::fs::write(
        path.join("reply.bin"),
        ic_wire::encode(
            reply,
            ic_wire::Encoding::Frame {
                method: wire::REMOVE,
                direction: ic_wire::Direction::Response,
            },
            wire::RESPONSE_LIMITS,
        )
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn removal_retains_one_authority_and_fences_publication_before_submission() {
    let (dir, selection, source, publication, _) = source();
    let (id, plan) = create_removal_plan(&dir, &source, &selection, wire::Removal::Delete).unwrap();
    let before = std::fs::read(dir.join(&id).join("plan.json")).unwrap();
    assert_eq!(
        create_removal_plan(&dir, &source, &selection, wire::Removal::Delete).unwrap(),
        (id.clone(), plan.clone())
    );
    assert!(create_removal_plan(&dir, &source, &selection, wire::Removal::Cancel).is_err());
    assert!(permit_publication(&dir, &publication).is_err());
    assert!(create_restoration_plan(&dir, &id, &selection).is_err());
    let mut other = selection;
    other.fleet = "other".into();
    assert!(create_restoration_plan(&dir, &id, &other).is_err());
    for field in ["target", "publication", "store_upgrade_root"] {
        let mut encoded = serde_json::to_value(&plan).unwrap();
        encoded[field] = serde_json::to_value(publication.publication()).unwrap();
        assert!(serde_json::from_value::<Plan>(encoded).is_err());
    }
    assert_eq!(serde_json::from_slice::<Plan>(&before).unwrap(), plan);
    assert_eq!(
        std::fs::read(dir.join(id).join("plan.json")).unwrap(),
        before
    );
}

#[test]
fn restoration_requires_exact_settled_reply_and_expired_dispatch_then_new_authority() {
    let (dir, selection, source, original, bytes) = source();
    let (id, removal) =
        create_removal_plan(&dir, &source, &selection, wire::Removal::Delete).unwrap();
    let deadline = crate::now().unwrap() + 60_000_000_000;
    retain_reply(
        &dir,
        &removal,
        &bytes,
        deadline,
        &Ok(expected(&removal, &bytes).unwrap()),
    );
    assert!(create_restoration_plan(&dir, &id, &selection).is_err());
    // A separately signed historical fixture avoids scheduling assumptions here.
    // Installed qualification waits out the same original signed request unchanged.
    let deadline = crate::now().unwrap() - 1;
    retain_reply(
        &dir,
        &removal,
        &bytes,
        deadline,
        &Ok(expected(&removal, &bytes).unwrap()),
    );
    let path = dir.join(&id).join("removal");
    let valid = std::fs::read(path.join("reply.bin")).unwrap();
    for reply in [
        Err(wire::Error::Uncertain),
        Err(wire::Error::Store(ic_wasm_store_protocol::Error::NotFound)),
        Ok(super::super::release_manifest(
            ic_wasm_store_protocol::Release {
                name: "other".into(),
                version: "v1".into(),
            },
            &bytes,
        )),
    ] {
        retain_reply(&dir, &removal, &bytes, deadline, &reply);
        assert!(create_restoration_plan(&dir, &id, &selection).is_err());
    }
    std::fs::remove_file(path.join("reply.bin")).unwrap();
    assert!(create_restoration_plan(&dir, &id, &selection).is_err());
    std::fs::write(path.join("reply.bin"), valid).unwrap();
    let (restored, plan) = create_restoration_plan(&dir, &id, &selection).unwrap();
    assert_ne!(restored, source);
    assert_eq!(
        plan.publication().unwrap().after_removal.as_deref(),
        Some(id.as_str())
    );
    assert_eq!(
        create_restoration_plan(&dir, &id, &selection).unwrap().0,
        restored
    );
    assert!(permit_publication(&dir, &original).is_err());
    assert!(permit_publication(&dir, &plan).is_ok());
    let (second, second_plan) =
        create_removal_plan(&dir, &restored, &selection, wire::Removal::Cancel).unwrap();
    assert!(permit_publication(&dir, &plan).is_err());
    assert!(create_restoration_plan(&dir, &id, &selection).is_err());
    retain_reply(
        &dir,
        &second_plan,
        &bytes,
        deadline,
        &Ok(expected(&second_plan, &bytes).unwrap()),
    );
    let (_, successor) = create_restoration_plan(&dir, &second, &selection).unwrap();
    assert!(permit_publication(&dir, &successor).is_ok());
    assert_eq!(
        publication_predecessors(&dir, &successor).unwrap(),
        [restored, source]
    );
}

#[test]
fn publication_observation_does_not_settle_an_ambiguous_mutation() {
    let (dir, selection, id, plan, bytes) = source();
    let path = dir.join(&id).join("publication/prepare");
    std::fs::create_dir_all(&path).unwrap();
    let deadline = crate::now().unwrap() - 1;
    store::put_json(&path.join("dispatch-deadline.json"), &deadline).unwrap();
    let expected = super::super::manifest(&plan, &bytes).unwrap();
    let call =
        super::super::call(&plan, wire::Command::Prepare(expected.clone()), deadline).unwrap();
    store::put(
        &path.join("signed.json"),
        &crate::signed::tests::signed_fixture(&plan, &call, plan.expires_at_ns),
    )
    .unwrap();
    assert!(create_removal_plan(&dir, &id, &selection, wire::Removal::Delete).is_err());
    for reply in [
        Err(wire::Error::Uncertain),
        Ok(ic_wasm_store_protocol::Status {
            manifest: expected,
            published: false,
            missing: vec![0],
        }),
    ] {
        std::fs::write(
            path.join("reply.bin"),
            ic_wire::encode(
                &reply,
                ic_wire::Encoding::Frame {
                    method: wire::EXECUTE,
                    direction: ic_wire::Direction::Response,
                },
                wire::RESPONSE_LIMITS,
            )
            .unwrap(),
        )
        .unwrap();
        let outcome = create_removal_plan(&dir, &id, &selection, wire::Removal::Delete);
        assert_eq!(outcome.is_ok(), reply.is_ok());
    }
}
