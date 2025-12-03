mod app;
mod subnet;

pub use app::*;
pub use subnet::*;

pub use crate::model::memory::directory::DirectoryView;

use candid::CandidType;
use serde::{Deserialize, Serialize};

///
/// DirectoryPageDto
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DirectoryPageDto {
    pub entries: DirectoryView,
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
}

///
/// Pagination
///

#[must_use]
pub(crate) fn paginate(view: DirectoryView, offset: u64, limit: u64) -> DirectoryPageDto {
    let total = view.len() as u64;
    let (start, end) = pagination_bounds(total, offset, limit);

    let entries = view.into_iter().skip(start).take(end - start).collect();

    DirectoryPageDto {
        entries,
        total,
        offset,
        limit,
    }
}

#[allow(clippy::cast_possible_truncation)]
fn pagination_bounds(total: u64, offset: u64, limit: u64) -> (usize, usize) {
    let start = offset.min(total);
    let end = (offset + limit).min(total);

    let start = start as usize;
    let end = end as usize;

    (start, end)
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::{DirectoryView, paginate};
    use crate::{
        model::memory::directory::PrincipalList,
        types::{CanisterType, Principal},
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn sample_view() -> DirectoryView {
        vec![
            ("a".into(), PrincipalList(vec![p(1)])),
            ("b".into(), PrincipalList(vec![p(2), p(3)])),
            ("c".into(), PrincipalList(vec![p(4)])),
        ]
    }

    #[test]
    fn paginate_within_bounds() {
        let page = paginate(sample_view(), 1, 1);

        assert_eq!(page.total, 3);
        assert_eq!(page.offset, 1);
        assert_eq!(page.limit, 1);
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.entries[0].0, CanisterType::from("b"));
    }

    #[test]
    fn paginate_truncates_at_total() {
        let page = paginate(sample_view(), 2, 5);

        assert_eq!(page.total, 3);
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.entries[0].0, CanisterType::from("c"));
    }

    #[test]
    fn paginate_handles_offset_beyond_range() {
        let page = paginate(sample_view(), 10, 5);

        assert_eq!(page.total, 3);
        assert!(page.entries.is_empty());
        assert_eq!(page.offset, 10);
        assert_eq!(page.limit, 5);
    }
}
