//! Native codec for one operator tick on the installed miner pool.
use canic::method::Contract;
use clap::{Parser, Subcommand};
use ex_miner_pool_hub::Tick;
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
            command: Command::Tick { round, goal },
        } => {
            let arguments = Tick::encode_request(&(round, goal))
                .map_err(|e| format!("invalid tick request: {e:?}"))?;
            serde_json::json!({"method": Tick::NAME, "arguments": arguments})
        }
        Action::Decode => {
            let mut bytes = Vec::new();
            std::io::stdin()
                .take(Tick::REPLY.bytes as u64 + 1)
                .read_to_end(&mut bytes)?;
            let reply =
                Tick::decode_reply(&bytes).map_err(|e| format!("invalid tick reply: {e:?}"))?;
            serde_json::Value::String(reply)
        }
    };
    println!("{output}");
    Ok(())
}
