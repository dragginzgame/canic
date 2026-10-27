# Independent frontend consumer

This small consumer qualifies Canic's frontend manifest using the published
`@tokostudios/ic-platform` 0.1.1 boundary. Its package lock pins transitive
SDK Core 6.1.1 and TypeScript
6.0.3 for declaration checks and export compilation, with Node 24.21.0 and npm 12.2.0. These are explicit
qualification versions; they are not a claim about the latest SDK or Toko's
complete dependency graph.

```sh
npm ci --ignore-scripts --no-audit --no-fund
node qualify.mjs verify /path/to/bundle <independent-manifest-sha256>
```

Use `verifyHandoff` from `handoff.mjs` with bounded fetched bytes, an independently
retained digest and a loader for relative artifact paths. Then statically import
the selected generated `bindings.js` module and call `createCanicClient` with the
verified handoff, exact selected canister ID and your authenticated SDK identity.
The caller owns HTTP fetch limits, timeouts and frontend build/deployment.

`createCanicClient` accepts only a verified, frozen handoff. It binds the agent to
the selected API origin and exact root key, with automatic root-key fetching
disabled. Agent policy explicitly retains three retries, query signature verification
enabled and construction-time clock synchronization disabled. Platform snapshots the admitted trust bytes and owns query rejection conversion. The native Node fixture supplies WebCrypto for the pinned Node runtime; browsers use
their built-in WebCrypto. Verification and client construction do not expose
operator credentials or create a privileged client.

Application login and renewal belong to the consuming application. Supply its
SDK-compatible authenticated identity to client construction. This fixture uses
Ed25519 identities directly and does not install a browser login provider.

## Platform boundary adoption

The canonical browser SDK owner is the published `@tokostudios/ic-platform`
0.1.1 package, using SDK Core 6.1.1. Agent/Principal/Ed25519 imports use its
explicit subpaths. `createAgentOptions` retains the admitted origin and root,
three retries, query verification and disabled construction-time synchronization;
`queryRaw` owns query rejection conversion. Updates retain SDK polling behavior.

`npm run check:boundary` rejects direct SDK declarations, aliases and source
references in this qualification root. Every `qualify.mjs` invocation runs this
check too. The transitive SDK remains necessary and is not a boundary violation.
Wire-generated codecs remain SDK-free. Cargo guards do not inspect this npm graph.
Installed generated-client qualification remains separate from package installation
or import checks; this migration alone is not live-provider or browser-UI evidence.

For the disposable PocketIC acceptance case, `qualify.mjs identity` creates a
private signing key in the test scratch directory. Its Principal is admitted
before installation. Query and update probes use the same policy. `call` asserts
the application sees that exact caller;
`denied` asserts that a separately generated, unadmitted caller receives a typed
error. Keys are never included in the handoff and the runner removes its scratch.
This proves generated bindings, network trust and authenticated ingress. It does
not emulate the Internet Identity user interface.

See the [frontend guide](../../../../docs/features/operations/frontend-handoff.md)
for input fields, origins, artifact budgets and native asset-cycle checks.

Export requires explicitly prepared compiler paths:

```sh
canic frontend export demo --input frontend-local.json --out browser/canic --node /absolute/path/to/node --typescript /absolute/path/to/node_modules/typescript
```

The package manifest owns exact Node/npm/TypeScript selections. Prepare the locked
packages with `npm ci` separately; export never installs them. The Host cache binds
the Node executable, entire TypeScript package, sealed sources and strict ES2022
compiler options. Bundles retain the Wire schema, source modules, JavaScript and
all declarations, each hashed under the manifest. `client.call(method, request)`
uses authenticated IC query/update transport and the selected generated codec.
External IC/provider Candid protocols are not part of this owned binding export.

## Continue From Here

- [Read the frontend handoff guide](../../../../docs/features/operations/frontend-handoff.md)
- [Browse Fleet operations](../../../../docs/features/operations/README.md)
- [Browse all documentation](../../../../docs/README.md)
- [Back to the main README](../../../../README.md)
