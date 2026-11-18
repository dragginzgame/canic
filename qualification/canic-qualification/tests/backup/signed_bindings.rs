//! Present valid disposable signatures against changed Backup requests.
use canic_host::backup::DirectSigner;
use ic_backup::store::{
    BackupSigner, CapturePlan, OperationPlan, OperationSigningContent as Content,
    OperationSigningRequest as Request,
};
use ic_host_sign::{
    Error, Signer,
    protocol::{Configuration, PROTOCOL, Response, encode_response, now_ns, tool_hash},
};
use std::{fs, os::unix::fs::PermissionsExt, path::Path};

pub(super) async fn exercise(
    dir: &Path,
    config: Configuration,
    mut real: DirectSigner,
    plan: &CapturePlan,
    other_sender: &CapturePlan,
) {
    let helper = dir.join("reply");
    fs::write(
        &helper,
        "#!/bin/sh\ncat >/dev/null\nexec cat \"$0.response\"\n",
    )
    .unwrap();
    fs::set_permissions(&helper, fs::Permissions::from_mode(0o700)).unwrap();
    let mut replay =
        DirectSigner(Signer::new(&helper, tool_hash(&helper).unwrap(), config).unwrap());
    let commitment = replay.0.commitment();
    let response = |bytes| {
        fs::write(
            helper.with_extension("response"),
            encode_response(&Response {
                protocol: PROTOCOL.into(),
                commitment,
                envelope: bytes,
            })
            .unwrap(),
        )
        .unwrap()
    };
    let now = now_ns().unwrap();
    let expiry = (now / 60_000_000_000 + 2) * 60_000_000_000 + 731;
    let request = |signing| Request {
        plan: OperationPlan::Capture(plan),
        signing,
        deadline_ns: now + 15_000_000_000,
        expires_ns: expiry,
        envelope_bytes: 8192,
        network_requests: 0,
    };
    let call = || Content::Call {
        method: "canister_status",
        arguments: b"fixture",
    };
    let status = || Content::Observation {
        request_id: [8; 32],
    };
    let bytes = real.sign(request(call())).await.unwrap();
    // Bind the replay provider's outer frame correctly so refusal exercises the inner signature.
    response(bytes.clone());
    assert_eq!(replay.sign(request(call())).await.unwrap(), bytes);
    for changed in [
        request(Content::Call {
            method: "list_canister_snapshots",
            arguments: b"fixture",
        }),
        request(Content::Call {
            method: "canister_status",
            arguments: b"other",
        }),
        Request {
            expires_ns: expiry + 1,
            ..request(call())
        },
        Request {
            envelope_bytes: 100,
            ..request(call())
        },
    ] {
        assert!(matches!(replay.sign(changed).await, Err(Error::Output)));
    }
    assert!(matches!(
        real.sign(Request {
            plan: OperationPlan::Capture(other_sender),
            ..request(call())
        })
        .await,
        Err(Error::Request)
    ));
    assert!(matches!(
        real.sign(Request {
            network_requests: 1,
            ..request(call())
        })
        .await,
        Err(Error::Request)
    ));
    response(real.sign(request(status())).await.unwrap());
    assert!(replay.sign(request(status())).await.is_ok());
    assert!(matches!(
        replay
            .sign(request(Content::Observation {
                request_id: [9; 32]
            }))
            .await,
        Err(Error::Output)
    ));
}
