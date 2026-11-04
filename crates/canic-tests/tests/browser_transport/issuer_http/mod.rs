//! Browser account ingress uses Rust-derived owned Wire contracts and original intent.
use super::*;
use canic::dto::portable_issuance::{
    IssuePortableCredential, IssuedPortableCredential, PortableCredentialIssuanceRefusal,
    PortableCredentialIssuanceStatus, ReconcilePortableCredential,
};
use ic_wire::typescript::{Wire, generate_registry, method_limits, native_schema};
use std::collections::BTreeMap;

const ISSUE: u64 = 0x5445_5354_0000_080c;
const RECONCILE: u64 = 0x5445_5354_0000_080d;

fn method<Request: Wire, Response: Wire>(name: &str, id: u64) -> serde_json::Value {
    let mut value = method_limits(
        canic::protocol::portable_issuance::REQUEST_LIMITS,
        canic::protocol::portable_issuance::RESPONSE_LIMITS,
    )
    .unwrap();
    value["name"] = name.into();
    value["id"] = id.to_string().into();
    value["mode"] = "update".into();
    value["request"] = native_schema::<Request>().unwrap();
    value["response"] = native_schema::<Response>().unwrap();
    value
}

#[test]
#[ignore = "requires prepared frontend npm dependencies and an explicit Chromium executable"]
fn browser_reconciles_issuance_before_admission() {
    let mut fixture = canic_tests::pic::setup_http_active_component_registry();
    let host = fixture.fresh_pic_mut().unwrap().make_live(None).unwrap();
    enroll_live(&fixture);
    let targets = [fixture.issuer.canister_id, fixture.verifier.canister_id];
    let devices = [65, 66];
    for target in targets {
        await_missing_session(fixture.pic(), target, 65, Duration::from_secs(30));
    }
    let requests = devices.map(|device| portable_issue_request(&fixture, device));
    let lookup = requests
        .each_ref()
        .map(|request| ReconcilePortableCredential {
            epoch: request.epoch,
            operation_id: request.operation_id,
        });
    let methods = [
        method::<
            IssuePortableCredential,
            Result<IssuedPortableCredential, PortableCredentialIssuanceRefusal>,
        >("test_issue_portable_credential", ISSUE),
        method::<
            ReconcilePortableCredential,
            Result<PortableCredentialIssuanceStatus, PortableCredentialIssuanceRefusal>,
        >("test_reconcile_portable_credential", RECONCILE),
    ];
    let codecs = generate_registry(
        &BTreeMap::new(),
        &methods,
        "canic-wire-runtime",
        &BTreeMap::new(),
        |_| {},
    )
    .unwrap();
    let input = serde_json::json!({
        "mode": "issuer", "host": host.as_str(), "rootKey": fixture.pic().root_key().unwrap().unwrap(),
        "issuer": fixture.issuer.canister_id.to_text(), "targets": targets.map(|target| target.to_text()),
        "devices": devices, "account": 70, "codecs": codecs, "runtime": ic_wire::typescript::RUNTIME,
        "requests": requests.iter().map(|request| ic_wire::native::encode_request(ISSUE, request, canic::protocol::portable_issuance::REQUEST_LIMITS).unwrap()).collect::<Vec<_>>(),
        "lookups": lookup.iter().map(|request| ic_wire::native::encode_request(RECONCILE, request, canic::protocol::portable_issuance::REQUEST_LIMITS).unwrap()).collect::<Vec<_>>(),
        "scopes": ["app00:xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
    });
    run_script("auth-browser-runner.mjs", &input);
    for target in targets {
        assert_eq!(
            portable_read(fixture.pic(), target, portable_browser(65)),
            Err(ic_auth_wire::model::SessionReadRefusal::MissingOrExpired)
        );
        let peer = portable_read(fixture.pic(), target, portable_browser(66)).unwrap();
        assert_eq!(peer.subject, portable_browser(70).into());
        assert_ne!(peer.subject, peer.caller);
    }
    fixture.fresh_pic_mut().unwrap().stop_live().unwrap();
    drop(fixture);
}
