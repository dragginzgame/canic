use serde::Serialize;

///
/// CyclesReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct CyclesReport {
    pub(super) deployment: String,
    pub(super) environment: String,
    pub(super) since_seconds: u64,
    pub(super) generated_at_secs: u64,
    pub(super) canisters: Vec<CyclesCanisterReport>,
}

///
/// CyclesCanisterReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct CyclesCanisterReport {
    pub(super) role: String,
    #[serde(skip)]
    pub(super) tree_prefix: String,
    pub(super) canister_id: String,
    pub(super) status: CyclesCanisterStatus,
    pub(super) sample_count: usize,
    pub(super) total_samples: u64,
    pub(super) requested_since_secs: u64,
    pub(super) coverage_seconds: Option<u64>,
    pub(super) coverage_status: CyclesCoverageStatus,
    pub(super) latest_timestamp_secs: Option<u64>,
    pub(super) latest_cycles: Option<u128>,
    pub(super) baseline_timestamp_secs: Option<u64>,
    pub(super) baseline_cycles: Option<u128>,
    pub(super) delta_cycles: Option<i128>,
    pub(super) rate_cycles_per_hour: Option<i128>,
    pub(super) burn_cycles: Option<u128>,
    pub(super) burn_cycles_per_hour: Option<u128>,
    pub(super) topup_cycles_per_hour: Option<u128>,
    pub(super) topups: Option<CyclesTopupSummary>,
    pub(super) error: Option<String>,
}

///
/// CyclesCanisterStatus
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CyclesCanisterStatus {
    Empty,
    Error,
    Ok,
}

impl CyclesCanisterStatus {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Error => "error",
            Self::Ok => "ok",
        }
    }
}

///
/// CyclesCoverageStatus
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CyclesCoverageStatus {
    Covered,
    None,
    Partial,
}

impl CyclesCoverageStatus {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Covered => "covered",
            Self::None => "none",
            Self::Partial => "partial",
        }
    }
}

///
/// CyclesTopupSummary
///

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub(super) struct CyclesTopupSummary {
    pub(super) request_scheduled: u64,
    pub(super) request_ok: u64,
    pub(super) request_err: u64,
    pub(super) transferred_cycles: u128,
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
