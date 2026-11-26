//! Native command codec. Canic owns signing and transport; this process only handles bytes.

use canic::method::Contract;
use clap::{Parser, Subcommand};
use example_counter::CounterMethod;
use std::io::Read as _;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Encode one counter operation, with no network effects.
    Encode {
        #[command(subcommand)]
        command: Counter,
    },
    /// Decode a retained counter reply from stdin.
    Decode,
}

#[derive(Subcommand)]
enum Counter {
    /// Increase the persistent value once.
    Increment,
    /// Read the persistent value.
    Read,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = match Args::parse().action {
        Action::Encode { command } => {
            let request = match command {
                Counter::Increment => example_counter::Request::Increment,
                Counter::Read => example_counter::Request::Read,
            };
            let arguments = CounterMethod::encode_request(&request)
                .map_err(|e| format!("counter request refused: {e:?}"))?;

            serde_json::json!({"method": CounterMethod::NAME, "arguments": arguments})
        }
        Action::Decode => {
            let mut bytes = Vec::new();
            std::io::stdin()
                .take(CounterMethod::REPLY.bytes as u64 + 1)
                .read_to_end(&mut bytes)?;
            let reply = CounterMethod::decode_reply(&bytes)
                .map_err(|e| format!("counter reply refused: {e:?}"))?;

            serde_json::to_value(reply)?
        }
    };
    println!("{result}");

    Ok(())
}
