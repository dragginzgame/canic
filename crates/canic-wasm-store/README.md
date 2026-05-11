# canic-wasm-store

Canonical `wasm_store` canister crate for Canic.

This crate exists so downstream projects can build the implicit Canic
`wasm_store` role from a published canonical source instead of carrying their
own local `wasm_store` canister crate.

## Who should use this crate

Use this crate directly when you:

- want the canonical published `wasm_store` role in a downstream workspace
- need to build or package `wasm_store` from a source crate instead of copying a local role crate
- are wiring the standard Canic root/bootstrap/store topology

Most ordinary Canic canister projects should start with `canic`, not
`canic-wasm-store`.

## What this crate owns

This crate is intentionally narrow. It owns the published source for the
canonical `wasm_store` role and depends on the standard `canic` facade so the
role stays aligned with the rest of the root/store runtime.

It is not a general facade crate and it is not intended to replace `canic` as
the normal entry surface for application canisters.

## Canonical DID ownership

[`wasm_store.did`](wasm_store.did) is the checked-in canonical interface for
this crate. Ordinary local/bootstrap artifact builds copy that file into
`.icp/local/canisters/wasm_store/wasm_store.did`; they do not rewrite the
checked-in source file as a side effect of unrelated workspace changes.

If you intentionally need to refresh the canonical checked-in DID from the
built crate, run the host artifact builder from the Canic workspace with:

```bash
CANIC_REFRESH_WASM_STORE_DID=1 cargo run -q -p canic-host --example build_artifact -- wasm_store
```
