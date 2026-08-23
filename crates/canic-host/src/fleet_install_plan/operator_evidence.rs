//! Module: fleet_install_plan::operator_evidence
//!
//! Responsibility: obtain live pre-effect identity funding evidence through ICP CLI.
//! Does not own: Fleet-input authority, funding requirements, or sufficiency policy.
//! Boundary: compares the active identity with explicit operator authority before observation.

use crate::{
    fleet_install_plan::{FreshFleetOperatorFundingEvidenceV1, PlannedCanisterCreationFunding},
    icp::{IcpBalanceError, IcpCli, IcpCommandError, IcpIdentityAccountFormat},
};
use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};

use candid::Principal;
use thiserror::Error as ThisError;

const OPERATOR_BALANCE_VALIDITY_SECS: u64 = 300;

///
/// FreshFleetOperatorEvidenceError
///
/// Typed failure while obtaining live operator identity and funding evidence.
///
#[derive(Debug, ThisError)]
pub enum FreshFleetOperatorEvidenceError {
    #[error("active ICP identity account could not be resolved: {source}")]
    Account {
        #[source]
        source: IcpCommandError,
    },

    #[error("active ICP identity must not be anonymous")]
    AnonymousIdentity,

    #[error("active ICP identity balance could not be observed: {source}")]
    Balance {
        #[source]
        source: IcpBalanceError,
    },

    #[error("system clock is before the Unix epoch: {0}")]
    Clock(#[from] SystemTimeError),

    #[error("active ICP identity account is empty")]
    EmptyAccount,

    #[error(
        "active ICP identity could not be resolved: {source}; for an encrypted identity in non-interactive execution, set CANIC_ICP_IDENTITY_PASSWORD_FILE to an absolute operator-owned password file"
    )]
    Identity {
        #[source]
        source: IcpCommandError,
    },

    #[error("Fleet input operator principal is invalid: {value}")]
    InvalidExpectedPrincipal { value: String },

    #[error("active ICP identity returned an invalid principal: {value}")]
    InvalidObservedPrincipal { value: String },

    #[error(
        "active ICP installation identity {observed} differs from Fleet input operator principal {expected}"
    )]
    OperatorMismatch {
        expected: String,
        observed: Principal,
    },

    #[error("operator balance observation validity overflowed")]
    ValidityOverflow,
}

/// Observe one exact operator account and its current balance before effects.
pub fn observe_fresh_fleet_operator_funding(
    icp: &IcpCli,
    expected_principal: &str,
    maximum_debit: &PlannedCanisterCreationFunding,
) -> Result<FreshFleetOperatorFundingEvidenceV1, FreshFleetOperatorEvidenceError> {
    let expected = Principal::from_text(expected_principal).map_err(|_| {
        FreshFleetOperatorEvidenceError::InvalidExpectedPrincipal {
            value: expected_principal.to_string(),
        }
    })?;
    if expected == Principal::anonymous() || expected.to_text() != expected_principal {
        return Err(FreshFleetOperatorEvidenceError::InvalidExpectedPrincipal {
            value: expected_principal.to_string(),
        });
    }

    let observed_text = icp
        .identity_principal_text()
        .map_err(|source| FreshFleetOperatorEvidenceError::Identity { source })?;
    let observed = Principal::from_text(&observed_text).map_err(|_| {
        FreshFleetOperatorEvidenceError::InvalidObservedPrincipal {
            value: observed_text.clone(),
        }
    })?;
    if observed == Principal::anonymous() {
        return Err(FreshFleetOperatorEvidenceError::AnonymousIdentity);
    }
    if observed != expected {
        return Err(FreshFleetOperatorEvidenceError::OperatorMismatch {
            expected: expected_principal.to_string(),
            observed,
        });
    }

    let (format, balance, source) = match maximum_debit {
        PlannedCanisterCreationFunding::Cycles { .. } => (
            IcpIdentityAccountFormat::Icrc1,
            PlannedCanisterCreationFunding::Cycles {
                cycles: icp
                    .identity_cycles_balance()
                    .map_err(|source| FreshFleetOperatorEvidenceError::Balance { source })?,
            },
            "icp_cli_cycles_ledger",
        ),
        PlannedCanisterCreationFunding::Icp { .. } => (
            IcpIdentityAccountFormat::IcpLedger,
            PlannedCanisterCreationFunding::Icp {
                e8s: icp
                    .identity_icp_balance_e8s()
                    .map_err(|source| FreshFleetOperatorEvidenceError::Balance { source })?,
            },
            "icp_cli_icp_ledger",
        ),
    };
    let funding_account = icp
        .identity_account_id_text(format)
        .map_err(|source| FreshFleetOperatorEvidenceError::Account { source })?;
    if funding_account.is_empty() {
        return Err(FreshFleetOperatorEvidenceError::EmptyAccount);
    }
    let observed_at_unix_secs = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let valid_until_unix_secs = observed_at_unix_secs
        .checked_add(OPERATOR_BALANCE_VALIDITY_SECS)
        .ok_or(FreshFleetOperatorEvidenceError::ValidityOverflow)?;

    Ok(FreshFleetOperatorFundingEvidenceV1 {
        principal: expected_principal.to_string(),
        funding_account,
        balance,
        source: source.to_string(),
        observed_at_unix_secs,
        valid_until_unix_secs,
        balance_fresh: true,
    })
}

