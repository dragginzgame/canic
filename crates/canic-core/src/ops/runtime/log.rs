use crate::{
    dto::page::{Page, PageRequest},
    log::Level,
    model::memory::log::{self, Log},
};

pub type LogEntryDto = log::LogEntry;

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

        paginate(entries, request)
    }
}

// You can reuse your existing pagination helper; this mirrors your MetricsOps.
#[allow(clippy::cast_possible_truncation)]
fn paginate<T>(entries: Vec<T>, request: PageRequest) -> Page<T> {
    let request = request.clamped();
    let total = entries.len() as u64;

    let start = request.offset.min(total) as usize;
    let end = request.offset.saturating_add(request.limit).min(total) as usize;

    let entries = entries.into_iter().skip(start).take(end - start).collect();
    Page { entries, total }
}
