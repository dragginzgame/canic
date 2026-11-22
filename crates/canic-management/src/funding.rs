//! Opt-in ongoing payments between an application parent and its direct children.
use canic::{
    method::{self, Contract},
    topology::directory::retained::DIRECTORY,
};
use canic_management_protocol::stored::{ChildFunding, ChildFundingStatus};
use ic_platform_runtime::{raw, system};
use ic_platform_types::Principal;
use ic_storage::{DefaultMemoryImpl, MemoryRuntime, adapter::StableCell};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::BTreeMap};

const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 32_768,
    allocation: 262_144,
    nodes: 16_384,
    depth: 16,
};
const CHECK_NS: u64 = 60_000_000_000;
const BOOK: ic_wire::Encoding = ic_wire::Encoding::Frame {
    method: ic_wire::method_id("canic.application_child_funding.v1"),
    direction: ic_wire::Direction::Response,
};

#[derive(Serialize, Deserialize)]
struct Grant {
    rule: ChildFunding,
    generation: [u8; 32],
    status: ChildFundingStatus,
    paid: u64,
}

#[derive(Default, Serialize, Deserialize)]
struct Book {
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

type Cell = StableCell<Vec<u8>, DefaultMemoryImpl>;
thread_local! { static CELL: RefCell<Option<Cell>> = const {RefCell::new(None)}; }

fn encode(book: &Book) -> Vec<u8> {
    ic_wire::encode(book, BOOK, LIMITS).expect("bounded funding ledger")
}
fn load(cell: &Cell) -> Book {
    ic_wire::decode(cell.get(), BOOK, LIMITS).expect("retained funding ledger")
}
fn change<T>(f: impl FnOnce(&mut Book) -> T) -> T {
    CELL.with_borrow_mut(|c| {
        let cell = c.as_mut().expect("open funding ledger");
        let mut book = load(cell);
        let result = f(&mut book);
        let bytes = encode(&book);
        if cell.get() != &bytes {
            cell.set(bytes);
        }
        result
    })
}
fn read<T>(f: impl FnOnce(&Book) -> T) -> T {
    CELL.with_borrow(|c| f(&load(c.as_ref().expect("open funding ledger"))))
}

/// Install or reopen the funding participant in the application's selected storage lease.
/// # Panics
/// Requires the original lease, format and lifecycle selection.
pub fn open(runtime: &MemoryRuntime<DefaultMemoryImpl>, key: &str, fresh: bool) {
    let cell = canic::storage::cell(
        runtime,
        key,
        LIMITS.bytes,
        fresh.then(|| encode(&Book::default())),
    );
    let book = load(&cell);
    for (child, grant) in book.children {
        let rule = grant.rule;
        let status = grant.status;
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
    let deadline = book.incoming.map(|i| i.deadline);
    CELL.set(Some(cell));
    if let Some(deadline) = deadline {
        schedule(deadline);
    }
}

fn direct(child: Principal) -> Result<[u8; 32], String> {
    DIRECTORY
        .with_borrow(|d| {
            d.as_ref()?
                .child_delivery(child)
                .and_then(|(hop, ack)| (ack.is_some()).then_some(hop.generation))
        })
        .ok_or_else(|| "acknowledged direct child required".into())
}

/// Configure the child-side balance trigger from the retained application parent.
pub struct Watch;
impl Contract for Watch {
    const NAME: &'static str = "watch_child_balance";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    type Input = (u64, u64);
    type Output = Result<(), String>;
}

/// Request the next fixed payment. The receiver uses the actual caller as payee.
pub struct Request;
impl Contract for Request {
    const NAME: &'static str = "request_child_funding";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    type Input = (u64, u128);
    type Output = Result<u64, String>;
}

/// Admit an immutable funding rule and deliver the threshold to its child.
/// # Errors
/// Requires the parent's controller, an acknowledged child and finite bounds.
pub async fn configure(rule: ChildFunding) -> Result<ChildFundingStatus, String> {
    controller()?;
    let generation = direct(rule.child)?;
    if rule.parent != system::canister_self()
        || rule.component.is_empty()
        || rule.component.len() > 64
        || rule.below_cycles == 0
        || rule.amount_cycles == 0
        || rule.amount_cycles > rule.budget_cycles
    {
        return Err("invalid direct funding approval".into());
    }
    let child = rule.child;
    let terms = (rule.below_cycles, rule.expires_at_ns);
    let status = change(|b| {
        if let Some(g) = b.children.get(&child) {
            return if g.rule == rule && g.generation == generation {
                Ok(g.status.clone())
            } else {
                Err("child funding approval differs")
            };
        }
        if b.children.len() >= usize::from(canic_management_protocol::stored::MAX_SUBNET_CHILDREN)
            || system::time() >= rule.expires_at_ns
        {
            return Err("child funding grant expired or ledger full");
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
        Ok(ChildFundingStatus::default())
    })?;
    if !status.closed && system::time() < terms.1 {
        method::call::<Watch>(child, 10, &terms)
            .await
            .map_err(|_| "balance trigger delivery pending")??;
    }
    Ok(read(|b| b.children[&child].status.clone()))
}

fn controller() -> Result<(), String> {
    system::is_controller(&system::caller())
        .then_some(())
        .ok_or_else(|| "controller required".into())
}

/// Revoke future payments and preserve any issued debit.
/// # Errors
/// Requires the parent's controller and an admitted child.
pub fn stop(child: Principal) -> Result<ChildFundingStatus, String> {
    controller()?;
    change(|b| {
        let g = b.children.get_mut(&child).ok_or("unknown funding child")?;
        g.status.closed = true;
        Ok(g.status.clone())
    })
}

/// Install the threshold under the actual directory parent.
/// # Errors
/// Refuses another caller or changes to installed terms.
pub fn watch((threshold, deadline): (u64, u64)) -> Result<(), String> {
    let parent = system::caller();
    if !DIRECTORY.with_borrow(|d| d.as_ref().is_some_and(|d| d.publisher() == parent))
        || threshold == 0
        || deadline <= system::time()
    {
        return Err("current parent and valid funding terms required".into());
    }
    let inserted = change(|b| {
        if let Some(i) = &b.incoming {
            return if (i.parent, i.threshold, i.deadline) == (parent, threshold, deadline) {
                Ok(false)
            } else {
                Err("balance trigger changed")
            };
        }
        b.incoming = Some(Incoming {
            parent,
            threshold,
            deadline,
            paid: 0,
        });
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
pub async fn request((sequence, balance): (u64, u128)) -> Result<u64, String> {
    let child = system::caller();
    let amount = change(|b| {
        let g = b.children.get_mut(&child).ok_or("no funding approval")?;
        if direct(child)? != g.generation {
            return Err("funded child generation changed".to_owned());
        }
        if sequence == 0 {
            return Err("invalid payment sequence".into());
        }
        if sequence <= g.paid {
            return Ok(None);
        }
        let s = &mut g.status;
        if s.closed
            || s.issued
            || g.paid.checked_add(1) != Some(sequence)
            || system::time() >= g.rule.expires_at_ns
            || balance >= u128::from(g.rule.below_cycles)
            || g.rule.amount_cycles > g.rule.budget_cycles - s.charged_cycles
            || raw::liquid_cycle_balance() < u128::from(g.rule.amount_cycles)
        {
            return Err("child funding refused or pending".into());
        }
        s.attempts += 1;
        s.charged_cycles += g.rule.amount_cycles;
        s.issued = true;
        s.refund_cycles = None;
        Ok(Some(g.rule.amount_cycles))
    })?;
    if let Some(amount) = amount {
        let (success, refund) = crate::engine::deposit_cycles(child, amount).await;
        change(|b| {
            let g = b.children.get_mut(&child).expect("issued child funding");
            g.status.issued = !success && refund != Some(u128::from(amount));
            g.status.closed |= !success;
            g.status.refund_cycles = refund;
            if success {
                g.paid = sequence;
            }
        });
        if !success {
            return Err("deposit failed or outcome uncertain".into());
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
pub async fn check() -> Result<bool, String> {
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
        .map_err(|_| "funding reply pending")??;
    if ack != seq {
        return Err("unexpected funding sequence".into());
    }
    change(|b| {
        let i = b.incoming.as_mut().expect("original trigger");
        i.paid = i.paid.max(seq);
    });
    Ok(true)
}
