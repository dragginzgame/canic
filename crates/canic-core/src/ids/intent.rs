use crate::cdk::types::BoundedString128;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

///
/// IntentId
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

pub type IntentResourceKey = BoundedString128;
