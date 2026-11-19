//! Persist the qualification signer's protected inputs alongside the supplier checkpoint.
use super::{Engine, STATE, config, initialize, renewal};
use ic_auth::issuer_memory::IssuerAuthority;
use ic_platform_runtime::system as api;
use ic_platform_types::Principal;
use ic_storage::{DefaultMemoryImpl, MemoryRuntime, StableKey, adapter::StableCell};
use std::cell::RefCell;

const HOST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 512,
    allocation: 16384,
    nodes: 256,
    depth: 32,
};
pub(crate) const KEY: &str = "auth.issuer.engine.v1";
pub(crate) const HOST_KEY: &str = "auth.issuer.host.v1";
thread_local! { static HOST: RefCell<Option<StableCell<Vec<u8>>>> = const { RefCell::new(None) }; }

// This fixture only uses the one predetermined Ed25519 test key. It has no
// external signing effects or certification-tree contributions.
pub(crate) fn open(runtime: &MemoryRuntime<DefaultMemoryImpl>, fresh: bool) {
    let lease = runtime
        .open_memory(&StableKey::parse(KEY).unwrap())
        .expect("issuer engine lease");
    let host = canic::storage::cell(runtime, HOST_KEY, HOST_LIMITS.bytes, fresh.then(Vec::new));
    let now = api::time();
    let engine = if fresh {
        Engine::new_fresh(lease, config(), now)
    } else {
        Engine::reopen(lease, config(), now)
    }
    .expect("open qualification issuer");
    initialize(engine);
    if !fresh {
        let (account, authority, allowed, signatures): (Principal, IssuerAuthority, bool, u32) =
            ic_wire::decode(host.get(), ic_wire::Encoding::Payload, HOST_LIMITS)
                .expect("issuer signer checkpoint");
        STATE.with_borrow_mut(|slot| {
            let s = slot.as_mut().unwrap();
            s.signer = super::SigningKey::from_bytes(&match authority.generation {
                1 => [7; 32],
                2 => [17; 32],
                _ => panic!("unknown test key generation"),
            });
            assert_eq!(
                authority.key_binding_id,
                super::Ed25519Key::new(s.signer.verifying_key().to_bytes())
                    .unwrap()
                    .binding()
            );
            s.account = account;
            s.authority = authority;
            s.allowed = allowed;
            s.signatures = signatures;
        });
    }
    HOST.with_borrow_mut(|slot| *slot = Some(host));
}

pub(crate) fn seal() {
    STATE.with_borrow_mut(|slot| {
        let s = slot.as_mut().expect("stable issuer must be open to seal");
        s.engine
            .seal()
            .unwrap_or_else(|err| api::trap(&format!("issuer checkpoint: {err:?}")));
        let bytes = ic_wire::encode(
            &(s.account, s.authority, s.allowed, s.signatures),
            ic_wire::Encoding::Payload,
            HOST_LIMITS,
        )
        .unwrap_or_else(|err| api::trap(&format!("issuer host record: {err:?}")));
        HOST.with_borrow_mut(|h| h.as_mut().unwrap().set(bytes));
    });
}

// Exercise delivery of a pre-upgrade callback using the original retained
// renewal ticket. The actual browser must own the renewal. The engine itself
// must refuse this mutation on its restart fence.
#[canic::update(limits = crate::CALLBACK_LIMITS)]
fn stale_callback(request: canic::auth::issuer::Renew) -> bool {
    STATE.with_borrow_mut(|slot| {
        let s = slot.as_mut().expect("open issuer");
        let request = renewal(request).unwrap();
        let ticket = s
            .engine
            .as_ref()
            .reconcile_renewal(api::caller(), &request)
            .unwrap()
            .expect("original operation")
            .intent
            .ticket;
        matches!(
            s.engine
                .state()
                .record_nonce(ticket, s.authority, [1; 16], api::time()),
            Err(ic_auth::issuer_memory::IssuerError::Denied)
        )
    })
}

#[canic::update]
fn reject_issuer_reply(reject: bool) {
    assert!(api::is_controller(&api::caller()));
    assert!(super::control(if reject { 6 } else { 7 }));
}
