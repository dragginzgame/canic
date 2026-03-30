//! IC post-upgrade lifecycle adapters.
//!
//! This module contains **synchronous glue code** that adapts the IC
//! `post_upgrade` hook into async bootstrap workflows.
//!
//! Responsibilities:
//! - Restore minimal environment state required by workflows
//! - Perform no async work directly
//! - Delegate immediately to workflow bootstrap
//!
//! This module must NOT:
//! - Perform sequencing or orchestration
//! - Encode policy decisions
//! - Call ops beyond minimal environment restoration

pub mod nonroot;
pub mod root;
