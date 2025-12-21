# Canic Configuration

This guide documents the canonical shape of `canic.toml`, the configuration file consumed by Canic build scripts and runtime helpers.

At a high level the file describes:

- Global cluster settings (`controllers`, `app_directory`, `pool`, `standards`, `whitelist`).
- Subnet-specific behaviour under `subnets.<name>`.
- Per-canister policies inside each subnet, with optional scaling and sharding pools.

All fields are validated when `canic::build!` or `canic::build_root!` run, so configuration drift fails fast at compile time.

---

## Runtime Config + Env Lifecycle

Canic treats config/env identity as startup invariants. Missing data is a fatal error.

- Build time: `CANIC_CONFIG_PATH` is embedded into the Wasm and `DFX_NETWORK` is baked in.
- Init/post-upgrade: `__canic_load_config!()` loads the embedded TOML; `ConfigOps::current_*` is infallible.
- Root env: `root_init(identity)` sets base env fields, then `root_set_subnet_id()` resolves the real subnet on IC.
  - On IC, registry lookup failure traps.
  - On local/PocketIC, it falls back to `self` as the subnet principal.
- Non-root env: children must receive a complete `EnvData` in `CanisterInitPayload` from root.
  - On IC, missing env fields always trap.
  - On local, missing env fields are filled from the embedded `CANISTER_ID_ROOT` (requires `dfx` builds).

---

## Global Keys

### `controllers = ["aaaaa-aa", ...]`

Optional list of controller principals appended to every provisioned canister.

### `app_directory = ["type_a", "type_b", ...]`

Global set of canister types that should appear in the prime root directory export. Every entry must also exist under `subnets.prime.canisters`.

### `[pool]`

Controls the warm canister pool.

- `minimum_size: u8` – minimum number of spare canisters to keep on hand (default `0`).

### `[log]`

Configure log retention for every canister.

- `max_entries: u64` – ring buffer cap on stored log entries (default `10000`).
- `max_age_secs: u64` – optional maximum age; entries older than this (in seconds) are purged (default `null` = no age limit).

### `[standards]`

Feature toggles tied to public standards.

- `icrc21: bool` – enable the ICRC-21 consent endpoint (default `false`).
- `icrc103: bool` – include ICRC-103 metadata (default `false`).

### `[whitelist]`

Optional allow-list for privileged operations.

- `principals = ["aaaaa-aa", ...]` – principal text strings authorised for whitelist checks.

---

## Subnets

Declare each subnet under `[subnets.<name>]`. The name is an arbitrary identifier; `prime` is reserved for the main orchestrator subnet and should always be present.

### `[subnets.<name>]`

- `auto_create = ["type_a", ...]` – canister types that root should ensure exist during bootstrap.
- `subnet_directory = ["type_a", ...]` – canister types exposed through `canic_subnet_directory()`.
- `canisters.*` – nested tables describing per-type policies (see below).

### `[subnets.<name>.canisters.<type>]`

Each child table configures a logical canister type within the subnet.

- `initial_cycles = "5T"` – cycles to allocate when provisioning (defaults to 5T).
- `topup.threshold = "10T"` – minimum cycles before requesting a top-up (optional).
- `topup.amount = "5T"` – cycles to request when topping up (optional).
- `randomness.enabled = true` – enable PRNG seeding (set `false` to disable).
- `randomness.reseed_interval_secs = 3600` – reseed interval in seconds (default `3600`).
- `randomness.source = "ic"` – seeding source (`ic` or `time`, default `ic`).
  - `time` uses `ic_cdk::api::time()` and is deterministic/low-entropy; use for non-sensitive randomness only.
- `scaling` – optional table that defines stateless worker pools.
- `sharding` – optional table that defines stateful shard pools.

#### Scaling Pools

Scaling pools model interchangeable workers with simple bounds on how many to keep alive.

```
[subnets.<name>.canisters.<type>.scaling.pools.<pool>]
canister_type = "worker_type"
policy.min_workers = 2
policy.max_workers = 16
```

Fields:

- `canister_type` – canister type that represents workers in this pool.
- `policy.min_workers` – minimum workers to keep alive (default `1`).
- `policy.max_workers` – hard cap on workers (default `32`).

#### Sharding Pools

Sharding pools manage stateful shards that own tenant partitions.

```
[subnets.<name>.canisters.<type>.sharding.pools.<pool>]
canister_type = "shard_type"
policy.capacity = 1000
policy.max_shards = 64
```

Fields:

- `canister_type` – canister type that implements the shard.
- `policy.capacity` – per-shard capacity (default `1000`).
- `policy.max_shards` – maximum shard count (default `4`).

### Randomness (Per-Canister)

```
[subnets.<name>.canisters.<type>.randomness]
enabled = true
reseed_interval_secs = 3600
source = "ic" # or "time"
```

Fields:

- `enabled` – enable PRNG seeding (default `true`).
- `reseed_interval_secs` – reseed interval in seconds (default `3600`).
- `source` – `ic` for management canister `raw_rand`, `time` for `ic_cdk::api::time()`.

---

## Example

```toml
controllers = ["aaaaa-aa"]
app_directory = ["scale_hub", "shard_hub"]

[pool]
minimum_size = 3

[standards]
icrc21 = true

[subnets.prime]
auto_create = ["app", "auth", "scale_hub", "shard_hub"]
subnet_directory = ["app", "auth", "scale_hub", "shard_hub"]

[subnets.prime.canisters.scale_hub]
topup.threshold = "10T"
topup.amount = "5T"

[subnets.prime.canisters.scale_hub.scaling.pools.scales]
canister_type = "scale"
policy.min_workers = 2

[subnets.prime.canisters.shard_hub]
topup.threshold = "10T"
topup.amount = "5T"

[subnets.prime.canisters.shard_hub.sharding.pools.shards]
canister_type = "shard"
policy.capacity = 100
policy.max_shards = 8

[subnets.general]

[subnets.general.canisters.blank]
initial_cycles = "3T"
```

This example defines two subnets (`prime` and `general`), enables the pool, enables ICRC-21, and configures both scaling and sharding strategies for hub canisters.
