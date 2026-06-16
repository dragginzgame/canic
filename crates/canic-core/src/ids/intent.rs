//! Module: ids::intent
//! Responsibility: intent identifiers and resource keys.
//! Does not own: intent execution, replay policy, or resource authorization.
//! Boundary: exposes compact IDs used across workflow and ops boundaries.

use crate::cdk::types::BoundedString128;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

///
/// IntentId
///
/// Numeric identifier for one recorded or replayable intent.
///

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct IntentId(pub u64);

impl Display for IntentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

///
/// IntentResourceKey
///
/// Bounded resource key associated with an intent.
///

pub type IntentResourceKey = BoundedString128;
