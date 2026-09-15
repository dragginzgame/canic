# Frontend handoff

Canic's host exports browser bindings from one terminal Fleet review. The
frontend remains an independently built application. Its asset canister and
Internet Identity provider remain outside Canic's managed topology.

## Bind the admission origin before installation

For browser users, set the intended Internet Identity derivation origin in the
selected environment's protected Fleet generation policy:

```toml
[admission]
identity_origin = "https://service.example.com"
principals = ["<users derived under that origin>"]
```

`fleet generate` validates this origin and retains it in the reviewed bootstrap
input. It therefore participates in the plan digest. The installation applies
the existing Fleet admission policy. Roles that accept these users must declare
`fleet_admission = true`. Canic does not derive Internet Identity users or
establish that a supplied Principal was derived under the asserted origin.

Non-browser deployments may omit `identity_origin`. A frontend export with a
nonempty admission policy requires the retained origin to equal its derivation
origin. Admission Principals and operator authority never enter the browser
bundle. Origins belong to environment input, not the network-neutral App name.

## Export the selected roles

Prepare `frontend-local.json` or `frontend-ic.json` with the following shape.
Replace the illustrative values with the reviewed environment and actual
canister identities; `info env <fleet> --json` supplies the terminal role map.

```json
{
  "schema_version": 1,
  "environment": "ic",
  "canonical_network_id": "<selected canonical network ID>",
  "api_origin": "https://icp-api.io",
  "identity": {
    "canister_id": "<Internet Identity canister>",
    "provider_origin": "https://identity.example.com",
    "derivation_origin": "https://service.example.com",
    "alternative_origins": ["https://app.example.com"]
  },
  "asset": {
    "canister_id": "<externally managed asset canister>",
    "origin": "https://app.example.com"
  },
  "roles": [{"role": "app", "canister_id": "<terminal application canister>"}]
}
```

Use `"asset": null` when no asset canister has been selected. Choose each role
instance explicitly; the exporter does not guess an instance from a role name.

```sh
canic --environment ic frontend export demo --input frontend-ic.json --out frontend/canic --json
canic frontend verify frontend/canic --sha256 <independently-retained-digest>
```

The directory contains `canic-frontend.json`, exact `.did` sidecars, generated
`.did.mjs` JavaScript factories, `.did.d.ts` TypeScript declarations and
`.well-known/ii-alternative-origins`. Candid is resolved from the selected
environment's build artifacts and checked against each retained release/profile
hash. Public canister metadata is unnecessary. Missing or stale sidecars fail.

The manifest binds App/Fleet/network, source plan digest, role Principals,
module/release identities, origins, local trust and every listed artifact's
hash and byte count. Its SHA-256 covers compact JSON with recursively sorted
object keys and an empty self-digest. Retain that digest independently in the
frontend build input. A digest read from the same untrusted download is not an
independent check. Verification covers listed files, not unrelated files in the
directory. Publication writes the manifest last, accepts identical retries and
refuses to overwrite conflicting bytes.

This is a projection of verified **retained terminal authority**. Export does
not query live installations or establish that they still respond. An
incomplete Ensure operation rejects. A Component created after that terminal
inventory may be exported with `info env --component-operation`; generating
bindings for that later instance requires a terminal inventory that includes
its exact protocol binding.

## Agent and Internet Identity integration

The [independent SDK consumer](../../../crates/canic-host/examples/frontend-consumer/README.md)
provides manifest verification and actor construction. Mainnet uses the SDK's
compiled root key. Local export includes only the exact explicitly enrolled DER
root key, and the consumer verifies that it derives the manifest's network ID.
It never fetches a replacement trust key from the gateway.

Use the selected provider origin for the Internet Identity login and the
retained derivation origin for its `derivationOrigin`. The frontend owns login,
delegation renewal and its actor identity. Provider identity and origin are
operator-supplied configuration, not a Canic certification of that provider.

