# ICU Configuration

This document defines the TOML schema used by ICU and how it is loaded at build time.

## How It Loads

- Add the macro call to your canister crate’s `build.rs`:
  - Root canister: `fn main() { icu::icu_build_root!("../icu.toml"); }`
  - Non‑root canister: `fn main() { icu::icu_build!("../icu.toml"); }`
- The macro validates the TOML, sets `ICU_CONFIG_PATH` (absolute), and enables the `icu_config` cfg.
- At runtime, ICU embeds the TOML via `include_str!(env!("ICU_CONFIG_PATH"))`, so deployments do not need to ship extra files.
- In CI (when `GITHUB_ACTIONS=true`), a build cfg `icu_github_ci` is emitted; some example includes use this to avoid bundling local WASMs.

## Minimal Example (icu.toml)

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
  - `auto_create: u16` – optional; pre-create N children for this type (root only).
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

- `icu::config::Config::try_get()` – returns `Arc<ConfigData>`.
- `Config::try_get_canister(&CanisterType)` – fetch a canister’s settings.
- `ConfigData::is_whitelisted(&Principal)` – helper for auth checks.

Example
```rust
use icu::config::Config;
use icu::types::CanisterType;
use candid::Principal;

let cfg = Config::try_get()?; // Arc<ConfigData>
let game_cfg = cfg.try_get_canister(&CanisterType::new("game"))?;
let allowed = cfg.is_whitelisted(&Principal::from_text("aaaaa-aa")?);
```
