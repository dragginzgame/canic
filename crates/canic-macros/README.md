# canic-macros

Proc macros for defining Internet Computer endpoints in Canic canisters.

Most downstream users should access these macros through `canic`, not by
depending on `canic-macros` directly.

This crate provides `#[query]` and `#[update]`, which are thin wrappers
that marshal requests, enforce access and route through Canic's pipeline
(requires -> dispatch). Owned application endpoints select native Wire with
explicit method IDs and budgets. Declare `ic-wire` in the owning workspace
catalog and inherit it in each endpoint package. External IC protocols retain
their required Candid encoding through explicit `#[candid_query]` and
`#[candid_update]` adapters. `#[query]` and `#[update]` require a Wire contract.
Codec-owner integrations use `#[raw_query]` and `#[raw_update]` with explicit
bounds and their own decoding/reply path.
Use `all(...)`, `any(...)`, and `not(...)` inside `requires(...)` for composition.
Every endpoint declares `public` or `requires(...)`. Public endpoints retain
the default Fleet guard and use a fallible reply unless they select
`on_access_denied = "reject"` for a plain reply.

```rust
use canic::{Error, query, update};

const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_384, allocation: 65_536, nodes: 4096, depth: 32,
};

#[query(public,
    wire(id = 0x4150_5000_0000_0001, request = LIMITS, response = LIMITS))]
fn ping() -> Result<String, Error> {
    Ok("ok".to_string())
}

#[update(requires(fleet::allows_updates(), caller::is_controller()),
    wire(id = 0x4150_5000_0000_0002, request = LIMITS, response = LIMITS))]
async fn admin_only_expr() -> Result<(), canic::Error> {
    Ok(())
}

#[update(internal, requires(caller::is_parent()),
    wire(id = 0x4150_5000_0000_0003, request = LIMITS, response = LIMITS))]
async fn sync_state() -> Result<(), canic::Error> {
    Ok(())
}
```

## Continue From Here

- [Explore runtime features](../../docs/features/runtime/README.md)
- [Review endpoint access rules](../../docs/contracts/ACCESS_ARCHITECTURE.md)
- [Review endpoint argument and reply controls](../../docs/features/runtime/update-payload-limits.md)
- [Browse the public Canic crate](../canic/README.md)
- [Browse all documentation](../../docs/README.md)
- [Back to the main README](../../README.md)
