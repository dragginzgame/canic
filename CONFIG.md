# Canic Configuration

This guide documents the canonical shape of `canic.toml`, the configuration file consumed by Canic build scripts and runtime helpers.

At a high level the file describes:

- Global cluster settings (`controllers`, `app_directory`, `standards`, `app`).
- Subnet-specific behaviour under `subnets.<name>` (including per-subnet pool settings).
- Per-canister policies inside each subnet, with optional scaling and sharding pools.

All fields are validated when `canic::build!` or `canic::build_root!` run, so configuration drift fails fast at compile time.

---

## Runtime Config + Env Lifecycle

Canic treats config/env identity as startup invariants. Missing env data is a fatal error.

- Build time: `CANIC_CONFIG_PATH` is embedded into the Wasm and `DFX_NETWORK` is baked in (`local` or `ic`), defaulting to `local` when unset.
- Init/post-upgrade: `__canic_load_config!()` loads the embedded TOML; `ConfigOps::current_*` is infallible.
- Root env: `root_init(identity)` sets base env fields directly from `SubnetIdentity` (no registry lookup).
  - `Prime` means root == subnet == prime root.
  - `Standard` carries the `subnet_type` and `prime_root_pid` from the prime subnet.
  - `Manual` is a test/support override that pins the subnet principal.
- Non-root env: children must receive a complete `EnvBootstrapArgs` in `CanisterInitPayload` from root.
  - Missing env fields always trap (no local fallback).

---

## Global Keys

### `controllers = ["aaaaa-aa", ...]`

Optional list of controller principals appended to every provisioned canister.

### `app_directory = ["role_a", "role_b", ...]`

Global set of canister roles that should appear in the prime root directory export. Every entry must also exist under `subnets.prime.canisters` and have `kind = "node"`.

### `[app]`

Initial application mode applied at canister install.

- `init_mode = "enabled" | "readonly" | "disabled"` – default `enabled`.

### `[app.whitelist]`

Optional allow-list for privileged operations.

- `principals = ["aaaaa-aa", ...]` – principal text strings authorised for whitelist checks.
  - If omitted, whitelist checks allow all principals.

### `[subnets.<name>.pool]`

Controls the warm canister pool for a subnet.

- `minimum_size: u8` – minimum number of spare canisters to keep on hand (default `0` when the table is omitted; required when the table is present).
- `import.initial: u16` – number of canisters to import immediately before queuing the rest (defaults to `minimum_size`).
- `import.local = ["aaaaa-aa", ...]` – canister IDs to import when built with `DFX_NETWORK=local` (also used when unset).
- `import.ic = ["aaaaa-aa", ...]` – canister IDs to import when built with `DFX_NETWORK=ic`.
  Import is destructive (controllers reset, code uninstalled); failures are logged and skipped.
If `pool.import.initial` is `0` and `auto_create` is non-empty, root bootstrap may create new canisters before queued imports are ready.

### `[log]`

Configure log retention for every canister.

- `max_entries: u64` – ring buffer cap on stored log entries (default `10000`).
- `max_entries` must be `<= 100000` (larger values are rejected at config validation).
- `max_entry_bytes: u32` – maximum message size in bytes per entry; oversized entries are truncated with a `...[truncated]` suffix (default `16384`).
- `max_age_secs: u64` – optional maximum age; entries older than this (in seconds) are purged (default `null` = no age limit).

### `[standards]`

Feature toggles tied to public standards.

- `icrc21: bool` – enable the ICRC-21 consent endpoint (default `false`).
- `icrc103: bool` – include ICRC-103 metadata (default `false`).

---

## Subnets

Declare each subnet under `[subnets.<name>]`. The name is an arbitrary identifier; `prime` is reserved for the main orchestrator subnet and should always be present.

### `[subnets.<name>]`

- `auto_create = ["role_a", ...]` – canister roles that root should ensure exist during bootstrap (must exist in `canisters`).
- `subnet_directory = ["role_a", ...]` – canister roles exposed through `canic_subnet_directory()`. Entries must have `kind = "node"`.
- `canisters.*` – nested tables describing per-role policies (see below).

### `[subnets.<name>.canisters.<role>]`

Each child table configures a logical canister role within the subnet. The role is derived
from the table key (`subnets.<name>.canisters.<role>`); do not declare `role`, `type`, or
`sharding.role` fields.

- `kind = "root" | "node" | "worker" | "shard"` – required; declares how this role attaches in the topology.
  - `root` cannot define scaling/sharding.
  - `root` must be unique across all subnets.
  - `subnets.prime.canisters.root` must exist and set `kind = "root"`.
  - `node` may define scaling or sharding pools for hub-style roles.
  - `worker` requires `scaling` and cannot define `sharding`.
  - `shard` does not require `sharding` and cannot define `scaling`.
