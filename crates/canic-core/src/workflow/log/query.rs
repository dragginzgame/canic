//! Module: workflow::log::query
//!
//! Responsibility: expose filtered runtime log pages.
//! Does not own: log storage mutation, endpoint authorization, or DTO schemas.
//! Boundary: workflow query facade over runtime log ops.

use crate::{
    dto::log::{LogPage, LogPageRequest},
    log::Level,
    ops::runtime::log::LogOps,
};

///
/// LogQuery
/// Read-only log views and pagination helpers.
///

pub struct LogQuery;

impl LogQuery {
    pub fn page(
        crate_name: Option<String>,
        topic: Option<String>,
        min_level: Option<Level>,
        page: LogPageRequest,
    ) -> Result<LogPage, crate::dto::error::Error> {
        LogOps::page_filtered(crate_name.as_deref(), topic.as_deref(), min_level, page)
            .map_err(Into::into)
    }
}
