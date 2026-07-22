use crate::{
    cli::{
        clap::{flag_arg, parse_matches, render_usage, required_string, typed_option, value_arg},
        globals::{internal_environment_arg, internal_icp_arg},
    },
    cycles::{CyclesCommandError, wallet::IcpTargetOptions},
};
use canic_core::cdk::utils::hash::decode_hex;
use clap::Command as ClapCommand;
use std::ffi::OsString;

const DEPLOYMENT_ARG: &str = "deployment";
const DRY_RUN_ARG: &str = "dry-run";
const FROM_SUBACCOUNT_ARG: &str = "from-subaccount";
const ICP_E8S_ARG: &str = "icp-e8s";
const JSON_ARG: &str = "json";
const OPERATION_ID_ARG: &str = "operation-id";

///
/// ConvertOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ConvertOptions {
    pub(super) target: IcpTargetOptions,
    pub(super) deployment: String,
    pub(super) amount_e8s: u64,
    pub(super) source_subaccount: Option<[u8; 32]>,
    pub(super) operation_id: Option<[u8; 32]>,
    pub(super) json: bool,
    pub(super) dry_run: bool,
}

impl ConvertOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, CyclesCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| CyclesCommandError::Usage(usage()))?;
        Ok(Self {
            target: IcpTargetOptions::parse(&matches),
            deployment: required_string(&matches, DEPLOYMENT_ARG),
            amount_e8s: typed_option(&matches, ICP_E8S_ARG)
                .expect("required ICP amount is present after Clap validation"),
            source_subaccount: typed_option(&matches, FROM_SUBACCOUNT_ARG),
            operation_id: typed_option(&matches, OPERATION_ID_ARG),
            json: matches.get_flag(JSON_ARG),
            dry_run: matches.get_flag(DRY_RUN_ARG),
        })
    }
}

pub(super) fn usage() -> String {
    render_usage(command)
}

fn parse_icp_e8s_amount(value: &str) -> Result<u64, String> {
    let compact = value.trim().replace('_', "");
    compact
        .parse::<u64>()
        .ok()
        .filter(|amount| *amount > 0)
        .ok_or_else(|| format!("invalid ICP e8s amount {value}; use a positive u64 e8s value"))
}

fn parse_fixed_32_hex(field: &'static str, value: &str) -> Result<[u8; 32], String> {
    let trimmed = value.trim();
    let bytes = decode_hex(trimmed).map_err(|err| format!("invalid {field}: {err}"))?;
    <[u8; 32]>::try_from(bytes.as_slice()).map_err(|_| {
        format!(
            "invalid {field}: expected 32 bytes (64 hex chars), got {} bytes",
            bytes.len()
        )
    })
}

fn command() -> ClapCommand {
    ClapCommand::new("convert")
        .bin_name("canic cycles convert")
        .about("Convert ICP held by an installed deployment root to cycles for that root")
        .disable_help_flag(true)
        .arg(
            value_arg(DEPLOYMENT_ARG)
                .value_name(DEPLOYMENT_ARG)
                .required(true),
        )
        .arg(
            value_arg(ICP_E8S_ARG)
                .long(ICP_E8S_ARG)
                .value_name("e8s")
                .value_parser(clap::builder::ValueParser::new(parse_icp_e8s_amount))
                .required(true),
        )
        .arg(
            value_arg(FROM_SUBACCOUNT_ARG)
                .long(FROM_SUBACCOUNT_ARG)
                .value_name("hex64")
                .value_parser(clap::builder::ValueParser::new(|value: &str| {
                    parse_fixed_32_hex(FROM_SUBACCOUNT_ARG, value)
                })),
        )
        .arg(
            value_arg(OPERATION_ID_ARG)
                .long(OPERATION_ID_ARG)
                .value_name("hex64")
                .value_parser(clap::builder::ValueParser::new(|value: &str| {
                    parse_fixed_32_hex(OPERATION_ID_ARG, value)
                })),
        )
        .arg(flag_arg(JSON_ARG).long(JSON_ARG))
        .arg(flag_arg(DRY_RUN_ARG).long(DRY_RUN_ARG))
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::cdk::utils::hash::hex_bytes;

    // Keep canister-side ICP conversion as a thin endpoint caller.
    #[test]
    fn parses_root_refill_options() {
        let operation_id = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        let subaccount = "202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f";
        let options = ConvertOptions::parse([
            OsString::from("demo"),
            OsString::from("--icp-e8s"),
            OsString::from("100_000_000"),
            OsString::from("--from-subaccount"),
            OsString::from(subaccount),
            OsString::from("--operation-id"),
            OsString::from(operation_id),
            OsString::from("--dry-run"),
            OsString::from("--json"),
        ])
        .expect("parse convert");

        assert_eq!(options.deployment, "demo");
        assert_eq!(options.amount_e8s, 100_000_000);
        assert_eq!(
            options.operation_id.map(hex_bytes).as_deref(),
            Some(operation_id)
        );
        assert_eq!(
            options.source_subaccount.map(hex_bytes).as_deref(),
            Some(subaccount)
        );
        assert!(options.dry_run);
        assert!(options.json);
    }

    #[test]
    fn rejects_missing_icp_amount() {
        std::assert_matches!(
            ConvertOptions::parse([OsString::from("demo")]),
            Err(CyclesCommandError::Usage(_))
        );
    }

    #[test]
    fn rejects_non_32_byte_hex() {
        std::assert_matches!(
            ConvertOptions::parse([
                OsString::from("demo"),
                OsString::from("--icp-e8s"),
                OsString::from("1"),
                OsString::from("--operation-id"),
                OsString::from("abcd"),
            ]),
            Err(CyclesCommandError::Usage(_))
        );
    }
}
