//! Qualification client and locally pinned receiver for the Root role issuer.
use super::{Principal, peer::controller};
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

#[canic::update(guard = controller)]
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
    canic::topology::attestation::admit_role::<Visit>(&proof).map_err(String::from)
}

#[canic::update(guard = controller)]
async fn present_role((receiver, copy): (Principal, Option<Proof>)) -> Result<(), String> {
    let proof = copy
        .or_else(|| PROOF.with_borrow(Clone::clone))
        .ok_or("no proof")?;
    call::<Use>(receiver, 5, &proof)
        .await
        .map_err(|_| "receiver unavailable")?
}

// Let the installed test replay a copied proof and alter its bytes.
#[canic::query(guard = controller)]
fn retained_role() -> Option<Proof> {
    PROOF.with_borrow(Clone::clone)
}

struct Visit;
impl Contract for Visit {
    const NAME: &'static str = "visit";
    const ID: u64 = wire::method_id(Self::NAME);
    type Input = ();
    type Output = Result<u64, String>;
}
impl RoleMethod for Visit {
    const ROLE: &'static str = "user_shard";
}

#[canic::update(role_contract = Visit)]
fn visit() -> Result<u64, String> {
    VISITS.update(|n| n + 1);
    Ok(VISITS.get())
}

#[canic::update(guard = controller)]
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

struct Visits;
impl Contract for Visits {
    const NAME: &'static str = "visits";
    const ID: u64 = wire::method_id(Self::NAME);
    type Input = ();
    type Output = u64;
}
impl RoleMethod for Visits {
    const ROLE: &'static str = "user_shard";
}

#[canic::query(role_contract = Visits)]
fn visits() -> u64 {
    VISITS.get()
}

#[canic::update(guard = controller)]
async fn read_visits(receiver: Principal) -> Result<u64, String> {
    let calls = CALLS.with(|s| s.get().expect("installed issuer").clone());
    calls
        .call_cached::<Visits>(receiver, 5, ())
        .await
        .map_err(|e| format!("{e:?}"))
}
