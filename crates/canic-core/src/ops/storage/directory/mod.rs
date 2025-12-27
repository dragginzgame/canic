mod app;
mod subnet;

pub use app::*;
pub use subnet::*;

use crate::{
    dto::{
        directory::DirectoryView,
        page::{Page, PageRequest},
    },
    ids::CanisterRole,
};
use candid::Principal;

///
/// Pagination
///

#[must_use]
pub(crate) fn paginate(
    view: DirectoryView,
    request: PageRequest,
) -> Page<(CanisterRole, Principal)> {
    let request = request.clamped();
    let total = view.len() as u64;
    let (start, end) = pagination_bounds(total, request);

    let entries = view.into_iter().skip(start).take(end - start).collect();

    Page { entries, total }
}

#[allow(clippy::cast_possible_truncation)]
fn pagination_bounds(total: u64, request: PageRequest) -> (usize, usize) {
    let start = request.offset.min(total);
    let end = request.offset.saturating_add(request.limit).min(total);

    let start = start as usize;
    let end = end as usize;

    (start, end)
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::{DirectoryView, PageRequest, paginate};
    use crate::{cdk::types::Principal, ids::CanisterRole};

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn sample_view() -> DirectoryView {
        vec![("a".into(), p(1)), ("b".into(), p(2)), ("c".into(), p(4))]
    }

    #[test]
    fn paginate_within_bounds() {
        let page = paginate(sample_view(), PageRequest::new(1, 1));

        assert_eq!(page.total, 3);
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.entries[0].0, CanisterRole::from("b"));
    }

    #[test]
    fn paginate_truncates_at_total() {
        let page = paginate(sample_view(), PageRequest::new(5, 2));

        assert_eq!(page.total, 3);
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.entries[0].0, CanisterRole::from("c"));
    }

    #[test]
    fn paginate_handles_offset_beyond_range() {
        let page = paginate(sample_view(), PageRequest::new(5, 10));

        assert_eq!(page.total, 3);
        assert!(page.entries.is_empty());
    }
}
