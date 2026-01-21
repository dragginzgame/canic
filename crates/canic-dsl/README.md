# canic-dsl

Symbolic tokens for Canic endpoint macros.

This crate defines zero-cost marker constants used inside `#[canic_query]` and
`#[canic_update]` attributes. The symbols are never evaluated; the proc-macro
crate pattern-matches on identifiers and expands them into runtime access calls.
Use `requires(...)` expressions with `all(...)`, `any(...)`, and `not(...)` to
compose predicates.

```rust
use canic_dsl::access::{app, caller};
use canic_dsl_macros::canic_update;

#[canic_update(requires(app::allows_updates(), caller::is_controller()))]
async fn admin_only_expr() -> Result<(), canic::Error> {
    Ok(())
}
```
