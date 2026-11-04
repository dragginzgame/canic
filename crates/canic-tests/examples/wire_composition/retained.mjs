// Qualify codecs compiled from the exact modules embedded in a managed schema.
// Usage: node retained.mjs /absolute/path/to/compiled/bindings.js
import assert from "node:assert/strict";
import { pathToFileURL } from "node:url";

const bindingUrl = pathToFileURL(process.argv[2]);
const { blob_inspect_account, canic_public_health } = await import(bindingUrl);
const { createCodec } = await import(new URL("./wire-adapter.js", bindingUrl));
assert.equal(canic_public_health.decodeRequest(canic_public_health.encodeRequest(null)), null);
const relationship = {
  paid_canister: Uint8Array.of(4), payment_account: new Uint8Array(),
  spending_limit_per_day: "-1_000", current_period_spent: "1_234_567",
  current_period_start: 1n, added_timestamp: 2n, expiration_timestamp: { some: 3n },
  bandwidth_baseline_uploaded: "1_000", bandwidth_baseline_downloaded: "0",
  bandwidth_baseline_ts_ns: 4n,
};
const reply = {
  ok: {
    request: {
      scope: { service: Uint8Array.of(4), namespace: 1n, cashier: Uint8Array.of(4), payment_account: new Uint8Array() },
      kind: { tag: "PaymentRelationship", value: null },
    },
    observation: { tag: "Relationship", value: relationship },
  },
};
const bytes = blob_inspect_account.encodeResponse(reply);
const decoded = blob_inspect_account.decodeResponse(bytes);
assert.deepEqual(blob_inspect_account.encodeResponse(decoded), bytes);
assert.equal(decoded.ok.observation.value.spending_limit_per_day, "-1_000");
for (const invalid of ["-0", "1000", "1_000\n"]) {
  reply.ok.observation.value = { ...relationship, spending_limit_per_day: invalid };
  assert.throws(() => blob_inspect_account.encodeResponse(reply));
}
reply.ok.observation.value = { ...relationship, bandwidth_baseline_uploaded: "-1" };
assert.throws(() => blob_inspect_account.encodeResponse(reply));

// This qualification-only method exercises the other owner through the same adapter.
const network = createCodec({
  name: "network", mode: "query", id: "1",
  request: { k: "text", max: 64, check: "canicNetworkId" }, response: { k: "unit" },
  requestBytes: 256, responseBytes: 256,
  requestAllocation: 1024, responseAllocation: 1024,
  requestNodes: 16, responseNodes: 16, requestDepth: 8, responseDepth: 8,
});
const canonical = "ab".repeat(32);
assert.equal(network.decodeRequest(network.encodeRequest(canonical)), canonical);
for (const invalid of [canonical.toUpperCase(), canonical + "\n", "a".repeat(63)]) {
  assert.throws(() => network.encodeRequest(invalid));
}
console.log("Retained generated bindings enforce Blob decimal and Canic network validators");
