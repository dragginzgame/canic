//! Native command codec. Canic owns signing and transport; this process only handles bytes.
use clap::{Parser, Subcommand};
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
                Counter::Increment => ex_counter::Request::Increment,
                Counter::Read => ex_counter::Request::Read,
            };
            let arguments = ic_wire::encode(
                &request,
                ic_wire::Encoding::Frame {
                    method: ex_counter::METHOD,
                    direction: ic_wire::Direction::Request,
                },
                ex_counter::LIMITS,
            )
            .map_err(|e| format!("counter request refused: {e:?}"))?;
            serde_json::json!({"method": "counter", "arguments": arguments})
        }
        Action::Decode => {
            let mut bytes = Vec::new();
            std::io::stdin()
                .take(ex_counter::LIMITS.bytes as u64 + 1)
                .read_to_end(&mut bytes)?;
            let reply: Result<ex_counter::Snapshot, ex_counter::Error> = ic_wire::decode(
                &bytes,
                ic_wire::Encoding::Frame {
                    method: ex_counter::METHOD,
                    direction: ic_wire::Direction::Response,
                },
                ex_counter::LIMITS,
            )
            .map_err(|e| format!("counter reply refused: {e:?}"))?;
            serde_json::to_value(reply)?
        }
    };
    println!("{result}");
    Ok(())
}
