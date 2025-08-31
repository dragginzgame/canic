# ICU Configuration

This document defines the TOML schema used by ICU and how it is loaded at build time.

## How It Loads

- Add the macro call to your canister crate’s `build.rs`:
  - `fn main() { icu::icu_build!("../icu.toml"); }`
- The macro validates the TOML, sets `ICU_CONFIG_PATH` (absolute), and enables the `icu_config` cfg.
- At runtime, ICU uses that path to embed the TOML with `include_str!`, so canisters run without extra files.

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
- `whitelist.principals: string[]` – optional; each must be a valid principal.
- `standards.icrc21: bool` – optional; off by default.
- `cycle_tracker: bool` – optional; off by default.

## Access at Runtime

- `icu::config::Config::try_get()` – returns `Arc<ConfigData>`.
- `Config::try_get_canister(&CanisterType)` – fetch a canister’s settings.
- `Config::is_whitelisted(&Principal)` – convenience helper for auth checks.
