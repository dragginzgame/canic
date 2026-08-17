//! Module: workflow::view::paginate
//!
//! Responsibility: clamp and apply page requests to in-memory view collections.
//! Does not own: storage reads, query authorization, or DTO schema definitions.
//! Boundary: maps existing vectors and page requests into bounded page responses.

use crate::dto::page::{Page, PageRequest};

///
/// Pagination
///
/// Shared page sizing helpers for workflow query projections.
///

pub const PAGE_REQUEST_MAX_LIMIT: u64 = 1_000;

#[must_use]
fn clamp_page_request(request: PageRequest) -> PageRequest {
    let limit = request.limit.min(PAGE_REQUEST_MAX_LIMIT);
    PageRequest {
        limit,
        offset: request.offset,
    }
}

#[must_use]
#[expect(clippy::cast_possible_truncation)]
pub fn paginate_vec<T>(items: Vec<T>, request: PageRequest) -> Page<T> {
    let request = clamp_page_request(request);
    let total = items.len() as u64;

    let start = request.offset.min(total) as usize;
    let end = request.offset.saturating_add(request.limit).min(total) as usize;

    let entries = items.into_iter().skip(start).take(end - start).collect();

    Page { entries, total }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_limit_accepts_the_boundary_and_clamps_its_first_excess() {
        let items = (0..PAGE_REQUEST_MAX_LIMIT + 2).collect::<Vec<_>>();
        let at_limit = paginate_vec(
            items.clone(),
            PageRequest {
                limit: PAGE_REQUEST_MAX_LIMIT,
                offset: 0,
            },
        );
        let first_excess = paginate_vec(
            items,
            PageRequest {
                limit: PAGE_REQUEST_MAX_LIMIT + 1,
                offset: 0,
            },
        );

        assert_eq!(at_limit.entries.len() as u64, PAGE_REQUEST_MAX_LIMIT);
        assert_eq!(first_excess.entries, at_limit.entries);
        assert_eq!(first_excess.total, PAGE_REQUEST_MAX_LIMIT + 2);
    }
}