// -----------------------------------------------------------------------------
// Tests

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;
    use std::{fs, os::unix::fs::PermissionsExt};

    const OPERATOR: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";

    #[test]
    fn observes_the_active_operator_account_and_live_cycles_balance() {
        let root = temp_dir("canic-operator-funding-evidence");
        fs::create_dir_all(&root).expect("create temp root");
        let executable = write_fake_icp(&root);
        let icp =
            IcpCli::new(executable.to_string_lossy(), Some("local".to_string())).with_cwd(&root);

        let evidence = observe_fresh_fleet_operator_funding(
            &icp,
            OPERATOR,
            &PlannedCanisterCreationFunding::Cycles {
                cycles: 140_000_000_000_000,
            },
        )
        .expect("observe funded operator");

        assert_eq!(evidence.principal, OPERATOR);
        assert_eq!(evidence.funding_account, "operator-cycles-account");
        assert_eq!(
            evidence.balance,
            PlannedCanisterCreationFunding::Cycles {
                cycles: 999_000_000_000_000,
            }
        );
        assert_eq!(evidence.source, "icp_cli_cycles_ledger");
        assert_eq!(
            evidence.valid_until_unix_secs,
            evidence.observed_at_unix_secs + OPERATOR_BALANCE_VALIDITY_SECS
        );
        assert!(evidence.balance_fresh);

        fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn parses_the_icp_cli_one_three_token_balance_variant() {
        let root = temp_dir("canic-operator-icp-funding-evidence");
        fs::create_dir_all(&root).expect("create temp root");
        let executable = write_fake_icp(&root);
        let icp = IcpCli::new(executable.to_string_lossy(), Some("ic".to_string())).with_cwd(&root);

        let evidence = observe_fresh_fleet_operator_funding(
            &icp,
            OPERATOR,
            &PlannedCanisterCreationFunding::Icp { e8s: 1 },
        )
        .expect("observe ICP CLI 1.3 token balance variant");

        assert_eq!(evidence.funding_account, "operator-icp-account");
        assert_eq!(
            evidence.balance,
            PlannedCanisterCreationFunding::Icp { e8s: 123_456_789 }
        );
        assert_eq!(evidence.source, "icp_cli_icp_ledger");

        fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn rejects_an_active_identity_that_differs_from_operator_authority() {
        let root = temp_dir("canic-operator-funding-mismatch");
        fs::create_dir_all(&root).expect("create temp root");
        let executable = write_fake_icp(&root);
        let icp =
            IcpCli::new(executable.to_string_lossy(), Some("local".to_string())).with_cwd(&root);
        let expected = Principal::from_slice(&[9; 29]).to_text();

        let error = observe_fresh_fleet_operator_funding(
            &icp,
            &expected,
            &PlannedCanisterCreationFunding::Cycles { cycles: 1 },
        )
        .expect_err("different active identity must reject");

        assert!(matches!(
            error,
            FreshFleetOperatorEvidenceError::OperatorMismatch { .. }
        ));
        fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn rejects_the_anonymous_active_identity() {
        let root = temp_dir("canic-operator-funding-anonymous");
        fs::create_dir_all(&root).expect("create temp root");
        let executable = write_identity_only_fake_icp(&root, Ok("2vxsx-fae"));
        let icp =
            IcpCli::new(executable.to_string_lossy(), Some("local".to_string())).with_cwd(&root);

        let error = observe_fresh_fleet_operator_funding(
            &icp,
            OPERATOR,
            &PlannedCanisterCreationFunding::Cycles { cycles: 1 },
        )
        .expect_err("anonymous identity must reject");

        assert!(matches!(
            error,
            FreshFleetOperatorEvidenceError::AnonymousIdentity
        ));
        fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn explains_noninteractive_encrypted_identity_setup() {
        let root = temp_dir("canic-operator-funding-locked");
        fs::create_dir_all(&root).expect("create temp root");
        let executable = write_identity_only_fake_icp(
            &root,
            Err("identity is encrypted and password input is unavailable"),
        );
        let icp =
            IcpCli::new(executable.to_string_lossy(), Some("local".to_string())).with_cwd(&root);

        let error = observe_fresh_fleet_operator_funding(
            &icp,
            OPERATOR,
            &PlannedCanisterCreationFunding::Cycles { cycles: 1 },
        )
        .expect_err("unusable encrypted identity must reject");
        assert!(matches!(
            error,
            FreshFleetOperatorEvidenceError::Identity { .. }
        ));
        let detail = error.to_string();
        assert!(detail.contains("identity is encrypted"));
        assert!(detail.contains("CANIC_ICP_IDENTITY_PASSWORD_FILE"));
        fs::remove_dir_all(root).expect("remove temp root");
    }

    fn write_fake_icp(root: &std::path::Path) -> std::path::PathBuf {
        let executable = root.join("icp");
        fs::write(
            &executable,
            r#"#!/bin/sh
case "$*" in
  "--version")
    printf '%s\n' 'icp 1.3.0'
    ;;
  *"identity principal"*)
    printf '%s\n' 'ryjl3-tyaaa-aaaaa-aaaba-cai'
    ;;
  *"cycles balance --json"*)
    printf '%s\n' '{"balance":"999_000_000_000_000 cycles"}'
    ;;
  *"token balance --json"*)
    printf '%s\n' '{"balance":"1.23456789 ICP"}'
    ;;
  *"identity account-id --format icrc1"*)
    printf '%s\n' 'operator-cycles-account'
    ;;
  *"identity account-id --format ledger"*)
    printf '%s\n' 'operator-icp-account'
    ;;
  *)
    printf '%s\n' "unexpected ICP command: $*" >&2
    exit 1
    ;;
esac
"#,
        )
        .expect("write fake ICP executable");
        let mut permissions = fs::metadata(&executable)
            .expect("fake ICP metadata")
            .permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&executable, permissions).expect("make fake ICP executable");
        executable
    }

    fn write_identity_only_fake_icp(
        root: &std::path::Path,
        identity: Result<&str, &str>,
    ) -> std::path::PathBuf {
        let executable = root.join("icp-identity-only");
        let identity_command = match identity {
            Ok(principal) => format!("printf '%s\\n' '{principal}'\nexit 0"),
            Err(detail) => format!("printf '%s\\n' '{detail}' >&2\nexit 1"),
        };
        fs::write(
            &executable,
            format!(
                "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  printf '%s\\n' 'icp 1.3.0'\n  exit 0\nfi\n{identity_command}\n"
            ),
        )
        .expect("write identity-only fake ICP executable");
        let mut permissions = fs::metadata(&executable)
            .expect("identity-only fake ICP metadata")
            .permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&executable, permissions)
            .expect("make identity-only fake ICP executable");
        executable
    }
}
