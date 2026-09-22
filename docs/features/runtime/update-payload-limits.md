# Application update payload limits

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
| `#[canic_update(...)]` without `payload` | Managed inspector default: 16 KiB | No explicit Canic raw payload adapter from this declaration |
| Bare `#[ic_cdk::update]` in a Canic-managed application | Managed inspector default: 16 KiB | Canic's ingress inspector is not on this call path |

This table describes ordinary application updates under Canic's generated
inspector. Framework protocol methods may have their own role/selector checks.
Queries are outside this update-limit setting. Inspection does not replace
endpoint authentication, authorization or argument validation.

The optional `name = "wire_method"` attribute binds an explicit payload limit
to the exported method name, even when the Rust function has another name.
Registration follows the compiled endpoint and its feature selection. Declaring
the same explicit method limit twice is invalid and inspection rejects it.

A browser budget or a larger IC platform maximum does not override this limit.
Oversized external ingress can be rejected during inspection before decoding or
entering the handler, so there may be no application error response or handler
log. Inspect the endpoint's declaration and the inherited default first.

## Client contract review

Run `canic --environment staging info endpoints <fleet> <canister> --json`
to inspect the selected declaration. Each update's `payload_limits` reports
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
