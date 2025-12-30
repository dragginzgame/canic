use crate::dto::page::{Page, PageRequest};

///
/// Pagination
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
#[allow(clippy::cast_possible_truncation)]
pub fn paginate_vec<T>(items: Vec<T>, request: PageRequest) -> Page<T> {
    let request = clamp_page_request(request);
    let total = items.len() as u64;

    let start = request.offset.min(total) as usize;
    let end = request.offset.saturating_add(request.limit).min(total) as usize;

    let entries = items.into_iter().skip(start).take(end - start).collect();

    Page { entries, total }
}
