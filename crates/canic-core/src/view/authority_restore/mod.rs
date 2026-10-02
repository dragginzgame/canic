//! Mutation-fence purpose projected from the current durable authority record.

/// Snapshot recovery cannot reopen a canister committed to reviewed Fleet release.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorityMutationFence {
    Open,
    Snapshot,
    Release,
}
