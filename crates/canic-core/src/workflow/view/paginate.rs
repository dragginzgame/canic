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
pub fn clamp_page_request(request: PageRequest) -> PageRequest {
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
