//! Preconfigured SNS deployments and helpers for looking up their canisters.

use candid::Principal;
use std::{collections::HashMap, sync::OnceLock};

// -----------------------------------------------------------------------------
// Parsing
// -----------------------------------------------------------------------------

fn parse_principal(sns: SnsType, role: &'static str, text: &'static str) -> Principal {
    Principal::from_text(text)
        .unwrap_or_else(|_| panic!("Invalid SNS {sns:?} {role} principal: {text}"))
}

// -----------------------------------------------------------------------------
// Types
// -----------------------------------------------------------------------------

///
/// SnsCanisters
///

#[derive(Clone, Debug)]
pub struct SnsCanisters {
    pub root: Principal,
    pub governance: Principal,
    pub index: Principal,
    pub ledger: Principal,
}

///
/// SnsRole
///

#[derive(Clone, Copy, Debug)]
pub enum SnsRole {
    Root,
    Governance,
    Index,
    Ledger,
}

include!("sns.inc.rs");
