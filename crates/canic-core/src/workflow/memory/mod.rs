//! Module: workflow::memory
//!
//! Responsibility: group read-only memory workflow queries.
//! Does not own: memory registry mutation, endpoint authorization, or DTO schemas.
//! Boundary: workflow query namespace over runtime memory ops.

pub mod query;
