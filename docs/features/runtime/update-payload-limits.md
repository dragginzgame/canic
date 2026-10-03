# Endpoint argument and reply controls

Managed application updates inherit a **16 KiB (16,384 byte) encoded argument
limit** unless their Canic endpoint declares another limit. The bound includes
the entire Candid argument encoding, including its type table and all arguments;
it is not a string, action-list or JSON body size. An argument exactly at the
selected limit passes the byte check. Other admission checks still apply.

The public Rust constant `canic::CANIC_DEFAULT_UPDATE_INGRESS_MAX_BYTES` names
the default. For a method that needs a larger payload, put the intended bound
beside its endpoint declaration:

```rust,ignore
#[canic::canic_update(public, payload(max_bytes = 48 * 1024))]
fn submit_batch(payload: Vec<u8>) -> Result<(), canic::Error> {
    // Authenticate/authorize and validate the application request as required.
    Ok(())
}
```

`public` in this example is an access decision, not part of the payload setting.
Keep the endpoint's intended authentication and authorization predicates. A
larger byte limit grants no application authority and does not change platform
message limits.

## Identify the effective limit

| Application endpoint declaration | External update ingress | Inter-canister update |
| --- | --- | --- |
| `#[canic_update(..., payload(max_bytes = N))]` | `N` encoded argument bytes | The generated raw adapter also enforces `N` before decoding |
| `#[canic_update(..., decode = LIMITS)]` | `LIMITS.max_bytes` encoded bytes | The same byte limit and Candid budgets apply before dispatch |
| `#[canic_update(...)]` without `payload` or `decode` | Managed inspector default: 16 KiB | No explicit Canic raw payload bound from this declaration |
| Bare `#[ic_cdk::update]` in a Canic-managed application | Managed inspector default: 16 KiB | Canic's ingress inspector is not on this call path |

This table describes ordinary application updates under Canic's generated
inspector. Framework protocol methods may have their own role/selector checks.
Queries do not inherit the update inspector default. They may select
`payload(max_bytes = N)` for a raw byte bound or `decode = LIMITS` for all decoder
bounds. Inspection does not replace endpoint authentication, authorization or
argument validation.

The optional `name = "wire_method"` attribute binds an explicit payload limit
to the exported method name, even when the Rust function has another name.
Registration follows the compiled endpoint and its feature selection. Declaring
the same explicit method limit twice is invalid and inspection rejects it.

A browser budget or a larger IC platform maximum does not override this limit.
Oversized external ingress can be rejected during inspection before decoding or
entering the handler, so there may be no application error response or handler
log. Inspect the endpoint's declaration and the inherited default first.

## Client contract review

After `canic build`, use its emitted release-build ID to inspect a role offline:

```sh
canic info endpoints <fleet> <role> --release-build <sha256> --json
```

The Fleet argument does not select live state when `--release-build` is present.
The existing managed artifact owner verifies finalization, canonical child
manifest digests and the selected declaration hash. The command reads the
verified bytes directly, without an installed Fleet or a copied ICP sidecar.
It does not infer a latest build or substitute live/local metadata when selected
build verification fails. Current source/config edits do not replace the sealed
build's contract. This checks the declaration, not deployed Wasm or live admission.

Without `--release-build`,
`canic --environment staging info endpoints <fleet> <canister> --json` inspects
live metadata when available, otherwise the selected environment's local sidecar.
JSON identifies `source_kind` (`built`, `live` or `local`), `source`, and nullable
`release_build_id`; plain output identifies the source path or live metadata.
A local sidecar alone does not prove a selected build's identity.

Each update's `payload_limits` reports
`ingress_max_bytes`, `update_guard_max_bytes` and `ingress_basis`. The basis is
`managed_default`, `explicit_override` or `variant_dependent`. Queries have no
update payload contract. Plain output also shows these limits beside the method
inventory.

The declaration build emits a version-1 typed CBOR record in the dedicated
`// canic:payload-contract ` Candid comment field, encoded as hexadecimal. It
reads the compiled inspector's default and the same macro registrations used by
runtime inspection, including renamed and feature-selected endpoints. The
existing artifact hash covers the record; the Candid service schema is unchanged.
No extra network endpoint or manually synchronized limit inventory is required.
Declarations without this field report unknown limits. Malformed or duplicate
records reject contract inspection rather than silently supplying a default.

Built-in commands can select a smaller limit after decoding their request
variant. Those methods report a variant-dependent ingress limit rather than
promising that every request accepts the raw adapter's frame ceiling. An explicit
`update_guard_max_bytes` describes the pre-decode adapter; authentication and any
additional endpoint checks still apply. These fields describe encoded argument
bytes, not decoded values or platform message limits.

