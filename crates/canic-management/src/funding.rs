//! Opt-in ongoing payments between an application parent and its direct children.
pub(crate) mod receipts;
use canic::{
    method::{self, Contract},
    topology::directory::retained::DIRECTORY,
};
use canic_management_protocol::stored::FundingError;
use canic_management_protocol::stored::{ChildFunding, ChildFundingStatus};
use ic_platform_runtime::{raw, system};
use ic_platform_types::Principal;
use ic_storage::{DefaultMemoryImpl, MemoryRuntime, adapter::StableBTreeMap};
use receipts::History;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::BTreeMap};

/// Report whether the forwarded funding call could have reached its recipient.
#[must_use]
pub fn call_failure(error: &method::CallError) -> FundingError {
    use method::CallError;
    match error {
        CallError::Request(_) | CallError::Dispatch(_) => FundingError::CallNotSubmitted,
        CallError::Rejected(_) | CallError::Unknown(_) | CallError::Reply(_) => {
            FundingError::ReplyUnconfirmed
        }
    }
}

const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 32_768,
    allocation: 262_144,
    nodes: 16_384,
    depth: 16,
};
const CHECK_NS: u64 = 60_000_000_000;

#[derive(Serialize, Deserialize)]
struct Grant {
    rule: ChildFunding,
    generation: [u8; 32],
    status: ChildFundingStatus,
    paid: u64,
}

#[derive(Default)]
struct Book {
    history: History,
    operator: Option<Principal>,
    children: BTreeMap<Principal, Grant>,
    incoming: Option<Incoming>,
}

