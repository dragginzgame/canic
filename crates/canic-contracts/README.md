# Canic Contracts

Passive DTOs, identifiers, diagnostic codes and method contracts shared by Canic
endpoints, Core, Control Plane, Host and CLI.

The `protocol` modules declare exact method identities and bounded request/reply
contracts. Endpoint macros and callers consume those same declarations. IC Wire
owns framing, serialization, resource admission and schema/client generation.
This package does not own authoritative state, storage engines, lifecycle, timers, authentication policy
or platform effects.

Use the method's `protocol` module and its named `dto` request/reply types. The
host-only `wire-schema` feature derives metadata from their actual Serde shapes;
it does not reconstruct schemas from Candid. Required external protocols,
installation arguments and cryptographic inputs retain their owning encodings.

External IC contracts come directly from `ic-platform`. ICRC-21 consent handlers
use `ic_platform::icrc_ledger_types::icrc21::{ConsentMessageRequest, ConsentInfo,
Icrc21Error}`; Canic owns registration and dispatch, while Platform supplies the
request/reply types and their required Candid encoding.

Production callers and framework fixtures reuse identical bounded shapes.
The shared `dto::role::OperationAcceptedResponse` carries asynchronous acceptance
from Root, managed and Store commands. Purpose-specific projections retain their
own bounds and method-bound admission checks.

Stable records, their bounds, memory ownership and registration remain with
Core/Control Plane. The shared `serialization` helper selects bounded native Wire
with the `CSW` storage marker and format byte 1. Wire handles full-range `u128`
values directly; there is no custom numeric serializer or intermediate value tree.
The maximum single value is 64 MiB, with 1 GiB traversal allocation allowance,
64 Mi visited nodes and depth 128; owners retain narrower record bounds.
Separately fixed-width records, external IC protocols and dedicated Fleet activation
and release-build commitments keep their own formats. Checkpoint/caller authority
hashes use `protocol::authority` Wire bytes and explicit Wire domains, independently
of the storage envelope. Adoption requires clean reinstall; same-release recovery
and snapshot restoration use the same current format.
Runtime configuration compilation, policy,
authority checks and effects remain outside this package.

Core retains internal intent IDs and bounded stable resource keys. Control Plane
retains `installation::FleetCoordinatorInitArgs`, which includes Core's compiled
runtime configuration. Neither edge is an application endpoint contract.

The `canic` facade exposes these types to applications. Consumers regenerate
bindings from the selected artifact's native schema when adopting the current
contract.

## Passive identities and external adapters

Defaults are empty. Owned Principal identities and the DTO prelude's existing
`ExternalPrincipal` alias come directly from `ic-platform-types` with
`principal-external`. The minimum is 0.18.1, whose external identity capability
explicitly enables Serde without depending on Candid feature unification.
Explicit SHA-256 commitments remain part of this crate's passive contracts.

Enable `external-candid` for Nat conversion, external management status/settings,
inspection outcomes and Nat-bearing refill responses. This selects Platform's
Candid boundary and num-bigint. Core and Host consumers opt in explicitly;
`wire-schema` alone does not select these external adapters. Passive declarations
in the same modules, including bounded Wire inspection/funding models, remain
available without the feature. No fields, variants or encoded layouts change.

The workspace Wire dependency enables no schema features. `wire-schema` selects
`ic-wire/platform-types` explicitly for Principal metadata. Codec-only consumers
retain only Wire's runtime codecs; published Wire 0.16.2 still includes an
unconditional derive dependency, which requires a later upstream release to remove.