- `initial_cycles = "5T"` – cycles to allocate when provisioning (defaults to 5T).
- `topup.threshold = "10T"` – minimum cycles before requesting a top-up (set both fields if enabling top-ups).
- `topup.amount = "5T"` – cycles to request when topping up (set both fields if enabling top-ups).
  Omit `topup` entirely to disable auto top-ups.
- `randomness.enabled = true` – enable PRNG seeding (set `false` to disable).
- `randomness.reseed_interval_secs = 3600` – reseed interval in seconds (default `3600`, must be > 0 when enabled).
- `randomness.source = "ic"` – seeding source (`ic` or `time`, default `ic`).
  - `time` uses `ic_cdk::api::time()` and is deterministic/low-entropy; use for non-sensitive randomness only.
- `scaling` – optional table that defines stateless worker pools.
- `sharding` – optional table that defines stateful shard pools.

#### Scaling Pools

Scaling pools model interchangeable workers with simple bounds on how many to keep alive.

```
[subnets.<name>.canisters.<role>.scaling.pools.<pool>]
canister_role = "worker_role"
policy.min_workers = 2
policy.max_workers = 16
```

Fields:

- `canister_role` – canister role that represents workers in this pool (must exist in the same subnet).
- `policy.min_workers` – minimum workers to keep alive (default `1`).
- `policy.max_workers` – hard cap on workers (default `32`, set to `0` for no max).

#### Sharding Pools

Sharding pools manage stateful shards that own tenant partitions.

```
[subnets.<name>.canisters.<role>.sharding.pools.<pool>]
canister_role = "shard_role"
policy.capacity = 1000
policy.max_shards = 64
```

Fields:

- `canister_role` – canister role that implements the shard (must exist in the same subnet).
- `policy.capacity` – per-shard capacity (default `1000`, must be > 0).
- `policy.max_shards` – maximum shard count (default `4`, must be > 0).

### Randomness (Per-Canister)

```
[subnets.<name>.canisters.<role>.randomness]
enabled = true
reseed_interval_secs = 3600
source = "ic" # or "time"
```

Fields:

- `enabled` – enable PRNG seeding (default `true`).
- `reseed_interval_secs` – reseed interval in seconds (default `3600`, must be > 0 when enabled).
- `source` – `ic` for management canister `raw_rand`, `time` for `ic_cdk::api::time()`.

---

## Example

```toml
controllers = ["aaaaa-aa"]
app_directory = ["auth_hub", "scale_hub", "shard_hub"]

[delegation]
enabled = true

[standards]
icrc21 = true

[subnets.prime]
auto_create = ["app", "auth_hub", "scale_hub", "shard_hub"]
subnet_directory = ["app", "auth_hub", "scale_hub", "shard_hub"]
pool.minimum_size = 3
pool.import.initial = 3
pool.import.local = ["aaaaa-aa"]
pool.import.ic = ["aaaaa-aa"]

[subnets.prime.canisters.app]
kind = "node"

[subnets.prime.canisters.auth_hub]
kind = "node"
topup.threshold = "10T"
topup.amount = "5T"

[subnets.prime.canisters.auth_hub.sharding.pools.auth_shards]
canister_role = "auth_shard"
policy.capacity = 100
policy.max_shards = 4

[subnets.prime.canisters.scale_hub]
kind = "node"
topup.threshold = "10T"
topup.amount = "5T"

[subnets.prime.canisters.scale_hub.scaling.pools.scales]
canister_role = "scale"
policy.min_workers = 2

[subnets.prime.canisters.scale]
kind = "worker"
[subnets.prime.canisters.scale.scaling]

[subnets.prime.canisters.shard_hub]
kind = "node"
topup.threshold = "10T"
topup.amount = "5T"

[subnets.prime.canisters.shard_hub.sharding.pools.shards]
canister_role = "shard"
policy.capacity = 100
policy.max_shards = 8

[subnets.prime.canisters.shard]
kind = "shard"
[subnets.prime.canisters.shard.sharding]

[subnets.prime.canisters.auth_shard]
kind = "shard"
[subnets.prime.canisters.auth_shard.sharding]

[subnets.general]

[subnets.general.canisters.blank]
kind = "worker"
initial_cycles = "3T"
[subnets.general.canisters.blank.scaling]
```

This example defines two subnets (`prime` and `general`), enables the pool, enables ICRC-21, and configures both scaling and sharding strategies for hub canisters.
