//! Module: access::application_authorization
//!
//! Responsibility: expose model-owned canonical application scope values.
//! Does not own: scope grammar, authorization decisions, state, or caller acquisition.
//! Boundary: the `canic` facade re-exports these types without redeclaring them.

pub use crate::model::auth::application_authorization::{
    ApplicationScope, ApplicationScopeError, ApplicationScopeRef, CanonicalApplicationScopes,
};
