# Canic Configuration

This document specifies the canonical shape of the `canic.toml` configuration file that Canic-based deployments consume.

## Top-Level Keys

- `controllers: [Principal]` – optional list of controller principals.
- `pool` – optional table describing shared shard pool defaults.
- `canisters` – table keyed by canister type identifiers.
- `whitelist` – optional table listing principals allowed to access privileged operations.
- `standards` – optional table for feature switches tied to interface standards.

## `[pool]`

- `minimum_size: u8` – minimum number of shards kept warm for assignment (default `0`).

## `[canisters.<type>]`

Each entry configures one logical canister type.

- `initial_cycles: string` – required amount of cycles to provision (e.g., `"6T"`).
- `uses_directory: bool` – whether the canister participates in directory lookups.
- `auto_create: bool` – create an instance automatically during root initialization.
- `delegation: bool` – expose delegation endpoints for this canister type.
- `topup.threshold: string` – optional minimum balance before top-up (default `"10T"`).
- `topup.amount: string` – optional refill amount (default `"5T"`).
- `sharder` – optional table describing how this type allocates work to shard pools.

### `[canisters.<type>.sharder]`

- `pools.<name>.canister_type: string` – required; target shard canister type.
- `pools.<name>.policy.initial_capacity: u32` – initial number of shards to create.
- `pools.<name>.policy.max_shards: u32` – hard cap on shard count.
- `pools.<name>.policy.growth_threshold_pct: u32` – percentage utilization that triggers expansion.

## `[whitelist]`

- `principals: [Principal]` – principals permitted for operations gated by the whitelist.

## `[standards]`

- `icrc21: bool` – enable ICRC-21 metadata exposure (default `false`).

## Example

```toml
controllers = ["aaaaa-aa"]

[pool]
minimum_size = 10

[canisters.example]
initial_cycles = "6T"
uses_directory = false

[canisters.example.topup]
threshold = "10T"
amount = "5T"

[standards]
icrc21 = true
```