Measure the fully encoded Candid argument tuple in client tests. Keep the client
budget within the endpoint ceiling, including encoding overhead. For a larger
declared limit, qualify an accepted call above the inherited default, the exact
encoded boundary and the first byte beyond it in real Wasm. If inter-canister
callers use the method, qualify that path separately as well.

The existing `payload_limit_probe` and `pic_ingress_payload_limits` integration
target demonstrate default, explicit, bare-CDK, exported-name and inter-canister
behavior, and compare compiled metadata against exact and first-excess boundaries.
They exercise the owned inspector and generated adapter; no bypass or default
increase is needed.

## Bounded decoding and plain replies

Select all five dimensions with a constant `canic::endpoint::ArgumentLimits`:

```rust
use canic::endpoint::ArgumentLimits;

const REQUEST_LIMITS: ArgumentLimits = ArgumentLimits {
    max_bytes: 32 * 1024,
    decoding_quota: 100_000,
    skipping_quota: 10_000,
    max_type_len: 128,
    max_header_len: 4096,
};

#[derive(candid::CandidType)]
struct Receipt {
    accepted_bytes: u64,
}

#[canic::canic_update(
    requires(caller::is_controller()),
    on_access_denied = "reject",
    decode = REQUEST_LIMITS
)]
fn accept(bytes: Vec<u8>) -> Receipt {
    // Commit application intent synchronously here.
    Receipt { accepted_bytes: bytes.len() as u64 }
}

#[canic::canic_query(public, decode = REQUEST_LIMITS)]
fn size(bytes: Vec<u8>) -> Result<u64, canic::Error> {
    Ok(bytes.len() as u64)
}
```

These numbers are examples; the artifact owner must measure valid inputs and
select its own budgets. `decoding_quota` and `skipping_quota` are Candid work
units, not IC instruction or cycle budgets. `max_type_len` bounds type-table
entries and `max_header_len` bounds header bytes and declared header complexity.
Both quotas also cover Candid's handling of extra fields and arguments.

The raw byte check happens before allocating/copying the application argument
buffer. A single configured Candid decode checks the entire envelope, including
unused arguments. Invalid, incompatible or over-budget input traps before
preflight, access evaluation and handler dispatch. Managed update inspection
also receives `LIMITS.max_bytes`; inter-canister updates enforce it at the raw
entrypoint independently of inspection. Queries and composite queries use the
same configured decoder. `decode` and `payload(...)` cannot be combined.

`on_access_denied = "reject"` preserves the handler's declared Candid result and
uses IC rejection for access refusal. Normal Fleet guards, custom checks,
preflight, dispatch instrumentation and Candid exports remain in place. Predicates
can await before dispatch. A synchronous handler commits and replies without
an intervening await. Successful access emits no denial metric; a denied update
emits exactly one and does not execute the handler. Existing Result endpoints
retain their default behavior when this option is omitted.

## Initial lifecycle envelope

The owning artifact may put `argument_limits` first in its lifecycle declaration:

```rust,ignore
canic::start!(
    argument_limits = REQUEST_LIMITS,
    lifecycle_participant(
        init = crate::lifecycle::after_init,
        post_upgrade = crate::lifecycle::after_post_upgrade,
    ),
);
```

`start_local!`, `start_fleet_root!`, `start_wasm_store!` and
`start_fleet_coordinator!` also accept this option. It bounds each lifecycle
entrypoint the selected macro owns. Choose limits for the complete framework
envelope and application arguments. Decoding succeeds before restoration,
participants or deferred hooks run. Invalid input fails installation/upgrade
without running participants. A no-argument post-upgrade still validates and
boundedly skips supplied arguments; an empty raw argument buffer represents
the empty Candid envelope.

The option bounds the outer envelope. An application-owned blob nested in that
envelope still needs its own semantic validation and decoding limits. The public
`REQUEST_LIMITS.decode::<(MyArgs,)>(&bytes)` helper can bound such a decode and
returns `ArgumentDecodeError::TooLarge { actual, maximum }` or `InvalidCandid`.
It does not replace selecting the initial lifecycle bound. Omitting the option
retains the existing lifecycle decoder.

## Continue From Here

- [Review stable-memory layout](stable-memory-layout.md)
- [Browse runtime features](README.md)
- [Browse all documentation](../../README.md)
- [Back to the main README](../../../README.md)
