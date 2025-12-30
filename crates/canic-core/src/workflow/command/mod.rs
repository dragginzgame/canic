//! Command-layer logic.
//!
//! This module is reserved for *behavioral commands*: imperative, higher-level
//! operations that coordinate multiple subsystems (orchestration, placement,
//! retries, policy, etc.).
//!
//! IMPORTANT:
//! - This module is **not** for wire contracts, RPC schemas, or request/response
//!   protocol definitions.
//! - Bidirectional request/response pairs and transport-facing APIs belong in
//!   `ops::rpc` (or equivalent protocol modules).
//!
//! If the code only defines *how canisters talk to each other*, it does not
//! belong here. If it defines *what the system does*, it probably does.
