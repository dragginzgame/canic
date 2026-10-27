//! Convert the authoritative fence view without altering its stored representation.
use crate::dto::{authority_restore as source, authority_restore_status as native};

impl From<source::AuthorityRestoreFenceStatusResponse> for native::Status {
    fn from(value: source::AuthorityRestoreFenceStatusResponse) -> Self {
        Self {
            authority_canister: value.authority_canister.into(),
            phase: match value.phase {
                source::AuthorityRestoreFencePhase::Open => native::Phase::Open,
                source::AuthorityRestoreFencePhase::Sealed => native::Phase::Sealed,
                source::AuthorityRestoreFencePhase::ReleaseSealed {
                    review_sha256,
                    recipient,
                } => native::Phase::ReleaseSealed {
                    review_sha256,
                    recipient: recipient.into(),
                },
            },
            operation_id: value.operation_id,
            history_total_num_changes: value.history_total_num_changes,
            changed_at_ns: value.changed_at_ns,
        }
    }
}
