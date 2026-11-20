//! Native codec for controller operations on a sharding Root.
use canic::{
    method::Contract,
    sharding::{Allocation, BeginShardDrain, InspectShard, ShardTarget},
};
use clap::{Parser, Subcommand};
use ic_platform_types::Principal;
use serde_json::{Value, json};
use std::{fmt::Write as _, io::Read as _};

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
    /// Stop new placements on the exact allocation returned by inspect-shard.
    DrainShard {
        shard: Principal,
        slot: u32,
        #[arg(value_parser = generation)]
        generation: [u8; 32],
    },
    /// Report the allocation, drain state and live placement count.
    InspectShard { shard: Principal },
}

fn generation(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("generation must contain 64 hex digits".into());
    }
    let mut bytes = [0; 32];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string())?;
    }
    Ok(bytes)
}

fn request<M: Contract>(input: &M::Input) -> Result<Value, String> {
    let arguments =
        M::encode_request(input).map_err(|e| format!("invalid {} request: {e:?}", M::NAME))?;
    Ok(json!({"method":M::NAME, "arguments":arguments}))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value = match Args::parse().action {
        Action::Encode {
            command: Command::InspectShard { shard },
        } => request::<InspectShard>(&shard)?,
        Action::Encode {
            command:
                Command::DrainShard {
                    shard,
                    slot,
                    generation,
                },
        } => request::<BeginShardDrain>(&ShardTarget {
            shard,
            allocation: Allocation { slot, generation },
        })?,
        Action::Decode => {
            let mut bytes = Vec::new();
            std::io::stdin()
                .take((canic::sharding::LIMITS.bytes + 1) as u64)
                .read_to_end(&mut bytes)?;
            if let Ok(reply) = InspectShard::decode_reply(&bytes) {
                let status = reply?;
                let mut generation = String::with_capacity(64);
                for b in status.target.allocation.generation {
                    write!(generation, "{b:02x}")?;
                }
                json!({"shard":status.target.shard.to_text(),"slot":status.target.allocation.slot,"generation":generation,"draining":status.draining,"live_placements":status.live_placements})
            } else {
                let count = BeginShardDrain::decode_reply(&bytes)
                    .map_err(|e| format!("invalid shard reply: {e:?}"))??;
                json!({"draining":true,"live_placements":count})
            }
        }
    };
    println!("{value}");
    Ok(())
}
