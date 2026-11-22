//! Qualification client and locally pinned receiver for the Root role issuer.
use super::Principal;
use canic::{
    method::{Contract, call, wire},
    topology::attestation::{Issue, Proof, RoleCalls, RoleMethod, RoleVerifier},
};
use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::Rc,
};

pub(super) const KEY: &str = "hub.rolepin.v1";
thread_local! {
    static CALLS: OnceCell<Rc<RoleCalls>> = const { OnceCell::new() };
    static VISITS: Cell<u64> = const {Cell::new(0)};
    static PROOF: RefCell<Option<Proof>> = const { RefCell::new(None) };
}

pub(super) fn open(
    runtime: &ic_storage::MemoryRuntime<ic_storage::DefaultMemoryImpl>,
    initial: Option<(Principal, Vec<u8>)>,
) {
    let cell = canic::storage::cell(
        runtime,
        KEY,
        128,
        initial.map(|(root, key)| {
            wire::encode(
                &(root, wire::Bytes::<32>::new(key).expect("key size")),
                wire::Encoding::Payload,
                canic::method::LIMITS,
            )
            .unwrap()
        }),
    );
    let (root, pin): (Principal, wire::Bytes<32>) =
        wire::decode(cell.get(), wire::Encoding::Payload, canic::method::LIMITS)
            .expect("retained issuer binding");
    CALLS.with(|slot| assert!(slot.set(canic::topology::attach_role_calls(root)).is_ok()));
    if !pin.as_slice().is_empty() {
        let verifier = RoleVerifier::new(root, pin.as_slice()).expect("valid issuer pin");
        verifier.attach().unwrap();
    }
}

#[canic::update(guard = canic::guard::require_controller)]
async fn renew_role((issuer, receiver): (Principal, Principal)) -> Result<u64, String> {
    let proof = call::<Issue>(issuer, 5, &receiver)
        .await
        .map_err(|_| "issuer unavailable")??;
    let expiry = proof.0.expiry()?;
    PROOF.set(Some(proof));
    Ok(expiry)
}

struct Use;
impl Contract for Use {
    const NAME: &'static str = "use_role";
    const ID: u64 = wire::method_id(Self::NAME);
    type Input = Proof;
    type Output = Result<(), String>;
}

#[canic::update(contract = Use)]
fn use_role(proof: Proof) -> Result<(), String> {
    canic::guard::require_role::<Visit>(&proof).map_err(String::from)
}

#[canic::update(guard = canic::guard::require_controller)]
async fn present_role((receiver, copy): (Principal, Option<Proof>)) -> Result<(), String> {
    let proof = copy
        .or_else(|| PROOF.with_borrow(Clone::clone))
        .ok_or("no proof")?;
    call::<Use>(receiver, 5, &proof)
        .await
        .map_err(|_| "receiver unavailable")?
}

// Let the installed test replay a copied proof and alter its bytes.
#[canic::query(guard = canic::guard::require_controller)]
fn retained_role() -> Option<Proof> {
    PROOF.with_borrow(Clone::clone)
}

const VISIT_REQUEST: wire::Limits = wire::Limits {
    bytes: 384,
    ..canic::method::LIMITS
};
const VISIT_REPLY: wire::Limits = wire::Limits {
    bytes: 64,
    ..canic::method::LIMITS
};
struct Visit;
impl Contract for Visit {
    const NAME: &'static str = "visit";
    const ID: u64 = wire::method_id(Self::NAME);
    const REQUEST: wire::Limits = VISIT_REQUEST;
    const REPLY: wire::Limits = VISIT_REPLY;
    type Input = ();
    type Output = Result<u64, String>;
}
impl RoleMethod for Visit {
    const ROLES: &'static [&'static str] = &["user_shard", "hub"];
}

const HUB: &str = "hub";

#[canic::update(roles = &["user_shard", HUB], limits = VISIT_REQUEST, reply_limits = VISIT_REPLY)]
fn visit() -> Result<u64, String> {
    VISITS.update(|n| n + 1);
    Ok(VISITS.get())
}

#[canic::update(guard = canic::guard::require_controller)]
async fn visit_hub() -> Result<u64, String> {
    let caller = ic_platform_runtime::system::caller();
    canic::topology::call_service::<Visit>("accounts_hub", 5, (), || {
        ic_platform_runtime::system::is_controller(&caller)
            .then_some(())
            .ok_or_else(|| "controller changed".into())
    })
    .await
    .map_err(|e| format!("{e:?}"))?
}

mod reader {
    use super::VISITS;
    use canic::query;
    include!("visit_count.rs");
}

mod protocol {
    use canic::attested_method as query;
    include!("visit_count.rs");
}

#[canic::update(guard = canic::guard::require_controller)]
async fn read_visits(receiver: Principal) -> Result<u64, String> {
    let calls = CALLS.with(|s| s.get().expect("installed issuer").clone());
    calls
        .call_cached::<protocol::visits>(receiver, 5, ())
        .await
        .map_err(|e| format!("{e:?}"))
}

#[canic::update(roles = &[])]
fn closed_visit() {}