Origins must be canonical HTTPS origins with no path, trailing slash, userinfo,
query or fragment. Explicitly enrolled local networks also permit loopback
HTTP. Alternative origins must be unique. The current
[Internet Identity specification](https://docs.internetcomputer.org/references/internet-identity-spec/)
permits **100** alternative origins. The derivation origin need not appear in
that list; it can be empty when the served origin equals the derivation origin.
This corrects the older ten-origin assumption in CANIC-008.

The asset owner must serve the generated alternative-origin file at the
derivation origin with the certification and CORS behavior required by Internet
Identity. Canic generates its bytes and digest; it does not certify an external
HTTP deployment. Likewise, if a frontend uses ICP's certified environment cookie,
its asset environment must link the selected role IDs before synchronisation
and verify the published cookie. The manifest is an equivalent explicit build
input; it does not rewrite ICP links or cookies.

## External asset capacity

Before a destructive asset reinstall or an upload, review the exact payload and
the canister's **native** cycle balance:

```sh
canic --environment ic frontend capacity <asset-canister> --payload frontend/dist --minimum-native-cycles <reviewed-floor> --maximum-payload-bytes <reviewed-bound> --maximum-files <reviewed-bound> --json
```

The result includes observation time, native balance, selected byte/file totals,
payload digest and whether the explicit floor is met. Insufficient native cycles
return a typed failure and a nonzero exit. The command streams regular files,
rejects symlinks/special files and performs no funding, reinstall or upload.

Choose the floor from the asset owner's measured upload costs and operating
reserve. This check does not estimate batch costs, reserve cycles, guarantee an
upload or cover activity after the observation. `icp canister top-up` credits
native canister cycles. `icp cycles transfer` credits a Cycles Ledger account;
that account balance is not the canister's execution balance. Failed atomic
upload batches and their cleanup remain the asset owner's responsibility.

## Bounds and qualification

Host export currently permits 128 selected instances, 4 MiB per file and 32 MiB
of referenced artifacts. These are host allocation budgets, not IC, IcyDB or
application capacity limits. Export smaller explicit selections for larger
Fleets; changing these budgets requires host/consumer agreement, not a stable
memory migration. Asset payload limits are separately chosen by the operator;
the tree-entry bound is eight times its file bound plus the root.

Native tests cover origin/admission binding, exact publication/retry, conflicting
bytes, explicit payload bounds, local trust and the independent SDK verifier.
The focused real-Fleet/SDK journey passes: fresh convergence, exact public CLI
export, TypeScript compilation, an authenticated call observing the exact user,
denial of another user, missing-journal/stale-sidecar/origin rejection and an
insufficient native-cycle failure. It uses a browser-owned signing identity to exercise
the admission contract, not an Internet Identity UI ceremony or a real asset
upload. Toko adoption and certified external delivery remain downstream work.

Run the focused qualification after installing the example's pinned npm inputs:

```sh
make test-pocketic-case CASE=pic::fleet_registry::baseline::tests::frontend_handoff_public_cli_and_sdk_preserve_admission_and_local_trust
```

This opt-in case is ignored by ordinary Rust unit runs because it additionally
requires Node.js and the example's npm dependencies. The recorded final run takes
148.27 seconds (174 seconds including the runner); its frontend/SDK phase takes
4.01 seconds. The one-role fixture's complete bundle is 136,858 bytes. These are
controlled qualification measurements, not a Toko workload or asset-upload cost.

## Verify the uploaded handoff after sync

Keep the existing asset uploader. After it completes, read back the exact handoff:

```sh
canic --environment ic frontend verify-uploaded frontend/canic --sha256 <independently-retained-digest> --canister <asset-principal> --prefix /canic --json
```

The command checks the local bundle first, binds the environment and asset Principal,
checks the connected network's trust identity, and reads the identity encoding of
all declared files with the asset canister's `get`/`get_chunk` queries. It compares
actual bytes, lengths and SHA-256, including the exact uploaded manifest JSON.
The asset uploader must retain SHA-256 metadata and identity encodings.
A missing asset, changed content, wrong target, unsupported encoding or exhausted
budget returns a typed failure and a nonzero exit. Rerunning verification has no
canister mutation effect.

Upload the manifest and role binding files beneath `--prefix`. Upload the generated
`.well-known/ii-alternative-origins` at `/.well-known/ii-alternative-origins`, regardless
of that prefix. The JSON report lists the exact remote keys that were checked.
The prefix `/` selects the root for all handoff files.

ICP 1.5 allows explicit sync steps after a recipe's sync steps. Append a script
step to the externally owned asset canister's existing recipe declaration:

```yaml
sync:
  steps:
    - type: script
      commands:
        - >-
          canic --environment "$ICP_CLI_ENVIRONMENT" frontend verify-uploaded frontend/canic
          --sha256 "$CANIC_FRONTEND_SHA256" --canister "$ICP_CLI_CID" --prefix /canic --json
```

Set `CANIC_FRONTEND_SHA256` from the independently retained build input. Resolve
`frontend/canic` relative to the canister's configured directory. ICP's
[script sync context](https://github.com/dfinity/icp-cli/blob/v1.5.0/crates/icp/src/canister/sync/script.rs)
supplies the selected environment and current asset canister ID. Conflicting
explicit and script-context targets reject. Fleet descendants are taken from the
verified handoff; they need not exist in ICP's canister ID store.

This is a host script integration. ICP's experimental project bundler rejects
script sync steps; it is not a portable WASI sync plugin. Capacity review belongs
before upload, since recipe-appended verification runs after upload.

Readback bounds are 4 MiB per file, at most 64 chunks per file, 30 seconds per query
and 120 seconds for all asset queries. HTTP responses and Candid decoding are also
bounded. Local ICP identity/network discovery precedes that query deadline.
The check observes exact asset content, not an atomic snapshot of every file,
HTTP certification, CORS, content-type headers, caching or a browser II ceremony.
Those remain the asset owner's delivery checks.
