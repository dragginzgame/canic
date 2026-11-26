//! Native codec for one operator tick on the installed miner pool.
use canic::method::Contract;
use clap::{Parser, Subcommand};
use example_miner_pool_hub::{PoolReport, Release, Tick};
use std::io::Read as _;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    Encode {
        #[command(subcommand)]
        command: Command,
    },
    Decode,
}

#[derive(Subcommand)]
enum Command {
    /// Read the current workers and their original allocations.
    Report,
    /// Release an exact parked worker target (JSON).
    ReleaseWorker { target: String },
    /// Advance one step toward the goal for this round.
    Tick {
        #[arg(value_parser = clap::value_parser!(u64).range(1..))]
        round: u64,
        goal: u8,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = match Args::parse().action {
        Action::Encode {
            command: Command::Report,
        } => {
            serde_json::json!({"method": PoolReport::NAME, "arguments": PoolReport::encode_request(&()).expect("empty request")})
        }
        Action::Encode {
            command: Command::ReleaseWorker { target },
        } => {
            let args = Release::encode_request(&serde_json::from_str(&target)?)
                .map_err(|e| format!("release request: {e:?}"))?;
            serde_json::json!({"method": Release::NAME, "arguments": args})
        }
        Action::Encode {
            command: Command::Tick { round, goal },
        } => {
            let arguments = Tick::encode_request(&(round, goal))
                .map_err(|e| format!("invalid tick request: {e:?}"))?;
            serde_json::json!({"method": Tick::NAME, "arguments": arguments})
        }
        Action::Decode => {
            let mut bytes = Vec::new();
            std::io::stdin()
                .take(PoolReport::REPLY.bytes as u64 + 1)
                .read_to_end(&mut bytes)?;
            if let Ok(report) = PoolReport::decode_reply(&bytes) {
                serde_json::to_value(report)?
            } else if let Ok(reply) = Release::decode_reply(&bytes) {
                reply?;
                serde_json::json!({"released": true})
            } else {
                serde_json::Value::String(
                    Tick::decode_reply(&bytes).map_err(|e| format!("unknown pool reply: {e:?}"))?,
                )
            }
        }
    };
    println!("{output}");
    Ok(())
}
