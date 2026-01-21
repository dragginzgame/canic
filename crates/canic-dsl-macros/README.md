# canic-dsl-macros

Proc macros for defining Internet Computer endpoints in Canic canisters.

This crate provides `#[canic_query]` and `#[canic_update]`, which are thin wrappers
around the IC CDK `#[query]` / `#[update]` attributes and route through Canic's
pipeline (requires -> dispatch).
Use `all(...)`, `any(...)`, and `not(...)` inside `requires(...)` for composition.

```rust
use canic_dsl_macros::{canic_query, canic_update};

#[canic_query]
fn ping() -> String {
    "ok".to_string()
}

#[canic_update(requires(app::allows_updates(), caller::is_controller()))]
async fn admin_only_expr() -> Result<(), canic::Error> {
    Ok(())
}

#[canic_update(internal, requires(caller::is_parent()))]
async fn sync_state() -> Result<(), canic::Error> {
    Ok(())
}
```
