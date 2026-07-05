///
/// LogRetentionParams
///

#[derive(Clone, Debug)]
pub struct LogRetentionParams {
    pub cutoff: Option<u64>,
    pub max_entries: usize,
    pub max_entry_bytes: u32,
}

///
/// LogRetentionPolicyInput
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LogRetentionPolicyInput {
    pub max_entries: u64,
    pub max_entry_bytes: u32,
    pub max_age_secs: Option<u64>,
}

#[must_use]
pub fn retention_params(input: LogRetentionPolicyInput, now: u64) -> LogRetentionParams {
    let max_entries = usize::try_from(input.max_entries).unwrap_or(usize::MAX);
    let cutoff = input
        .max_age_secs
        .map(|max_age| now.saturating_sub(max_age));

    LogRetentionParams {
        cutoff,
        max_entries,
        max_entry_bytes: input.max_entry_bytes,
    }
}
