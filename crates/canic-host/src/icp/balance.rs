//! Module: icp::balance
//!
//! Responsibility: observe the selected identity's default cycles account.
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
        let mut command = self.request_command();
        command.args(["cycles", "balance", "--json"]);
        self.add_target_args(&mut command);
        self.record_remote_call();
        let output = run_json::<BalanceOutput>(&mut command, self)?;
        parse_cycles(&output.balance)
    }
}

fn parse_cycles(value: &str) -> Result<u128, IcpBalanceError> {
    let amount = strip_unit(value, "cycles")?;
    amount
        .replace('_', "")
        .parse()
        .map_err(|_| invalid_amount("cycles", value))
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

    const ICP_CLI_1_5_CYCLES_BALANCE_JSON: &str = r#"{"balance":"3_519_900_000_000 cycles"}"#;

    #[test]
    fn decodes_icp_cli_one_five_balance_json_goldens() {
        let cycles: BalanceOutput = serde_json::from_str(ICP_CLI_1_5_CYCLES_BALANCE_JSON)
            .expect("ICP CLI 1.5 cycles balance JSON");
        assert_eq!(parse_cycles(&cycles.balance).unwrap(), 3_519_900_000_000);
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
}
