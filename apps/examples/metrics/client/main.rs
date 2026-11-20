//! Application codec for the metrics example's two signed fleet calls.

use canic::method::Contract;
use clap::{Parser, Subcommand};
use ex_metrics::{Checksum, Payload, Ping};
use serde_json::json;
use std::io::Read as _;

///
/// Args
/// Native codec invocation by the fleet CLI.
///

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    action: Action,
}

///
/// Action
/// Selects encoding a command or decoding its reply.
///

#[derive(Subcommand)]
enum Action {
    Encode {
        #[command(subcommand)]
        command: Command,
    },
    Decode,
}

///
/// Command
/// Names the two calls exposed by this example.
///

#[derive(Subcommand)]
enum Command {
    /// Record one ping.
    Ping {
        #[arg(default_value = "")]
        payload: String,
    },
    /// Compute a checksum.
    Checksum { payload: String },
}

fn request<M: Contract>(input: &M::Input) -> Result<serde_json::Value, String> {
    let bytes = M::encode_request(input).map_err(|e| format!("encode request: {e:?}"))?;

    Ok(json!({"method": M::NAME, "arguments": bytes}))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value = match Args::parse().action {
        Action::Encode {
            command: Command::Ping { payload },
        } => request::<Ping>(
            &Payload::new(payload).map_err(|_| "ping accepts up to 256 UTF-8 bytes")?,
        )?,
        Action::Encode {
            command: Command::Checksum { payload },
        } => request::<Checksum>(
            &Payload::new(payload).map_err(|_| "checksum accepts up to 256 UTF-8 bytes")?,
        )?,
        Action::Decode => {
            let mut bytes = Vec::new();
            std::io::stdin()
                .take(Ping::REPLY.bytes.max(Checksum::REPLY.bytes) as u64 + 1)
                .read_to_end(&mut bytes)?;
            let metrics = Checksum::decode_reply(&bytes)
                .or_else(|_| Ping::decode_reply(&bytes))
                .map_err(|e| format!("invalid metrics reply: {e:?}"))?;

            serde_json::to_value(metrics)?
        }
    };
    println!("{value}");

    Ok(())
}