#[derive(Serialize, Deserialize)]
struct Incoming {
    parent: Principal,
    threshold: u64,
    deadline: u64,
    paid: u64,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
enum Key {
    Operator,
    Incoming,
    History,
    Child(Principal),
}
type Rows = StableBTreeMap<Vec<u8>, Vec<u8>>;
thread_local! { static STATE: RefCell<Option<(Book,Rows)>> = const { RefCell::new(None) }; }
fn bytes(value: &impl Serialize) -> Vec<u8> {
    ic_wire::encode(value, ic_wire::Encoding::Payload, LIMITS).expect("funding record")
}
fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> T {
    ic_wire::decode(bytes, ic_wire::Encoding::Payload, LIMITS).expect("retained funding record")
}
fn put(rows: &mut Rows, key: Key, value: &impl Serialize) {
    rows.insert(bytes(&key), bytes(value));
}
fn change<T>(f: impl FnOnce(&mut Book, &mut Rows) -> T) -> T {
    STATE.with_borrow_mut(|s| {
        let (book, rows) = s.as_mut().expect("open funding ledger");
        f(book, rows)
    })
}
fn read<T>(f: impl FnOnce(&Book) -> T) -> T {
    STATE.with_borrow(|s| f(&s.as_ref().expect("open funding ledger").0))
}

/// Install or reopen the funding participant in the application's selected storage lease.
/// # Panics
/// Requires the original lease, format and lifecycle selection.
pub fn open(
    runtime: &MemoryRuntime<DefaultMemoryImpl>,
    key: &str,
    install: Option<Option<Principal>>,
) {
    let mut rows: Rows = canic::storage::map(runtime, key, install.is_some());
    if let Some(operator) = install {
        put(&mut rows, Key::Operator, &operator);
    }
    let operator = decode(
        &rows
            .get(&bytes(&Key::Operator))
            .expect("funding operator record"),
    );
    let mut book = Book {
        operator,
        ..Book::default()
    };
    for entry in rows.iter() {
        match decode(entry.key()) {
            Key::Operator => (),
            Key::Incoming => book.incoming = Some(decode(&entry.value())),
            Key::History => book.history = decode(&entry.value()),
            Key::Child(child) => {
                book.children.insert(child, decode(&entry.value()));
            }
        }
    }
    book.history
        .validate(|child| book.children.get(&child).map(|grant| grant.paid));
    for (&child, grant) in &book.children {
        let rule = &grant.rule;
        let status = &grant.status;
        assert!(
            child == rule.child
                && rule.parent == system::canister_self()
                && rule.amount_cycles > 0
                && rule.below_cycles > 0
                && status.charged_cycles <= rule.budget_cycles
                && status.attempts.checked_mul(rule.amount_cycles) == Some(status.charged_cycles)
                && status
                    .attempts
                    .checked_sub(grant.paid)
                    .is_some_and(|pending| pending <= 1),
            "retained child funding custody"
        );
    }
    let deadline = book.incoming.as_ref().map(|i| i.deadline);
    STATE.set(Some((book, rows)));
    if let Some(deadline) = deadline {
        schedule(deadline);
    }
}

fn direct(child: Principal) -> Result<[u8; 32], FundingError> {
    DIRECTORY
        .with_borrow(|d| {
            d.as_ref()?
                .child_delivery(child)
                .and_then(|(hop, ack)| (ack.is_some()).then_some(hop.generation))
        })
        .ok_or(FundingError::UnknownChild)
}

/// Configure the child-side balance trigger from the retained application parent.
pub struct Watch;
impl Contract for Watch {
    const NAME: &'static str = "watch_child_balance";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    type Input = (u64, u64);
    type Output = Result<(), FundingError>;
}

/// Request the next fixed payment. The receiver uses the actual caller as payee.
pub struct Request;
impl Contract for Request {
    const NAME: &'static str = "request_child_funding";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    type Input = (u64, u128);
    type Output = Result<u64, FundingError>;
}

/// Admit an immutable funding rule and deliver the threshold to its child.
/// # Errors
/// Requires the parent's installed funding operator, an acknowledged child and finite bounds.
pub async fn configure(rule: ChildFunding) -> Result<ChildFundingStatus, FundingError> {
    if !read(|book| book.operator == Some(system::caller())) {
        return Err(FundingError::Unauthorized);
    }
    let generation = direct(rule.child)?;
    if rule.parent != system::canister_self()
        || rule.component.is_empty()
        || rule.component.len() > 64
        || rule.below_cycles == 0
        || rule.amount_cycles == 0
        || rule.amount_cycles > rule.budget_cycles
    {
        return Err(FundingError::InvalidApproval);
    }
    let child = rule.child;
    let terms = (rule.below_cycles, rule.expires_at_ns);
    let status = change(|b, rows| {
        if let Some(g) = b.children.get(&child) {
            return if g.rule == rule && g.generation == generation {
                Ok(g.status.clone())
            } else {
                Err(FundingError::ApprovalConflict)
            };
        }
        if system::time() >= rule.expires_at_ns {
            return Err(FundingError::Expired);
        }
        b.children.insert(
            child,
            Grant {
                rule,
                generation,
                status: ChildFundingStatus::default(),
                paid: 0,
            },
        );
        put(rows, Key::Child(child), &b.children[&child]);
        Ok(ChildFundingStatus::default())
    })?;
    if !status.closed && system::time() < terms.1 {
        method::call::<Watch>(child, 10, &terms)
            .await
            .map_err(|error| call_failure(&error))??;
    }
    Ok(read(|b| b.children[&child].status.clone()))
}

/// Revoke future payments and preserve any issued debit.
/// # Errors
/// Requires the parent's installed funding operator and an admitted child.
pub fn stop(child: Principal) -> Result<ChildFundingStatus, FundingError> {
    if !read(|book| book.operator == Some(system::caller())) {
        return Err(FundingError::Unauthorized);
    }
    change(|b, rows| {
        let g = b
            .children
            .get_mut(&child)
            .ok_or(FundingError::UnknownChild)?;
        g.status.closed = true;
        put(rows, Key::Child(child), g);
        Ok(g.status.clone())
    })
}

/// Install the threshold under the actual directory parent.
/// # Errors
/// Refuses another caller or changes to installed terms.
pub fn watch((threshold, deadline): (u64, u64)) -> Result<(), FundingError> {
    let parent = system::caller();
    if !DIRECTORY.with_borrow(|d| d.as_ref().is_some_and(|d| d.publisher() == parent)) {
        return Err(FundingError::Unauthorized);
    }
    if deadline <= system::time() {
        return Err(FundingError::Expired);
    }
    if threshold == 0 {
        return Err(FundingError::InvalidApproval);
    }
    let inserted = change(|b, rows| {
        if let Some(i) = &b.incoming {
            return if (i.parent, i.threshold, i.deadline) == (parent, threshold, deadline) {
                Ok(false)
            } else {
                Err(FundingError::ApprovalConflict)
            };
        }
        let incoming = Incoming {
            parent,
            threshold,
            deadline,
            paid: 0,
        };
        put(rows, Key::Incoming, &incoming);
        b.incoming = Some(incoming);
        Ok(true)
    })?;
    if inserted {
        schedule(deadline);
    }
    Ok(())
}

/// Authenticate the child and dispatch each approved sequence once.
/// # Errors
/// A refused or uncertain deposit retains its original exposure.
/// # Panics
/// Traps if the original grant disappears during a deposit.
pub async fn request((sequence, balance): (u64, u128)) -> Result<u64, FundingError> {
    let child = system::caller();
    let amount = change(|b, rows| {
        let g = b
            .children
            .get_mut(&child)
            .ok_or(FundingError::UnknownChild)?;
        if direct(child)? != g.generation {
            return Err(FundingError::GenerationMismatch);
        }
        if sequence == 0 {
            return Err(FundingError::InvalidSequence);
        }
        if sequence <= g.paid {
            return Ok(None);
        }
        let s = &mut g.status;
        if s.issued {
            return Err(FundingError::PaymentPending);
        }
        if s.closed {
            return Err(FundingError::Closed);
        }
        if g.paid.checked_add(1) != Some(sequence) {
            return Err(FundingError::InvalidSequence);
        }
        if system::time() >= g.rule.expires_at_ns {
            return Err(FundingError::Expired);
        }
        if balance >= u128::from(g.rule.below_cycles) {
            return Err(FundingError::AboveThreshold);
        }
        if g.rule.amount_cycles > g.rule.budget_cycles - s.charged_cycles {
            return Err(FundingError::BudgetExhausted);
        }
        if raw::liquid_cycle_balance() < u128::from(g.rule.amount_cycles) {
            return Err(FundingError::InsufficientCycles);
        }
        s.attempts += 1;
        s.charged_cycles += g.rule.amount_cycles;
        s.issued = true;
        s.refund_cycles = None;
        put(rows, Key::Child(child), g);
        Ok(Some(g.rule.amount_cycles))
    })?;
    if let Some(amount) = amount {
        let (success, refund) = crate::engine::deposit_cycles(child, amount).await;
        change(|b, rows| {
            let g = b.children.get_mut(&child).expect("issued child funding");
            g.status.issued = !success && refund != Some(u128::from(amount));
            g.status.closed |= !success;
            g.status.refund_cycles = refund;
            if success {
                g.paid = sequence;
                b.history.record(child, sequence, system::time());
                put(rows, Key::History, &b.history);
            }
            put(rows, Key::Child(child), g);
        });
        if !success {
            return Err(if refund == Some(u128::from(amount)) {
                FundingError::DepositRefunded
            } else {
                FundingError::PaymentPending
            });
        }
    }
    Ok(sequence)
}

fn schedule(deadline: u64) {
    use ic_timers::{Lifetime, Observation, OnceDemand, OnceNext, OnceResult};
    if system::time() >= deadline {
        return;
    }
    ic_timers::bind_once(
        ["canic", "child-funding", "request"],
        Lifetime::RemoveWhenStopped,
        move || async move {
            let observation = match check().await {
                Ok(true) => Observation::Success,
                Ok(false) => Observation::NoWork,
                Err(_) => Observation::ExpectedFailure,
            };
            let now = system::time();
            OnceResult {
                observation,
                next: if now >= deadline {
                    OnceNext::Stop
                } else {
                    OnceNext::At(now.saturating_add(CHECK_NS).min(deadline))
                },
            }
        },
    )
    .expect("child balance timer")
    .set(OnceDemand::At(
        system::time().saturating_add(CHECK_NS).min(deadline),
    ))
    .expect("schedule child balance timer");
}

/// Check this child's own balance and resume its pending request, if any.
/// # Errors
/// A missing acknowledgement retains the same sequence for the next low-balance check.
/// # Panics
/// Traps if the retained trigger disappears during a payment.
pub async fn check() -> Result<bool, FundingError> {
    // The next sequence follows from the durable acknowledgement. Even after a
    // lost reply, a new balance observation cannot authorize a second debit.
    let request = read(|b| {
        let i = b.incoming.as_ref()?;
        let balance = raw::liquid_cycle_balance();
        if system::time() >= i.deadline || balance >= u128::from(i.threshold) {
            return None;
        }
        Some((i.parent, i.paid.checked_add(1)?, balance))
    });
    let Some((parent, seq, balance)) = request else {
        return Ok(false);
    };
    let ack = method::call::<Request>(parent, 10, &(seq, balance))
        .await
        .map_err(|error| call_failure(&error))??;
    if ack != seq {
        return Err(FundingError::InvalidSequence);
    }
    change(|b, rows| {
        let i = b.incoming.as_mut().expect("original trigger");
        i.paid = i.paid.max(seq);
        put(rows, Key::Incoming, i);
    });
    Ok(true)
}

/// Read a child's settled operating grants from this application's own funding ledger.
/// # Panics
/// Requires the payer's funding lease to be open.
#[must_use]
pub fn receipts(child: Principal) -> Option<canic::funding::FundingReceipts> {
    read(|book| {
        let grant = book.children.get(&child)?;
        Some(book.history.report(&grant.rule, &grant.status, grant.paid))
    })
}
