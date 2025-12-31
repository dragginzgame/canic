use crate::{
    Error,
    cdk::utils::time::now_secs,
    dto::{
        log::LogEntryView,
        page::{Page, PageRequest},
    },
    log::Level,
    model::memory::log::{Log, RetentionSummary, apply_retention_with_cfg},
    ops::{adapter::log::log_entry_to_view, config::ConfigOps, view::paginate::paginate_vec},
};

///
/// LogOps
///
/// Logging control operations.
///
/// Callers must ensure that control ops are only invoked
/// from update/workflow contexts.
///
pub struct LogOps;

impl LogOps {
    pub fn append_runtime_log(
        crate_name: &str,
        topic: Option<&str>,
        level: Level,
        message: &str,
    ) -> Result<u64, Error> {
        if !crate::log::is_ready() {
            return Ok(0);
        }

        let cfg = ConfigOps::log_config()?;
        let now = now_secs();

        Log::append(&cfg, now, crate_name, topic, level, message)
    }
}

impl LogOps {
    /// Apply log retention policy and return a summary.
    pub fn apply_retention() -> Result<RetentionSummary, Error> {
        let cfg = ConfigOps::log_config()?;
        let now = now_secs();

        apply_retention_with_cfg(&cfg, now)
    }
}

///
/// LogViewOps
///
/// Read-only log views and pagination helpers.
///
pub struct LogViewOps;

impl LogViewOps {
    #[must_use]
    pub fn page(
        crate_name: Option<String>,
        topic: Option<String>,
        min_level: Option<Level>,
        request: PageRequest,
    ) -> Page<LogEntryView> {
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

        let views: Vec<LogEntryView> = entries.iter().map(log_entry_to_view).collect();

        paginate_vec(views, request)
    }
}
