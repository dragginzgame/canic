//! IC lifecycle adapters.
//!
//! This module adapts the IC’s synchronous lifecycle hooks (`init`,
//! `post_upgrade`, etc.) into the system’s two-phase initialization model:
//!
//! 1. **Synchronous runtime seeding**
//!    Minimal, non-async work that must execute inside the IC hook.
//!
//! 2. **Asynchronous bootstrap**
//!    Full initialization workflows scheduled via the timer immediately
//!    after the hook returns.
//!
//! This module exists to isolate **IC execution constraints** (synchronous
//! hooks, no `await`, strict time limits) from application orchestration.
//!
//! **DO NOT MERGE INTO WORKFLOW.**
//!
//! `lifecycle` is responsible only for *when* and *how* workflows are
//! permitted to start under IC rules. All orchestration, sequencing,
//! policy, and domain logic must remain in `workflow`.

pub mod nonroot;
pub mod root;
