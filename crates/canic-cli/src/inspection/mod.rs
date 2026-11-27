//! Read-only Canic inspection. Applications retain query/write and exposure policy.

#[cfg(test)]
mod tests;

use canic::db::{RPC_LIMITS, inspection as protocol};
use canic_host::{Error, InspectionClient};
use clap::{Args, Subcommand};
use ic_host_text::{bordered_table, escape};
use ic_wire::{Direction, Encoding};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Subcommand)]
pub(super) enum DbCommand {
    /// Read the executable schema fingerprint and stored database admission.
    Inspect(Options),
}

#[derive(Args)]
pub(super) struct Options {
    #[command(flatten)]
    network: crate::args::NetworkArgs,
    /// Explicit target canister principal.
    #[arg(long)]
    canister: String,
    /// Make the query as the anonymous principal.
    #[arg(
        long,
        required_unless_present = "identity",
        conflicts_with = "identity"
    )]
    anonymous: bool,
    /// Ed25519 or secp256k1 PEM identity.
    #[arg(long, value_name = "PEM")]
    identity: Option<PathBuf>,
    /// Whole deadline per verified query, in seconds.
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..=300))]
    timeout_secs: u64,
}

fn decode(bytes: &[u8]) -> canic_host::Result<protocol::Response> {
    ic_wire::decode(
        bytes,
        Encoding::Frame {
            method: protocol::INSPECT,
            direction: Direction::Response,
        },
        RPC_LIMITS,
    )
    .map_err(|e| Error::Invalid(format!("invalid Canic database status reply: {e}")))
}

pub(super) async fn run(options: &Options, json: bool) -> canic_host::Result<crate::Output> {
    let network = options.network.clone().into_network()?;
    let client = InspectionClient::new(&network, &options.canister, options.identity.as_deref())?;
    let payload = ic_wire::encode(
        &(),
        Encoding::Frame {
            method: protocol::INSPECT,
            direction: Direction::Request,
        },
        RPC_LIMITS,
    )
    .map_err(|e| Error::Invalid(e.to_string()))?;
    let start = Instant::now();
    let result = client
        .inspect(&payload, Duration::from_secs(options.timeout_secs))
        .await
        .and_then(|bytes| decode(&bytes));
    let timing = format!(
        "query call (canic_db_inspect): {:.2} sec, cycles unavailable",
        start.elapsed().as_secs_f64()
    );
    if json || result.is_err() {
        eprintln!("{timing}");
    }
    let response = result?;
    let provider = network.url();
    if json {
        Ok(crate::Output::Json(
            serde_json::json!({"provider":provider, "canister":options.canister, "caller":client.caller().to_text(), "inspection":response}),
        ))
    } else {
        Ok(crate::Output::Text(format!(
            "APPLICATION INSPECTION\n{}\n{}{}\n",
            bordered_table(
                &["Target", "Value"],
                &[
                    vec!["Provider".into(), escape(provider).into_owned()],
                    vec!["Canister".into(), options.canister.clone()],
                    vec!["Caller".into(), client.caller().to_text()]
                ]
            ),
            render(&response),
            timing
        )))
    }
}

fn render(response: &protocol::Response) -> String {
    let (admission, incarnation) = match response.admission {
        protocol::Admission::Readable { incarnation } => {
            ("Readable", incarnation.map(|v| format!("{v:02x}")).concat())
        }
        protocol::Admission::Maintenance => ("Maintenance", "—".into()),
        protocol::Admission::Incompatible => ("Incompatible", "—".into()),
        protocol::Admission::Unavailable => ("Unavailable", "—".into()),
    };
    format!(
        "{}\n",
        bordered_table(
            &["Property", "Value"],
            &[
                vec![
                    "Executable schema".into(),
                    response.schema.map(|v| format!("{v:02x}")).concat()
                ],
                vec!["Stored admission".into(), admission.into()],
                vec!["Incarnation".into(), incarnation],
            ]
        )
    )
}
