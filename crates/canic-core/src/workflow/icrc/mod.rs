//! Module: workflow::icrc
//!
//! Responsibility: group read-only ICRC standards workflow queries.
//! Does not own: endpoint authorization, dispatch handlers, or standards DTO schemas.
//! Boundary: workflow query namespace over ICRC registries and dispatchers.

pub mod query;
