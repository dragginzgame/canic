//! Module: workflow::auth
//!
//! Responsibility: coordinate proof-derived local application authorization state.
//! Does not own: endpoint variants, protected configuration, proof cryptography, or access policy.
//! Boundary: B4 supplies verified proof authority; this workflow performs exact retry and atomic state commit.

pub mod application_sessions;
