# Endpoint argument and reply controls

Owned Canic and application endpoints use native Wire with an explicit method ID
and separate request/reply budgets. Declare `ic-wire` in the owning workspace
catalog and inherit it in the endpoint package.

```rust
const REQUEST: ic_wire::Limits = ic_wire::Limits {
    bytes: 48 * 1024, allocation: 512 * 1024, nodes: 200_000, depth: 32,
};
const RESPONSE: ic_wire::Limits = ic_wire::Limits {
    bytes: 1024, allocation: 4096, nodes: 256, depth: 16,
};

#[canic::canic_update(requires(caller::is_controller()),
    wire(id = 0x4150_5000_0000_0001, request = REQUEST, response = RESPONSE))]
fn submit_batch(payload: Vec<u8>) -> Result<(), canic::Error> {
    // Validate application semantics and commit the intended operation here.
    Ok(())
}
```

These are illustrative budgets. Measure valid application inputs and choose the
actual frame-byte, decoded-allocation, traversal-node and nesting-depth bounds.
The request byte cap includes the entire method-bound Wire frame. Canic checks
that cap before copying ingress bytes and registers it with managed update
inspection. IC Wire owns framing and decoder resource admission. The same
endpoint checks apply to inter-canister updates; inspection alone does not.
Queries use their declared request budgets without an update inspector default.

A Wire handler accepts one Serde request value, or no arguments for unit, and
returns one Serde response. Authentication and semantic validation remain at
their Canic/application owners. Ordinary Result responses preserve typed access
refusals; `on_access_denied = "reject"` selects IC rejection. Reply encoding
failure traps, rolling back synchronous mutations. Async callbacks retain the
IC's transaction boundaries. There is no fallback codec.

## Inspect the retained client contract

```sh
canic info endpoints <fleet> <role> --release-build <sha256> --json
canic --environment staging info endpoints <fleet> <canister> --json
```

The first command inspects the selected completed build. The second resolves the
exact retained deployed release; target selection needs a unique role or exact
member Principal. Both read the release-admitted native Wire schema. Missing
release/schema evidence is an error; current source or an adjacent DID cannot
replace the selected contract. This inspection does not query live Wasm.

JSON records the source, source kind, release-build identity, types and method
entries. Each method supplies its ID, name, kind, mode, request/reply shapes and
`requestBytes`, `requestAllocation`, `requestNodes`, `requestDepth` plus the
corresponding `response*` budgets. Generators derive these declarations from
actual endpoint types and verify them against final Wasm exports and compiled
metadata. Clients must retain method identity and all applicable budgets.

Qualify accepted requests, exact byte boundaries and first-excess refusals in
real Wasm, including inter-canister callers where present. Byte length alone does
not prove allocation, node or depth admission. A browser budget or platform
maximum never overrides the selected endpoint limits.

## External Candid argument controls

External IC protocols keep their specified Candid ABI. For these methods,
Canic's managed update inspector has a default encoded-argument limit of 16 KiB,
exposed as `canic::CANIC_DEFAULT_UPDATE_INGRESS_MAX_BYTES`. The byte count includes
the complete Candid tuple and type table. Explicit adapter limits take priority:

| External method declaration | Managed update inspection | Raw adapter |
| --- | --- | --- |
| `payload(max_bytes = N)` | `N` bytes | Checks `N` before decoding |
| `decode = LIMITS` | `LIMITS.max_bytes` | Checks byte and Candid decoding budgets |
| No explicit limit | 16 KiB default | No explicit Canic raw payload bound |
| Bare IC CDK update | 16 KiB default | Canic's adapter does not run |

Queries may select `payload` or `decode`, but do not inherit update inspection.
Renamed endpoints register their exported name; feature selection follows the
compiled endpoint. Duplicate explicit method registrations reject inspection.
The Candid `// canic:payload-contract ` comment carries typed version-1 CBOR
metadata used by the external-adapter qualification fixture. It is separate from
native Wire schemas and the CLI's Wire inventory. The `payload_limit_probe` and
`pic_ingress_payload_limits` cases compare compiled registrations with actual
external/inter-canister boundary behavior.

## Bounded external Candid decoding and plain replies

For a protocol whose external owner specifies Candid, select all five dimensions
with a constant `canic::endpoint::ArgumentLimits`:

```rust
use canic::endpoint::ArgumentLimits;

const REQUEST_LIMITS: ArgumentLimits = ArgumentLimits {
    max_bytes: 32 * 1024,
    decoding_quota: 100_000,
    skipping_quota: 10_000,
    max_type_len: 128,
    max_header_len: 4096,
};

#[canic::canic_update(
    requires(caller::is_controller()),
    on_access_denied = "reject",
    decode = REQUEST_LIMITS
)]
fn accept(bytes: Vec<u8>) -> u64 {
    // Commit application intent synchronously here.
    bytes.len() as u64
}

#[canic::canic_query(public, decode = REQUEST_LIMITS)]
fn size(bytes: Vec<u8>) -> u64 {
    bytes.len() as u64
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

Managed initialization uses native Wire, including an empty post-upgrade tuple.
The owning artifact may put an `ic_wire::Limits` value first in its lifecycle declaration:

```rust,ignore
canic::start!(
    argument_limits = canic::protocol::initialization::LIMITS,
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
without running participants. A no-argument post-upgrade accepts only the empty
native tuple; supplied trailing bytes are rejected.

The option bounds the outer envelope. An application-owned blob nested in that
envelope still needs its own semantic validation and decoding limits. The public
`ic_wire::native::decode::<MyArgs>(&bytes, limits)` bounds that nested decode.
It does not replace selecting the initial lifecycle allowance. Omitting the
option selects the shared native initialization limits. Management install calls
retain their external Candid envelope, with these exact opaque argument bytes
retained across uncertain retries.

## Continue From Here

- [Review stable-memory layout](stable-memory-layout.md)
- [Browse runtime features](README.md)
- [Browse all documentation](../../README.md)
- [Back to the main README](../../../README.md)
