// Read the visit count for an attested user shard using the default Wire budgets.
#[query(roles = &["user_shard"])]
pub(super) fn visits() -> u64 {
    VISITS.get()
}
