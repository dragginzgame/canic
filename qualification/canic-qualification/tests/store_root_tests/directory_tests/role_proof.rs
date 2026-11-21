//! Real Root ledger, signing key, placed caller and install-time verifier pin.

use super::{CanisterId, Client, artifacts};
use canic::{
    method::Contract,
    sharding::Placement,
    topology::{
        attestation::Proof,
        directory::ServiceInstall,
        dormancy::{DormancyReceipt, Receipt},
    },
};
use std::time::Duration;

pub(super) async fn prepare(root: &Client<'_>) -> Vec<u8> {
    root.update::<_, Result<ic_wire::Bytes<32>, String>>(
        "prepare_role_key",
        ic_wire::method_id("prepare_role_key"),
        &(),
        canic::method::LIMITS,
    )
    .await
    .unwrap()
    .into_vec()
}

pub(super) async fn exercise(
    issuer: &Client<'_>,
    hub: CanisterId,
    project: CanisterId,
    pin: Vec<u8>,
    service_generation: [u8; 32],
) {
    let pic = issuer.pic;
    let owner = issuer.operator;
    assert_eq!(prepare(issuer).await, pin);
    let receiver = Client {
        pic,
        id: hub,
        operator: issuer.id,
    };
    let placed: Placement = receiver
        .update(
            "place",
            ic_wire::method_id("place"),
            &(false, 1u8),
            canic::sharding::LIMITS,
        )
        .await;
    let Placement::Assigned(ref allocation, ref pid) = placed else {
        panic!("ready shared shard")
    };
    let child = Client {
        pic,
        id: CanisterId::from_text(pid).unwrap(),
        operator: issuer.id,
    };
    let limits = canic::method::LIMITS;
    let id = ic_wire::method_id;
    let renew = async |client: &Client<'_>| {
        client
            .update::<_, Result<u64, String>>(
                "renew_role",
                id("renew_role"),
                &(issuer.id, receiver.id),
                limits,
            )
            .await
    };
    let present = async || {
        child
            .update::<_, Result<(), String>>(
                "present_role",
                id("present_role"),
                &(receiver.id, None::<Proof>),
                limits,
            )
            .await
    };
    let frame = |direction| ic_wire::Encoding::Frame {
        method: id("retained_role"),
        direction,
    };
    let retained = async |subject| {
        let reply = pic
            .query_call(
                subject,
                issuer.id,
                "retained_role",
                ic_wire::encode(&(), frame(ic_wire::Direction::Request), limits).unwrap(),
            )
            .await
            .unwrap();

        ic_wire::decode::<Option<Proof>>(&reply, frame(ic_wire::Direction::Response), limits)
            .unwrap()
            .unwrap()
    };
    let registry = Client {
        pic,
        id: project,
        operator: issuer.id,
    };
    let dedicated: Placement = registry
        .update("place", id("place"), &(true, 1u8), canic::sharding::LIMITS)
        .await;
    let Placement::Assigned(original, instance) = dedicated else {
        panic!("ready dedicated project")
    };
    let instance = Client {
        pic,
        id: CanisterId::from_text(instance).unwrap(),
        operator: issuer.id,
    };
    let public =
        ic_platform_crypto::ed25519::VerifyingKey::from_bytes(pin.as_slice().try_into().unwrap())
            .unwrap();
    for (caller, role, generation) in [
        (&registry, "hub", service_generation),
        (&instance, "project", original.generation),
    ] {
        let expiry = renew(caller).await.unwrap();
        let (claim, signature) = retained(caller.id).await;
        assert_eq!(claim.subject.as_slice(), caller.id.as_slice());
        assert_eq!(claim.root.as_slice(), issuer.id.as_slice());
        assert_eq!(claim.receiver.as_slice(), receiver.id.as_slice());
        assert_eq!(claim.role, role);
        assert_eq!(claim.generation.as_slice(), generation);
        assert_eq!(
            expiry - claim.issued_at_ns,
            canic::topology::attestation::LIFETIME_NS
        );
        public
            .verify_strict(
                &claim.signing_bytes().unwrap(),
                &ic_platform_crypto::ed25519::Signature::from_slice(signature.as_slice()).unwrap(),
            )
            .unwrap();
        let closed: Result<u64, String> = issuer
            .update(
                "close_role_issuance",
                id("close_role_issuance"),
                &(caller.id, claim.generation),
                limits,
            )
            .await;
        assert_eq!(closed, Err("shared shard required".into()));
        // The sender reopens with an empty volatile cache and renews for the same generation.
        canic_qualification::upgrade_stopped(
            pic,
            caller.id,
            std::fs::read(artifacts().join("directory.wasm")).unwrap(),
            vec![],
            Some(issuer.id),
        )
        .await
        .unwrap();
        pic.advance_time(Duration::from_secs(1)).await;
        assert!(renew(caller).await.unwrap() > expiry);
        let renewed = retained(caller.id).await.0;
        assert_eq!(renewed.generation.as_slice(), generation);
        assert_eq!(renewed.role, role);
    }
    let exports = canic_host::endpoints(&artifacts().join("directory.wasm")).unwrap();
    assert_eq!(
        exports
            .iter()
            .filter(|e| e.name == "visit")
            .map(|e| e.mode)
            .collect::<Vec<_>>(),
        ["update"]
    );
    let visit = async |sender: &Client<'_>| {
        sender
            .update::<_, Result<u64, String>>("visit_hub", id("visit_hub"), &(), limits)
            .await
    };
    let bare = ic_wire::encode(
        &(),
        ic_wire::Encoding::Frame {
            method: id("visit"),
            direction: ic_wire::Direction::Request,
        },
        limits,
    )
    .unwrap();
    assert!(
        pic.update_call(receiver.id, registry.id, "visit", bare)
            .await
            .is_err()
    );
    assert_eq!(visit(&registry).await, Ok(1));
    // Project has a valid receiver-bound proof, but the typed client refuses dispatch.
    assert_eq!(
        visit(&instance).await.unwrap_err(),
        r#"Proof("issuer returned a different or expired claim")"#
    );
    let project_proof = retained(instance.id).await;
    let direct = async |method: &str, sender, proof: Proof| {
        pic.update_call(
            receiver.id,
            sender,
            method,
            ic_wire::encode(
                &(proof, ()),
                ic_wire::Encoding::Frame {
                    method: id(method),
                    direction: ic_wire::Direction::Request,
                },
                limits,
            )
            .unwrap(),
        )
        .await
    };
    let refusal = direct("visit", instance.id, project_proof)
        .await
        .unwrap_err();
    assert!(refusal.to_string().contains("role refused"), "{refusal}");
    let hub_proof = retained(registry.id).await;
    let refusal = direct("closed_visit", registry.id, hub_proof)
        .await
        .unwrap_err();
    assert!(refusal.to_string().contains("role refused"), "{refusal}");
    let unrelated = pic.create_canister().await;
    let denied: Result<Proof, String> = Client {
        pic,
        id: issuer.id,
        operator: unrelated,
    }
    .update("issue_role", id("issue_role"), &receiver.id, limits)
    .await;
    assert_eq!(denied.err().as_deref(), Some("unassigned caller"));
    child
        .update::<_, Result<u64, String>>(
            "renew_role",
            id("renew_role"),
            &(issuer.id, project),
            limits,
        )
        .await
        .unwrap();
    assert_eq!(present().await, Err("role refused".into()));
    let read_visits = async || {
        child
            .update::<_, Result<u64, String>>(
                "read_visits",
                id("read_visits"),
                &receiver.id,
                limits,
            )
            .await
    };
    assert!(read_visits().await.is_err());
    assert_eq!(visit(&child).await, Ok(2));
    let expiry = renew(&child).await.unwrap();
    canic_qualification::upgrade_stopped(
        pic,
        receiver.id,
        std::fs::read(artifacts().join("directory.wasm")).unwrap(),
        vec![],
        Some(issuer.id),
    )
    .await
    .unwrap();
    pic.stop_canister(issuer.id, Some(owner)).await.unwrap();
    assert_eq!(visit(&child).await, Ok(1)); // receiver upgrade cleared its volatile visit count
    assert_eq!(present().await, Ok(()));
    assert_eq!(read_visits().await, Ok(1));
    let proof: Proof = retained(child.id).await;
    let refusal = direct("visit", project, proof.clone()).await.unwrap_err();
    assert!(refusal.to_string().contains("role refused"), "{refusal}");
    assert_eq!(read_visits().await, Ok(1));
    assert_eq!(proof.0.expiry().unwrap(), expiry);
    assert_eq!(proof.0.generation.as_slice(), allocation.generation);
    let submit = async |operator, proof: &Proof| {
        Client {
            pic,
            id: receiver.id,
            operator,
        }
        .update::<_, Result<(), String>>("use_role", id("use_role"), proof, limits)
        .await
    };
    assert_eq!(submit(owner, &proof).await, Err("role refused".into()));
    let mut forged = proof;
    forged.0.issued_at_ns -= 1;
    assert_eq!(submit(child.id, &forged).await, Err("signature".into()));
    pic.advance_time(Duration::from_secs(7200)).await;
    pic.tick().await;
    assert_eq!(present().await, Err("role refused".into()));
    assert!(read_visits().await.is_err());
    assert!(visit(&child).await.is_err());
    pic.start_canister(issuer.id, Some(owner)).await.unwrap();
    assert_eq!(visit(&child).await, Ok(2));
    assert_eq!(read_visits().await, Ok(2));
    assert!(renew(&child).await.unwrap() > expiry);
    assert_eq!(present().await, Ok(()));
    // Keep a signed copy to exercise out-of-band IC controller reuse below.
    let copy = retained(child.id).await;
    let close = async |generation| {
        issuer
            .update::<_, Result<u64, String>>(
                "close_role_issuance",
                id("close_role_issuance"),
                &(child.id, generation),
                limits,
            )
            .await
    };
    assert_eq!(
        close(ic_wire::Bytes::<32>::new(vec![0; 32]).unwrap()).await,
        Err("assignment changed".into())
    );
    let request = |method, input: &(CanisterId, ic_wire::Bytes<32>)| {
        ic_wire::encode(
            input,
            ic_wire::Encoding::Frame {
                method,
                direction: ic_wire::Direction::Request,
            },
            limits,
        )
        .unwrap()
    };
    let closure = request(
        id("close_role_issuance"),
        &(child.id, copy.0.generation.clone()),
    );
    assert!(
        pic.update_call(issuer.id, project, "close_role_issuance", closure)
            .await
            .is_err()
    );
    assert!(renew(&child).await.is_ok());
    let custody = ic_wire::encode(
        &(),
        ic_wire::Encoding::Frame {
            method: canic_management_protocol::application::CUSTODY,
            direction: ic_wire::Direction::Request,
        },
        limits,
    )
    .unwrap();
    assert!(
        pic.update_call(issuer.id, owner, "observe_application_children", custody)
            .await
            .is_err()
    );
    let target = canic::sharding::ShardTarget {
        allocation: allocation.clone(),
        shard: copy.0.subject,
    };
    let codec = artifacts().join("role-root.client");
    let command = async |args: Vec<String>| {
        let request = canic_host::application::codec::encode(&codec, &args).unwrap();
        let bytes = pic
            .update_call(issuer.id, owner, &request.method, request.arguments)
            .await
            .unwrap();

        canic_host::application::codec::decode(&codec, &bytes).unwrap()
    };
    let inspect = async || command(vec!["inspect-shard".into(), child.id.to_text()]).await;
    let observed = inspect().await;
    assert_eq!(observed["draining"], false);
    assert_eq!(observed["live_placements"], 2);
    assert_eq!(observed["shard"], child.id.to_text());
    let drain_request = vec![
        "drain-shard".into(),
        child.id.to_text(),
        observed["slot"].to_string(),
        observed["generation"].as_str().unwrap().into(),
    ];
    let drain = async || command(drain_request.clone()).await;
    let release = async |key: u8, target: canic::sharding::ShardTarget| {
        receiver
            .update::<_, Result<(), String>>(
                "release_route",
                id("release_route"),
                &([key; 32], target),
                canic::sharding::LIMITS,
            )
            .await
    };
    assert_eq!(
        release(1u8, target.clone()).await,
        Err("shard is not draining".into())
    );
    assert_eq!(drain().await["live_placements"], 2);
    assert_eq!(inspect().await["draining"], true);
    assert_eq!(
        close(copy.0.generation.clone()).await,
        Err("empty draining shard required".into())
    );
    let place = async |key| {
        receiver
            .update::<_, Placement>("place", id("place"), &(false, key), canic::sharding::LIMITS)
            .await
    };
    assert_eq!(place(3u8).await, Placement::Full); // one free seat is fenced
    assert_eq!(place(1u8).await, placed);
    let retirement_status = async || {
        let bytes = pic
            .query_call(
                issuer.id,
                owner,
                "retirement_status",
                request(
                    id("retirement_status"),
                    &(child.id, copy.0.generation.clone()),
                ),
            )
            .await
            .unwrap();
        ic_wire::decode::<Result<(u64, u32), String>>(
            &bytes,
            ic_wire::Encoding::Frame {
                method: id("retirement_status"),
                direction: ic_wire::Direction::Response,
            },
            limits,
        )
        .unwrap()
    };
    assert_eq!(
        retirement_status().await,
        Err("role issuance remains open".into())
    );
    pic.advance_time(Duration::from_secs(7201)).await;
    pic.tick().await;
    assert!(renew(&child).await.is_ok());
    let mut stale = target.clone();
    stale.allocation.generation[0] ^= 1;
    assert!(release(1u8, stale).await.is_err());
    let raw = canic::sharding::ReleaseShard::encode_request(&([1; 32], target.clone())).unwrap();
    assert!(
        pic.update_call(issuer.id, owner, "release_shard", raw)
            .await
            .is_err()
    );
    assert_eq!(release(1u8, target.clone()).await, Ok(()));
    assert_eq!(release(1u8, target.clone()).await, Ok(()));
    assert_eq!(inspect().await["live_placements"], 1);
    assert_eq!(place(1u8).await, Placement::Full);
    assert_eq!(place(2u8).await, placed);
    canic_qualification::upgrade_stopped(
        pic,
        issuer.id,
        std::fs::read(artifacts().join("role-root.wasm")).unwrap(),
        vec![],
        Some(owner),
    )
    .await
    .unwrap();
    assert_eq!(prepare(issuer).await, pin);
    assert_eq!(inspect().await["live_placements"], 1);
    assert!(renew(&child).await.is_ok());
    let copy = retained(child.id).await;
    assert_eq!(place(2u8).await, placed);
    assert_eq!(release(2u8, target.clone()).await, Ok(()));
    assert_eq!(inspect().await["live_placements"], 0);
    let close_request = vec![
        "close-role-issuance".into(),
        child.id.to_text(),
        observed["generation"].as_str().unwrap().into(),
    ];
    let closed = command(close_request.clone()).await;
    let deadline: u64 = closed["proof_deadline_ns"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(deadline >= copy.0.expiry().unwrap());
    canic_qualification::upgrade_stopped(
        pic,
        issuer.id,
        std::fs::read(artifacts().join("role-root.wasm")).unwrap(),
        vec![],
        Some(owner),
    )
    .await
    .unwrap();
    assert_eq!(command(close_request).await, closed);
    assert_eq!(renew(&child).await, Err("role issuance closed".into()));
    assert_eq!(retirement_status().await, Ok((deadline, 0)));
    assert_eq!(place(1u8).await, Placement::Full);
    assert_eq!(place(3u8).await, Placement::Full);
    // Exercise the shipped dormant actor independently of Root's effect orchestration.
    let wasm = std::fs::read(artifacts().join("dormant.wasm")).unwrap();
    let binding = Receipt {
        root: issuer.id.to_text().parse().unwrap(),
        subject: copy.0.subject,
        generation: ic_wire::ByteArray::new(allocation.generation),
        proof_deadline_ns: deadline,
    };
    for wrong in [
        Receipt {
            root: project.to_text().parse().unwrap(),
            ..binding.clone()
        },
        Receipt {
            subject: project.to_text().parse().unwrap(),
            ..binding.clone()
        },
    ] {
        assert!(
            pic.reinstall_canister(
                child.id,
                wasm.clone(),
                wrong.encode().unwrap(),
                Some(issuer.id)
            )
            .await
            .is_err()
        );
    }
    pic.reinstall_canister(
        child.id,
        wasm.clone(),
        binding.encode().unwrap(),
        Some(issuer.id),
    )
    .await
    .unwrap();
    let observe = async || {
        child
            .update::<_, Receipt>(DormancyReceipt::NAME, DormancyReceipt::ID, &(), limits)
            .await
    };
    assert_eq!(observe().await, binding);
    // Neither a dormant receipt nor restarting its Root releases this principal.
    let standby = async || {
        use canic::sharding::standby::InspectStandby;
        issuer
            .update::<_, <InspectStandby as Contract>::Output>(
                InspectStandby::NAME,
                InspectStandby::ID,
                &(),
                InspectStandby::REPLY,
            )
            .await
    };
    assert!(standby().await.is_empty());
    canic_qualification::upgrade_stopped(
        pic,
        issuer.id,
        std::fs::read(artifacts().join("role-root.wasm")).unwrap(),
        vec![],
        Some(owner),
    )
    .await
    .unwrap();
    assert_eq!(retirement_status().await, Ok((deadline, 0)));
    assert!(pic.get_time().await.as_nanos_since_unix_epoch() < deadline);
    assert!(standby().await.is_empty());
    use canic::sharding::standby::ResetWorker;
    let dispatch_until = pic.get_time().await.as_nanos_since_unix_epoch() + 60_000_000_000;
    let worker_reset: Result<_, String> = issuer
        .update::<_, <ResetWorker as Contract>::Output>(
            ResetWorker::NAME,
            ResetWorker::ID,
            &(target.clone(), dispatch_until),
            ResetWorker::REQUEST,
        )
        .await;
    assert_eq!(
        worker_reset.unwrap_err(),
        "released worker allocation required"
    );
    canic_qualification::upgrade_stopped(pic, child.id, wasm, vec![], Some(issuer.id))
        .await
        .unwrap();
    assert_eq!(observe().await, binding);
    assert!(
        pic.update_call(child.id, owner, "renew_role", vec![])
            .await
            .is_err()
    );

    // The simulator controller performs the subsequent handover to exercise proof expiry.
    let owner = pic.create_canister().await;
    pic.set_controllers(child.id, Some(issuer.id), vec![owner])
        .await
        .unwrap();
    let install = ServiceInstall {
        name: "replacement".into(),
        role: "worker".into(),
        application: vec![],
    };
    pic.reinstall_canister(
        child.id,
        std::fs::read(artifacts().join("directory.wasm")).unwrap(),
        install.encode().unwrap(),
        Some(owner),
    )
    .await
    .unwrap();
    let replacement = Client {
        pic,
        id: child.id,
        operator: owner,
    };
    assert_eq!(
        renew(&replacement).await,
        Err("role issuance closed".into())
    );
    let present_copy = async |copy: Option<Proof>| {
        replacement
            .update::<_, Result<(), String>>(
                "present_role",
                id("present_role"),
                &(receiver.id, copy),
                limits,
            )
            .await
    };
    assert_eq!(present_copy(None).await, Err("no proof".into()));
    assert_eq!(present_copy(Some(copy.clone())).await, Ok(()));
    pic.advance_time(Duration::from_secs(7200)).await;
    pic.tick().await;
    assert_eq!(present_copy(Some(copy)).await, Err("role refused".into()));
}
