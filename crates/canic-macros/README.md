# canic-macros

Proc macros for defining Internet Computer endpoints in Canic canisters.

This crate provides `#[canic_query]` and `#[canic_update]`, which are thin wrappers
around the IC CDK `#[query]` / `#[update]` attributes and route through Canic’s
pipeline (guard → auth → policy → dispatch).

```rust
use canic::macros::{canic_query, canic_update};
use canic::prelude::*;

#[canic_query]
fn ping() -> String {
    "ok".to_string()
}

#[canic_update(guard(app), auth_any(is_controller), policy(is_prime_subnet))]
async fn admin_only() -> Result<(), canic::Error> {
    Ok(())
}
```
