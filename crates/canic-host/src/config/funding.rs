//! Fleet top-up declarations normalize human-authored amounts before planning.
use serde::{Deserialize, Serialize};

/// A declared funding edge. Historical name-only entries authorize no automatic debit.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged, try_from = "Input")]
pub enum ChildTopUp {
    /// Eligibility retained from an existing fleet without a replenishment rule.
    Eligible(String),
    /// Send the fixed amount when the child's balance falls strictly below the threshold.
    Rule {
        /// Direct child component in the fleet.
        child: String,
        /// Trigger threshold in cycles.
        below_cycles: u64,
        /// Fixed transfer in cycles.
        amount_cycles: u64,
    },
}

impl ChildTopUp {
    /// Component receiving funds from its declared parent.
    #[must_use]
    pub fn child(&self) -> &str {
        match self {
            Self::Eligible(child) | Self::Rule { child, .. } => child,
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged, deny_unknown_fields)]
enum Input {
    Eligible(String),
    Rule {
        child: String,
        below_cycles: Amount,
        amount_cycles: Amount,
    },
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Amount {
    Cycles(u64),
    Quantity(String),
}

impl TryFrom<Input> for ChildTopUp {
    type Error = String;

    fn try_from(input: Input) -> Result<Self, Self::Error> {
        match input {
            Input::Eligible(child) => Ok(Self::Eligible(child)),
            Input::Rule {
                child,
                below_cycles,
                amount_cycles,
            } => Ok(Self::Rule {
                below_cycles: below_cycles.cycles(&child, "below_cycles")?,
                amount_cycles: amount_cycles.cycles(&child, "amount_cycles")?,
                child,
            }),
        }
    }
}

impl Amount {
    fn cycles(self, child: &str, field: &str) -> Result<u64, String> {
        match self {
            Self::Cycles(value) => Ok(value),
            Self::Quantity(text) => quantity(&text).map_err(|reason| {
                format!("top_up_children {child:?} has {field}={text:?}: {reason}")
            }),
        }
    }
}

fn quantity(text: &str) -> Result<u64, &'static str> {
    let (number, places) = [('B', 9), ('T', 12), ('Q', 15)]
        .into_iter()
        .find_map(|(unit, places)| text.strip_suffix(unit).map(|number| (number, places)))
        .ok_or("expected an uppercase B, T or Q suffix")?;
    let (whole, fraction) = number.split_once('.').unwrap_or((number, ""));
    if (whole.is_empty() && fraction.is_empty())
        || !whole
            .bytes()
            .chain(fraction.bytes())
            .all(|b| b.is_ascii_digit())
    {
        return Err("expected an unsigned decimal quantity");
    }
    let fraction = fraction.trim_end_matches('0');
    if fraction.len() > places {
        return Err("precision is below one cycle");
    }
    whole
        .bytes()
        .chain(fraction.bytes())
        .chain(std::iter::repeat_n(b'0', places - fraction.len()))
        .try_fold(0_u64, |cycles, digit| {
            cycles.checked_mul(10)?.checked_add(u64::from(digit - b'0'))
        })
        .ok_or("cycle amount exceeds u64")
}
