use serde::Serialize;

///
/// CyclesReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CyclesReport {
    pub fleet: String,
    pub network: String,
    pub since_seconds: u64,
    pub generated_at_secs: u64,
    pub canisters: Vec<CyclesCanisterReport>,
}

///
/// CyclesCanisterReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CyclesCanisterReport {
    pub role: String,
    #[serde(skip)]
    pub tree_prefix: String,
    pub canister_id: String,
    pub status: String,
    pub sample_count: usize,
    pub total_samples: u64,
    pub requested_since_secs: u64,
    pub coverage_seconds: Option<u64>,
    pub coverage_status: String,
    pub latest_timestamp_secs: Option<u64>,
    pub latest_cycles: Option<u128>,
    pub baseline_timestamp_secs: Option<u64>,
    pub baseline_cycles: Option<u128>,
    pub delta_cycles: Option<i128>,
    pub rate_cycles_per_hour: Option<i128>,
    pub burn_cycles: Option<u128>,
    pub burn_cycles_per_hour: Option<u128>,
    pub topup_cycles_per_hour: Option<u128>,
    pub topups: Option<CyclesTopupSummary>,
    pub error: Option<String>,
}

///
/// CyclesTopupSummary
///

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct CyclesTopupSummary {
    pub request_scheduled: u64,
    pub request_ok: u64,
    pub request_err: u64,
    pub transferred_cycles: u128,
}

impl CyclesTopupSummary {
    pub(super) const fn is_empty(&self) -> bool {
        self.request_scheduled == 0 && self.request_ok == 0 && self.request_err == 0
    }
}

///
/// CycleTopupEventPage
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CycleTopupEventPage {
    pub(super) entries: Vec<CycleTopupEventSample>,
    pub(super) total: u64,
}

///
/// CycleTopupEventSample
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CycleTopupEventSample {
    pub(super) timestamp_secs: u64,
    pub(super) transferred_cycles: Option<u128>,
    pub(super) status: CycleTopupStatus,
}

///
/// CycleTopupStatus
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CycleTopupStatus {
    RequestErr,
    RequestOk,
    RequestScheduled,
}

///
/// CycleTrackerPage
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CycleTrackerPage {
    pub(super) entries: Vec<CycleTrackerSample>,
    pub(super) total: u64,
}

///
/// CycleTrackerSample
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CycleTrackerSample {
    pub(super) timestamp_secs: u64,
    pub(super) cycles: u128,
}
