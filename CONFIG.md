# Canic Configuration

This document defines the TOML schema consumed by Canic and how it is loaded at build time. Legacy ICU-compatible cfg/env hooks remain for downstream crates that have not migrated yet.

## How It Loads

- Add the macro call to your canister crate’s `build.rs`:
  - Root canister: `fn main() { canic::canic_build_root!("../canic.toml"); }`
  - Non‑root canister: `fn main() { canic::canic_build!("../canic.toml"); }`
- The macro validates the TOML, sets `CANIC_CONFIG_PATH` (absolute), and enables the `canic` cfg marker (plus legacy `icu`).
- At runtime, Canic embeds the TOML via `include_str!(env!("CANIC_CONFIG_PATH"))`, so deployments do not need to ship extra files. For backward compatibility the `ICU_CONFIG_PATH` environment variable is exported with the same value.
- In CI (when `GITHUB_ACTIONS=true`), build cfgs `canic_github_ci` and `icu_github_ci` are emitted; some example includes use this to avoid bundling local WASMs.

## Minimal Example (canic.toml)

```toml
controllers = ["aaaaa-aa"]

[pool]
minimum_size = 10

[canisters.example]
initial_cycles = "6T"
uses_directory = false
topup.threshold = "10T"
topup.amount = "5T"

[standards]
icrc21 = true
```

## Schema

- `controllers: string[]` – optional list of controller principals.
- `pool.minimum_size: u8` – optional; default 0.
- `canisters.<type>` – per-canister settings:
  - `initial_cycles: string` – required (e.g., "6T").
  - `topup.threshold: string`, `topup.amount: string` – optional; defaults to 10T/5T.
  - `uses_directory: bool` – optional.
  - `auto_create: bool` – optional; create this canister under root at init
  - `delegation: bool` – optional; enable delegation endpoints for this canister type.
  - `sharder` – optional; only for hub-like parents that assign items to shard pools:
    - `pools.<name>.canister_type: string` – child canister type.
    - `pools.<name>.policy.initial_capacity: u32`
    - `pools.<name>.policy.max_shards: u32`
    - `pools.<name>.policy.growth_threshold_pct: u32`
- `whitelist.principals: string[]` – optional; each must be a valid principal.
- `standards.icrc21: bool` – optional; off by default.
- `cycle_tracker: bool` – reserved; currently tracking runs unconditionally.

Validation rules
- Whitelist principals are validated for correct format.
- Sharder pool `canister_type` values must reference types declared under `[canisters]`.

## Access at Runtime

- `canic::config::Config::try_get()` – returns `Arc<ConfigData>`.
- `Config::try_get_canister(&CanisterType)` – fetch a canister’s settings.
- `ConfigData::is_whitelisted(&Principal)` – helper for auth checks.

Example
```rust
use canic::config::Config;
use canic::types::CanisterType;
use candid::Principal;

let cfg = Config::try_get()?; // Arc<ConfigData>
let game_cfg = cfg.try_get_canister(&CanisterType::new("game"))?;
let allowed = cfg.is_whitelisted(&Principal::from_text("aaaaa-aa")?);
```
