//! Public access helpers re-exported from the core access layer.

pub use crate::__internal::core::access::{
    AccessError, auth, caller_authority, deployment, env,
    expr::{AccessCheck, AccessContext},
    fleet,
};
