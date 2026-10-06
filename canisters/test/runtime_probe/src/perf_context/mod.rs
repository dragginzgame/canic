//! Module: runtime_probe::perf_context
//!
//! Responsibility: bracket production instruction accounting across controlled IC awaits.
//! Boundary: simulated HTTP replies control scheduling; Core owns real counter reads.

#![expect(
    clippy::future_not_send,
    reason = "the IC probe retains invocation-owned instrumentation on one canister thread"
)]

use canic::{
    __internal::core::{dispatch, perf},
    ids::{EndpointCall, EndpointCallKind, EndpointId},
};
use ic_management_canister_types::{HttpMethod, HttpRequestArgs};
use std::cell::RefCell;

thread_local! {
    static ENTERED: RefCell<[bool; 2]> = const { RefCell::new([false; 2]) };
    static COMPLETED: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Real counter reads bracketing the recorder independently in its own call context.
#[derive(candid::CandidType)]
pub struct PerfProbeObservation {
    before_enter: u64,
    after_enter: u64,
    before_first: u64,
    after_first: u64,
    before_resumed: u64,
    after_resumed: u64,
    before_exit: u64,
    after_exit: u64,
}

/// Read-only sample projection from the production table.
#[derive(candid::CandidType)]
pub struct PerfProbeMetric {
    name: String,
    count: u64,
    instructions: u64,
}

fn work(iterations: u32) {
    let mut value = std::hint::black_box(1_u64);
    for _ in 0..iterations {
        value = std::hint::black_box(value.wrapping_mul(3).wrapping_add(1));
    }
    std::hint::black_box(value);
}

async fn await_controlled_response(which: u8) {
    let args = HttpRequestArgs {
        url: format!("https://perf-context.invalid/{which}"),
        max_response_bytes: Some(256),
        method: HttpMethod::GET,
        headers: Vec::new(),
        body: None,
        transform: None,
        is_replicated: None,
        // Match the counter probe's version-1 cycle quote rather than changing its budget.
        pricing_version: Some(1),
    };
    let cycles = ic_cdk::api::cost_http_request(args.url.len() as u64, 256);
    ic_cdk::call::Call::unbounded_wait(candid::Principal::management_canister(), "http_request")
        .with_arg(args)
        .with_cycles(cycles)
        .await
        .expect("controlled simulator HTTP reply");
}

#[ic_cdk::query]
fn perf_context_entered() -> Vec<bool> {
    ENTERED.with_borrow(|entered| entered.to_vec())
}

#[ic_cdk::query]
fn perf_context_completed() -> Vec<u8> {
    COMPLETED.with_borrow(Clone::clone)
}

#[canic::canic_update(public)]
async fn perf_context_probe(which: u8) -> Result<PerfProbeObservation, canic::Error> {
    assert!(which < 2);
    work(if which == 0 { 100 } else { 10_000 });
    let before_enter = perf::perf_counter();
    let call = EndpointCall {
        endpoint: EndpointId::new(if which == 0 {
            "perf_context_a"
        } else {
            "perf_context_b"
        }),
        kind: EndpointCallKind::Update,
    };
    let mut observed = dispatch::measure_endpoint_async(call, async {
        let after_enter = perf::perf_counter();
        ENTERED.with_borrow_mut(|entered| entered[usize::from(which)] = true);
        work(if which == 0 { 10_000 } else { 20_000 });
        let before_first = perf::perf_counter();
        canic::perf!("first_{which}");
        let after_first = perf::perf_counter();
        await_controlled_response(which).await;
        work(10_000);
        let before_resumed = perf::perf_counter();
        canic::perf!("resumed_{which}");
        let after_resumed = perf::perf_counter();
        let before_exit = perf::perf_counter();
        PerfProbeObservation {
            before_enter,
            after_enter,
            before_first,
            after_first,
            before_resumed,
            after_resumed,
            before_exit,
            after_exit: 0,
        }
    })
    .await;
    observed.after_exit = perf::perf_counter();
    COMPLETED.with_borrow_mut(|completed| completed.push(which));
    Ok(observed)
}

#[ic_cdk::query]
fn perf_context_metrics() -> Vec<PerfProbeMetric> {
    perf::entries()
        .into_iter()
        .filter_map(|entry| {
            let name = match entry.key {
                perf::PerfKey::Endpoint { name, .. } if name.starts_with("perf_context_") => name,
                perf::PerfKey::Checkpoint { scope, label } if scope == module_path!() => label,
                _ => return None,
            };
            Some(PerfProbeMetric {
                name,
                count: entry.count,
                instructions: entry.total_instructions,
            })
        })
        .collect()
}
