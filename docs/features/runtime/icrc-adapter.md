# Optional ICRC adapter

Canic's role startup emits owned Wire endpoints. ICRC discovery and wallet consent
are explicit application adapters, independent of `canic.toml`, role capabilities
and the build-generated cfg catalog. Enabling the feature alone mounts nothing.
Outbound management, CMC and ledger clients are unchanged.

Enable `canic`'s `icrc` Cargo feature in the application canister, then mount before
`canic::finish!()`:

```rust,ignore
const ICRC_LIMITS: canic::endpoint::ArgumentLimits = canic::endpoint::ArgumentLimits {
    max_bytes: 16_384,
    decoding_quota: 100_000,
    skipping_quota: 1_000,
    max_type_len: 128,
    max_header_len: 4_096,
};
canic::icrc::mount!(limits = ICRC_LIMITS);
```

This mounts only `icrc10_supported_standards`. It returns the external named
`SupportedStandard { name, url }` records and advertises ICRC-10. The previous
unconditional discovery response used tuple pairs, which do not match the
[ICRC-10 contract](https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-10/ICRC-10.did).
Regenerate consumers for the corrected external response.

To also support consent, supply a synchronous handler:

```rust,ignore
fn consent(request: canic::icrc::ConsentMessageRequest)
    -> Result<canic::icrc::ConsentInfo, canic::icrc::Icrc21Error>
{
    application_consent(request)
}
canic::icrc::mount!(limits = ICRC_LIMITS, consent = consent);
```

This additionally mounts `icrc21_canister_call_consent_message` and advertises
ICRC-21. The request, success and error retain Platform's upstream type identities.
The application owns routing by `request.method`, validation of embedded arguments,
language/device handling and descriptions. A consent request never authorizes
execution of that application operation. There is no global handler registry,
registration hook, cfg switch or adapter-owned lifecycle.

Discovery is a public external Candid query; ICRC-21 consent is a public external
Candid update, as required by the external contract. The artifact supplies
all argument limits; the example values above are not defaults. Mounting them
uses Platform 0.8.2's bounded Candid decoder, including its ceiling of 16
constructed wire levels before upstream decoding. Canic preserves caller-selected
byte/work/header limits and empty lifecycle envelope normalization. This admission
also checks skipped values; raw unbounded Candid adapters retain their distinct
decoder path. Installed standalone fixtures verify the 16-level admission
boundary and oversized input rejection on the selected dependency graph. Update
byte limits are also enforced by ingress inspection before execution.
Owned application endpoints remain Wire, regardless of visibility.

For the pending 0.113 hard cut, remove the global and component/child `[standards]`
tables from configuration. Delete uses of the old `api::protocol::icrc21`
dispatcher and select either no adapter, discovery only, or the direct consent
handler above. Default Coordinator/Store Candid inventories contain no ICRC
methods; applications deliberately mounting the adapter generate their own exact
external service metadata. Rebuild artifacts and their current release identities
through the governed source/build flow. No retained authority, cycle accounting,
transport retry policy or release version is changed by this adapter.

Focused qualification covers the two adapter integration tests, protocol and
managed-endpoint contracts, configuration/topology and role/feature catalog parity.
The pending 0.113 batch remains uncommitted; release and publication remain
maintainer-owned. Application-specific consent semantics remain application tests.


The complete generated ICRC-10/21 service is compared with the pinned external DID
in `crates/canic/tests/icrc_adapter/contract.did`, including the consent update mode.
Current installed evidence covers default, feature-only, discovery and consent
profiles, anonymous successful/unsupported consent calls, absent query routing for
consent, and input limits. See `target/icrc-completion-20261029/README.md`.
These standalone proofs do not validate an application's consent descriptions or
constitute a managed Fleet deployment. The earlier query-based consent proof is
historical and is superseded by this corrected update-based proof.
