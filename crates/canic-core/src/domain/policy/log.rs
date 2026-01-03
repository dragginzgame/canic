use crate::config::schema::LogConfig;

///
/// LogRetentionParams
///

#[derive(Clone, Debug)]
pub struct LogRetentionParams {
    pub cutoff: Option<u64>,
    pub max_entries: usize,
    pub max_entry_bytes: u32,
}

#[must_use]
pub fn retention_params(cfg: &LogConfig, now: u64) -> LogRetentionParams {
    let max_entries = usize::try_from(cfg.max_entries).unwrap_or(usize::MAX);
    let cutoff = cfg.max_age_secs.map(|max_age| now.saturating_sub(max_age));

    LogRetentionParams {
        cutoff,
        max_entries,
        max_entry_bytes: cfg.max_entry_bytes,
    }
}
