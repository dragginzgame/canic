# canic-dsl

Symbolic tokens for Canic endpoint macros.

This crate defines zero-cost marker constants used inside `#[canic_query]` and
`#[canic_update]` attributes. The symbols are never evaluated; the proc-macro
crate pattern-matches on identifiers and expands them into runtime access calls.

```rust
use canic_dsl::access::{auth::caller_is_controller, env::self_is_prime_subnet};
use canic_dsl_macros::canic_update;

#[canic_update(auth(caller_is_controller), env(self_is_prime_subnet))]
async fn admin_only() -> Result<(), canic::Error> {
    Ok(())
}
```
