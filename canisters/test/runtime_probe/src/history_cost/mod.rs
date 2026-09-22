//! Measure complete sampling and independent cached-history reads in real Wasm.
//! This fixture owns synthetic inputs only; the production API owns all work.

use canic::{
    Error,
    api::public_status::{ApplicationMetricsSampler, PublicStatusApi},
    dto::{
        page::PageRequest,
        public_status::{PublicHistoryRequest, PublicMetric, PublicMetricFamily, PublicMetricKind},
    },
};
use std::cell::Cell;

thread_local! {
    static ROWS: Cell<u16> = const { Cell::new(0) };
}

/// Complete production callback cost, including provider construction and history.
pub fn sample(rows: u16) -> Result<u64, Error> {
    assert!(rows <= 256);
    ROWS.set(rows);
    PublicStatusApi::set_application_sampler(Some(ApplicationMetricsSampler::new(provider)));
    let start = ic_cdk::api::performance_counter(0);
    PublicStatusApi::sample_metrics()?;
    Ok(ic_cdk::api::performance_counter(0) - start)
}

/// Separate query-message cost and response evidence for one exact series.
#[derive(candid::CandidType)]
pub struct HistoryCost {
    instructions: u64,
    points: u64,
    total: u64,
    reserved_bytes: u64,
    values_valid: bool,
}

pub fn history(limit: u64) -> HistoryCost {
    let start = ic_cdk::api::performance_counter(0);
    let history = PublicStatusApi::history(PublicHistoryRequest {
        family: PublicMetricFamily::Application,
        name: metric_name(0),
        canister_id: None,
        page: PageRequest { offset: 0, limit },
    });
    let instructions = ic_cdk::api::performance_counter(0) - start;
    HistoryCost {
        instructions,
        points: history.points.entries.len() as u64,
        total: history.points.total,
        reserved_bytes: history.reserved_bytes,
        values_valid: history.points.entries.iter().all(|point| point.value == 7),
    }
}

fn metric_name(index: u16) -> String {
    format!("icydb.entity.synthetic_qualified_entity_path_{index:04}.instructions_total")
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "production sampler callback signature"
)]
fn provider() -> Result<Vec<PublicMetric>, Error> {
    let now = ic_cdk::api::time();
    Ok((0..ROWS.get())
        .map(|index| PublicMetric {
            name: metric_name(index),
            unit: "instructions".into(),
            value: u128::from(index) + 7,
            canister_id: None,
            observed_at_ns: now,
            kind: PublicMetricKind::Gauge,
        })
        .collect())
}
