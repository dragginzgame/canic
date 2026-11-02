# Optional ICRC adapter

Canic's role startup emits owned Wire endpoints. ICRC discovery and wallet consent
are explicit application adapters, independent of `canic.toml`, role capabilities
and the build-generated cfg catalog. Enabling the feature alone mounts nothing. The governed build admits explicit ICRC
selection independently of compiled role authority. A package may forward its
`icrc` feature to `canic/icrc`; feature forwarding cannot enable role capabilities.
Platform owns the external types; Canic retains the small endpoint adapter.
Outbound management, CMC and ledger clients remain independent.

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
`SupportedStandard { name, url }` records from Platform and advertises ICRC-10,
as specified by the [external contract](https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-10/ICRC-10.did).

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
Candid update, as required by the external contract. Discovery also supports
[replicated query calls](https://docs.internetcomputer.org/references/ic-interface-spec/).
The artifact supplies
all argument limits; the example values above are not defaults. Mounting them
uses Platform 0.16.0's bounded Candid decoder, including its ceiling of 16
constructed wire levels before upstream decoding. Canic preserves caller-selected
byte/work/header limits and empty lifecycle envelope normalization. This admission
also checks skipped values; raw unbounded Candid adapters retain their distinct
decoder path. Installed standalone fixtures verify the 16-level admission
boundary and oversized input rejection on the selected dependency graph. Update
byte limits are also enforced by ingress inspection before execution.
Owned application endpoints remain Wire, regardless of visibility.

The embedded `request.arg` remains opaque to the adapter. The application
interprets it using the described method's own protocol, including Wire for
Canic-owned calls. Producing consent does not execute that method, admit its
caller to the Fleet or grant application spending authority.

## Maintained qualification

The [native adapter tests](../../../crates/canic/tests/icrc_adapter/mod.rs)
compare the complete generated service against the pinned external DID, including
ICRC-21's update mode. Discovery uses Platform's exact named-record identity.
Default role service inventories remain independent of optional application mounts.

The [installed target](../../../crates/canic-tests/tests/icrc_mounting.rs) builds
the existing managed-lifecycle probe in four explicit profiles: default, feature
only, discovery and consent. It inspects the actual exports and uses ordinary
managed activation with one lifecycle, timer and memory owner. It checks:

- Exact mounted methods, public query/update exports and standard names/URLs,
  including discovery through the IC replicated-query path.
- Anonymous and non-admitted callers receiving application-owned consent, with
  metadata, embedded Wire arguments and typed application refusals preserved.
- No described-operation execution or new admission authority; the normal
  application guard still refuses an unadmitted caller and permits an admitted one.
- Malformed/oversized envelopes and Platform's 16/17-level unused-header boundary.
- Same-release restoration of the selected static handler without registration.

Run the focused installed proof with `make test-pocketic-case CASE=icrc_mounting`.
Applications own their actual consent descriptions, language/device support and
business policy. These framework tests qualify local managed-App composition;
full Fleet deployment and wallet-client acceptance retain their owning flows.
