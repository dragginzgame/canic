//! Module: workflow::topology::directory
//!
//! Responsibility: group read-only Fleet and Subnet Directory workflow queries.
//! Does not own: Directory storage mutation, endpoint authorization, or DTO schemas.
//! Boundary: workflow query namespace over Directory storage ops.

pub mod query;
