# canic-dsl-macros

Proc macros for defining Internet Computer endpoints in Canic canisters.

This crate provides `#[canic_query]` and `#[canic_update]`, which are thin wrappers
around the IC CDK `#[query]` / `#[update]` attributes and route through Canic’s
pipeline (guard → auth → env → rule → dispatch).

```rust
use canic_dsl::access::{auth::caller_is_controller, env::self_is_prime_subnet, guard::app_is_live};
use canic_dsl_macros::{canic_query, canic_update};

#[canic_query]
fn ping() -> String {
    "ok".to_string()
}

#[canic_update(guard(app_is_live), auth_any(caller_is_controller), env(self_is_prime_subnet))]
async fn admin_only() -> Result<(), canic::Error> {
    Ok(())
}
```
