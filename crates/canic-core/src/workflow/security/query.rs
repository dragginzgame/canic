use crate::{
    dto::{
        page::{Page, PageRequest},
        security::SecurityEvent,
    },
    ops::runtime::security::SecurityOps,
    workflow::view::paginate::paginate_vec,
};

///
/// SecurityQuery
///

pub struct SecurityQuery;

impl SecurityQuery {
    /// Return newest-first security events from the bounded runtime ring.
    #[must_use]
    pub fn page(page: PageRequest) -> Page<SecurityEvent> {
        paginate_vec(SecurityOps::snapshot_newest_first(), page)
    }
}
