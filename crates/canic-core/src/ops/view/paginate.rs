use crate::dto::page::{Page, PageRequest};

///
/// Pagination
///

pub fn paginate_vec<T>(items: Vec<T>, request: PageRequest) -> Page<T> {
    let request = request.clamped();
    let total = items.len() as u64;

    let start = request.offset.min(total) as usize;
    let end = request.offset.saturating_add(request.limit).min(total) as usize;

    let entries = items.into_iter().skip(start).take(end - start).collect();

    Page { entries, total }
}
