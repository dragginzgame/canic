//! Module: view::replay_release
//!
//! Responsibility: retain one exact replay journal row and continuation evidence.
//! Does not own: persistence, replay admission or release decisions.
//! Boundary: storage supplies rows; runtime ops project controller-visible metadata.

use crate::storage::stable::replay::ReplayReceiptEntryRecord;

/// One observed stable value with key-only lookahead.
pub struct ReplayReleasePageView {
    pub entry: Option<ReplayReceiptEntryRecord>,
    pub has_more: bool,
}
