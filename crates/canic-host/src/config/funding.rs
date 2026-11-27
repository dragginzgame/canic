//! Fleet top-up declarations normalize human-authored amounts before planning.
use serde::{Deserialize, Serialize};

/// Fixed replenishment rule for one direct child.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "Input")]
pub struct ChildTopUp {
    /// Direct child component in the fleet.
    pub child: String,
    /// Trigger threshold in cycles.
    pub below_cycles: u64,
    /// Fixed transfer in cycles.
    pub amount_cycles: u64,
}

#[derive(Deserialize)]
#[serde(
    deny_unknown_fields,
    expecting = "a top_up_children rule with child, below_cycles and amount_cycles"
)]
struct Input {
    child: String,
    below_cycles: Amount,
    amount_cycles: Amount,
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
        Ok(Self {
            below_cycles: input.below_cycles.cycles(&input.child, "below_cycles")?,
            amount_cycles: input.amount_cycles.cycles(&input.child, "amount_cycles")?,
            child: input.child,
        })
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
