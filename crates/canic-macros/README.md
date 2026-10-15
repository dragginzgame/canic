# canic-macros

Proc macros for defining Internet Computer endpoints in Canic canisters.

Most downstream users should access these macros through `canic`, not by
depending on `canic-macros` directly.

This crate provides `#[canic_query]` and `#[canic_update]`, which are thin wrappers
around the IC CDK `#[query]` / `#[update]` attributes and route through Canic's
pipeline (requires -> dispatch).
Use `all(...)`, `any(...)`, and `not(...)` inside `requires(...)` for composition.
Every endpoint declares `public` or `requires(...)`. Public endpoints retain
the default Fleet guard and use a fallible reply unless they select
`on_access_denied = "reject"` for a plain reply.

```rust
use canic::{Error, canic_query, canic_update};

#[canic_query(public)]
fn ping() -> Result<String, Error> {
    Ok("ok".to_string())
}

#[canic_update(requires(fleet::allows_updates(), caller::is_controller()))]
async fn admin_only_expr() -> Result<(), canic::Error> {
    Ok(())
}

#[canic_update(internal, requires(caller::is_parent()))]
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

## License

Source-available under the Dragginz Verifiable ICP Burn license (packaged as `LICENSE`).
Third-party Live Deployments require verifiable mainnet ICP burning: 100% of
ordinary protocol resource funding, or at least 20% of Cloud Engine infrastructure
value. Private and self-hosted deployments are included; zero-price arrangements
do not waive the requirement. Toko Studios Ltd has the express operational
exception in section 5. Earlier grants and third-party rights remain intact.
