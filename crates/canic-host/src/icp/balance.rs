//! Module: icp::balance
//!
//! Responsibility: observe the selected identity's default ICP and cycles accounts.
//! Does not own: funding sufficiency policy, Fleet authority, or deployment planning.
//! Boundary: parses the exact machine-readable balance output owned by ICP CLI.

use super::{error::IcpCommandError, model::IcpCli, run::run_json};

use serde::Deserialize;
use thiserror::Error as ThisError;

///
/// IcpBalanceError
///
/// Typed failure while observing an identity ledger balance through ICP CLI.
///
#[derive(Debug, ThisError)]
pub enum IcpBalanceError {
    #[error("ICP CLI returned an invalid {unit} balance: {value}")]
    InvalidAmount { unit: &'static str, value: String },

    #[error("ICP balance does not fit e8s: {value}")]
    IcpAmountOverflow { value: String },

    #[error(transparent)]
    Icp(#[from] IcpCommandError),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BalanceOutput {
    balance: String,
}

impl IcpCli {
    /// Observe the selected identity's default Cycles Ledger account.
    pub fn identity_cycles_balance(&self) -> Result<u128, IcpBalanceError> {
        let mut command = self.command();
        command.args(["cycles", "balance", "--json"]);
        self.add_target_args(&mut command);
        let output = run_json::<BalanceOutput>(&mut command)?;
        parse_cycles(&output.balance)
    }

    /// Observe the selected identity's default ICP Ledger account in e8s.
    pub fn identity_icp_balance_e8s(&self) -> Result<u64, IcpBalanceError> {
        let mut command = self.command();
        command.args(["token", "balance", "--json"]);
        self.add_target_args(&mut command);
        let output = run_json::<BalanceOutput>(&mut command)?;
        parse_icp_e8s(&output.balance)
    }
}

fn parse_cycles(value: &str) -> Result<u128, IcpBalanceError> {
    let amount = strip_unit(value, "cycles")?;
    amount
        .replace('_', "")
        .parse()
        .map_err(|_| invalid_amount("cycles", value))
}

fn parse_icp_e8s(value: &str) -> Result<u64, IcpBalanceError> {
    const E8S_PER_ICP: u64 = 100_000_000;

    let amount = strip_unit(value, "ICP")?.replace('_', "");
    let mut parts = amount.split('.');
    let whole = parts
        .next()
        .filter(|whole| !whole.is_empty())
        .and_then(|whole| whole.parse::<u64>().ok())
        .ok_or_else(|| invalid_amount("ICP", value))?;
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some()
        || fraction.len() > 8
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid_amount("ICP", value));
    }
    let fractional_e8s = if fraction.is_empty() {
        0
    } else {
        let padding =
            u32::try_from(8_usize - fraction.len()).map_err(|_| invalid_amount("ICP", value))?;
        fraction
            .parse::<u64>()
            .map_err(|_| invalid_amount("ICP", value))?
            .checked_mul(10_u64.pow(padding))
            .ok_or_else(|| IcpBalanceError::IcpAmountOverflow {
                value: value.to_string(),
            })?
    };
    whole
        .checked_mul(E8S_PER_ICP)
        .and_then(|whole_e8s| whole_e8s.checked_add(fractional_e8s))
        .ok_or_else(|| IcpBalanceError::IcpAmountOverflow {
            value: value.to_string(),
        })
}

fn strip_unit<'a>(value: &'a str, unit: &'static str) -> Result<&'a str, IcpBalanceError> {
    value
        .trim()
        .strip_suffix(unit)
        .map(str::trim)
        .filter(|amount| !amount.is_empty())
        .ok_or_else(|| invalid_amount(unit, value))
}

fn invalid_amount(unit: &'static str, value: &str) -> IcpBalanceError {
    IcpBalanceError::InvalidAmount {
        unit,
        value: value.to_string(),
    }
}

// -----------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    const ICP_CLI_1_3_CYCLES_BALANCE_JSON: &str = r#"{"balance":"3_519_900_000_000 cycles"}"#;
    const ICP_CLI_1_3_TOKEN_BALANCE_JSON: &str = r#"{"balance":"1.23456780 ICP"}"#;

    #[test]
    fn decodes_icp_cli_one_three_balance_json_goldens() {
        let cycles: BalanceOutput = serde_json::from_str(ICP_CLI_1_3_CYCLES_BALANCE_JSON)
            .expect("ICP CLI 1.3 cycles balance JSON");
        let token: BalanceOutput = serde_json::from_str(ICP_CLI_1_3_TOKEN_BALANCE_JSON)
            .expect("ICP CLI 1.3 token balance JSON");

        assert_eq!(parse_cycles(&cycles.balance).unwrap(), 3_519_900_000_000);
        assert_eq!(parse_icp_e8s(&token.balance).unwrap(), 123_456_780);
    }

    #[test]
    fn parses_exact_cycles_ledger_amounts() {
        assert_eq!(
            parse_cycles("3_519_900_000_000 cycles").unwrap(),
            3_519_900_000_000
        );
        assert_eq!(parse_cycles("0 cycles").unwrap(), 0);
        assert!(matches!(
            parse_cycles("1.5 cycles"),
            Err(IcpBalanceError::InvalidAmount { unit: "cycles", .. })
        ));
    }

    #[test]
    fn parses_exact_icp_amounts_into_e8s() {
        assert_eq!(parse_icp_e8s("1.23456780 ICP").unwrap(), 123_456_780);
        assert_eq!(parse_icp_e8s("1 ICP").unwrap(), 100_000_000);
        assert_eq!(parse_icp_e8s("0.1 ICP").unwrap(), 10_000_000);
        assert!(matches!(
            parse_icp_e8s("0.000000001 ICP"),
            Err(IcpBalanceError::InvalidAmount { unit: "ICP", .. })
        ));
    }
}
