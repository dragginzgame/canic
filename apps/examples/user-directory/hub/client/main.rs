//! Native codec for direct administration of a user-directory hub.
use canic::{
    method::Contract,
    sharding::{Allocation, BeginShardDrain, ConfigureShardRoutes, ObserveShard, ShardTarget},
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
    /// Set the immutable number of live keys allowed on each shard of this pool.
    ConfigureShardRoutes { pool: String, keys_per_shard: u32 },
    /// Stop new placements on the exact allocation returned by observe-shard.
    DrainShard {
        shard: Principal,
        slot: u32,
        #[arg(value_parser = generation)]
        generation: [u8; 32],
    },
    /// Report the allocation, drain state and live placement count.
    ObserveShard { shard: Principal },
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
            command:
                Command::ConfigureShardRoutes {
                    pool,
                    keys_per_shard,
                },
        } => request::<ConfigureShardRoutes>(&(pool, keys_per_shard))?,
        Action::Encode {
            command: Command::ObserveShard { shard },
        } => request::<ObserveShard>(&shard)?,

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
            std::io::stdin().read_to_end(&mut bytes)?;
            decode(&bytes)?
        }
    };
    println!("{}", serde_json::to_string(&value)?);
    Ok(())
}
fn decode(bytes: &[u8]) -> Result<Value, Box<dyn std::error::Error>> {
    if let Ok(reply) = ConfigureShardRoutes::decode_reply(bytes) {
        reply?;
        return Ok(json!({"configured":true}));
    }
    if let Ok(reply) = ObserveShard::decode_reply(bytes) {
        let status = reply?;
        let mut generation = String::with_capacity(64);
        for b in status.target.allocation.generation {
            write!(generation, "{b:02x}")?;
        }

        return Ok(json!({
            "shard": status.target.shard.to_text(),
            "slot": status.target.allocation.slot,
            "generation": generation,
            "draining": status.draining,
            "live_placements": status.live_placements,
        }));
    }

    if let Ok(reply) = BeginShardDrain::decode_reply(bytes) {
        return Ok(json!({"draining": true, "live_placements": reply?}));
    }
    Err("unknown hub response".into())
}
