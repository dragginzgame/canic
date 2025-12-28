use crate::{
    Error,
    dto::log::LogEntryView,
    dto::page::{Page, PageRequest},
    log::Level,
    model::memory::log::{Log, RetentionSummary as ModelRetentionSummary, apply_retention},
    ops::adapter::log::log_entry_to_view,
    ops::view::paginate_vec,
};

///
/// RetentionSummary
/// Ops-level summary for retention sweeps.
///

#[derive(Clone, Debug, Default)]
pub struct RetentionSummary {
    pub before: u64,
    pub retained: u64,
    pub dropped_by_age: u64,
    pub dropped_by_limit: u64,
}

impl RetentionSummary {
    #[must_use]
    pub const fn dropped_total(&self) -> u64 {
        self.dropped_by_age + self.dropped_by_limit
    }
}

pub type LogEntryDto = LogEntryView;

///
/// LogOps
///
/// Read-only facade over stable log storage.
///
/// This is a **view op**:
/// - no mutation
/// - no policy
/// - safe to call directly from query endpoints
///

pub struct LogOps;

impl LogOps {
    #[must_use]
    pub fn page(
        crate_name: Option<String>,
        topic: Option<String>,
        min_level: Option<Level>,
        request: PageRequest,
    ) -> Page<LogEntryDto> {
        let mut entries = Log::snapshot();

        // Filter
        if let Some(ref name) = crate_name {
            entries.retain(|e| &e.crate_name == name);
        }
        if let Some(ref t) = topic {
            entries.retain(|e| e.topic.as_deref() == Some(t.as_str()));
        }
        if let Some(min) = min_level {
            entries.retain(|e| e.level >= min);
        }

        // Newest first
        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let views: Vec<LogEntryDto> = entries.iter().map(log_entry_to_view).collect();

        paginate_vec(views, request)
    }
}

/// Apply log retention policy and return a summary.
pub fn apply_log_retention() -> Result<RetentionSummary, Error> {
    let summary: ModelRetentionSummary = apply_retention()?;
    Ok(RetentionSummary {
        before: summary.before,
        retained: summary.retained,
        dropped_by_age: summary.dropped_by_age,
        dropped_by_limit: summary.dropped_by_limit,
    })
}
