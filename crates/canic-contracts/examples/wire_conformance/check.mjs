import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { pagination, principals, diagnostic, accounts, identities, unsignedMagnitudes } from "./compiled/bindings.js";
import { createCodec } from "./compiled/wire-runtime.js";
import { createCodec as checkedCodec } from "./compiled/adapter.js";
import { canicNetworkId } from "./compiled/identities.js";

const vectors = JSON.parse(readFileSync(new URL("./vectors.json", import.meta.url), "utf8"));
const request = { limit: 128n, offset: 1n };
const response = { entries: [vectors.network], total: 1n };
assert.deepEqual([...pagination.encodeRequest(request)], vectors.request);
assert.deepEqual([...pagination.encodeResponse(response)], vectors.response);
assert.deepEqual({ ...pagination.decodeRequest(Uint8Array.from(vectors.request)) }, request);
assert.deepEqual({ ...pagination.decodeResponse(Uint8Array.from(vectors.response)) }, response);
for (const invalid of ["AB".repeat(32), "a".repeat(63), "a".repeat(65), "gg".repeat(32), "ab".repeat(32) + "\n"]) {
  assert.throws(() => canicNetworkId(invalid));
  assert.throws(() => pagination.encodeResponse({ entries: [invalid], total: 1n }));
  // Construct the same native text payload without calling the domain serializer.
  const bytes = new TextEncoder().encode(invalid);
  const frame = Uint8Array.from([...vectors.response.slice(0, 11), 1, bytes.length, ...bytes, 1]);
  assert.throws(() => pagination.decodeResponse(frame));
}
const wrong = Uint8Array.from(vectors.response);
wrong[2] = 18;
assert.throws(() => pagination.decodeResponse(wrong));
assert.throws(() => pagination.decodeRequest(Uint8Array.from(vectors.response)));
assert.throws(() => pagination.decodeResponse(Uint8Array.from([...vectors.response, 0])));
assert.throws(() => pagination.decodeResponse(new Uint8Array(1025)));
const missing = createCodec({ id: "17", request: { k: "text", check: "canicNetworkId" },
  response: { k: "unit" }, requestBytes: 1024, responseBytes: 1024,
  requestNodes: 128, responseNodes: 128, requestDepth: 16, responseDepth: 16 });
assert.throws(() => missing.encodeRequest(vectors.network));
console.log("Canic generated Wire client conformance passed");

const principalResponse = { entries: vectors.principals.map(p => Uint8Array.from(p)), total: 3n };
assert.deepEqual([...principals.encodeResponse(principalResponse)], vectors.principalResponse);
assert.deepEqual({ ...principals.decodeResponse(Uint8Array.from(vectors.principalResponse)) }, principalResponse);
assert.throws(() => principals.encodeResponse({ entries: [new Uint8Array(30)], total: 1n }));
const oversizedPrincipal = Uint8Array.from([...vectors.principalResponse.slice(0, 11), 1, 30, ...new Uint8Array(30), 1]);
assert.throws(() => principals.decodeResponse(oversizedPrincipal));
console.log("Native Platform Principal client conformance passed");

const results = vectors.results;
assert.deepEqual([...diagnostic.encodeRequest(false)], results.request);
assert.equal(diagnostic.decodeRequest(Uint8Array.from(results.request)), false);
const maximum = (1n << 64n) - 1n;
assert.deepEqual([...diagnostic.encodeResponse({ ok: maximum })], results.success);
assert.equal(diagnostic.decodeResponse(Uint8Array.from(results.success)).ok, maximum);
for (const { code, frame } of results.errors) {
  assert.deepEqual([...diagnostic.encodeResponse({ err: { code } })], frame);
  const decoded = diagnostic.decodeResponse(Uint8Array.from(frame));
  assert.deepEqual(Object.keys(decoded), ["err"]);
  assert.equal(decoded.err.code, code);
  assert.throws(() => diagnostic.decodeResponse(Uint8Array.from(frame.slice(0, -1))));
  assert.throws(() => diagnostic.decodeResponse(Uint8Array.from([...frame, 0])));
}
for (const code of [-1, 65536, 1.5, NaN]) {
  assert.throws(() => diagnostic.encodeResponse({ err: { code } }));
}
console.log("Canic lossless diagnostic Result client conformance passed");

for (const vector of vectors.accounts.cases) {
  const account = { owner: Uint8Array.from(vector.owner),
    subaccount: vector.subaccount === null ? null : { some: vector.subaccount } };
  const input = { account };
  assert.deepEqual([...accounts.encodeRequest(input)], vector.request);
  assert.deepEqual([...accounts.encodeResponse(input)], vector.response);
  for (const decoded of [accounts.decodeRequest(Uint8Array.from(vector.request)),
      accounts.decodeResponse(Uint8Array.from(vector.response))]) {
    assert.deepEqual(decoded.account.owner, account.owner);
    assert.deepEqual(decoded.account.subaccount, account.subaccount);
  }
  for (const [frame, decode] of [[vector.request, accounts.decodeRequest],
      [vector.response, accounts.decodeResponse]]) {
    assert.throws(() => decode(Uint8Array.from(frame.slice(0, -1))));
    assert.throws(() => decode(Uint8Array.from([...frame, 0])));
  }
}
for (const length of [0, 31, 33]) {
  const input = { account: { owner: new Uint8Array(), subaccount: { some: new Array(length).fill(0) } } };
  assert.throws(() => accounts.encodeRequest(input));
  assert.throws(() => accounts.encodeResponse(input));
}
const invalidOwner = { account: { owner: new Uint8Array(30), subaccount: null } };
assert.throws(() => accounts.encodeRequest(invalidOwner));
assert.throws(() => accounts.encodeResponse(invalidOwner));
assert.throws(() => accounts.decodeRequest(Uint8Array.from(vectors.accounts.malformedRequest)));
assert.throws(() => accounts.decodeResponse(Uint8Array.from(vectors.accounts.malformedResponse)));
console.log("Owned Platform Account client conformance passed");

const identity = vectors.identities;
assert.deepEqual([...identities.encodeRequest(identity.value)], identity.request);
assert.deepEqual([...identities.encodeResponse(identity.value)], identity.response);
for (const decoded of [identities.decodeRequest(Uint8Array.from(identity.request)),
    identities.decodeResponse(Uint8Array.from(identity.response))]) {
  assert.deepEqual(JSON.parse(JSON.stringify(decoded)), identity.value);
}
for (const invalid of identity.invalid) {
  const value = structuredClone(identity.value);
  if (invalid.field === "fleet") value.fleet.fleet.fleet_id = invalid.value;
  else value[invalid.field] = invalid.value;
  assert.throws(() => identities.encodeRequest(value));
  assert.throws(() => identities.encodeResponse(value));
  assert.throws(() => identities.decodeResponse(Uint8Array.from(invalid.frame)));
}
console.log("Fleet and Component identity client conformance passed");

for (const name of ["canicNetworkId", "canicFleetId", "canicComponentInstanceId", "canicComponentSpecId"]) {
  assert.throws(() => checkedCodec({}, {}, { [name]: () => {} }));
}

const runtimeBindings = await import("./compiled/bindings.js");
for (const vector of vectors.runtimeValues) {
  const binding = runtimeBindings[vector.method];
  const value = { tag: vector.label, value: null };
  assert.deepEqual([...binding.encodeRequest(value)], vector.request);
  assert.deepEqual([...binding.encodeResponse(value)], vector.response);
  assert.deepEqual(binding.decodeRequest(Uint8Array.from(vector.request)), value);
  assert.deepEqual(binding.decodeResponse(Uint8Array.from(vector.response)), value);
  assert.throws(() => binding.encodeRequest({ tag: "invalid", value: null }));
  assert.throws(() => binding.decodeResponse(Uint8Array.from([...vector.response, 0])));
}
console.log("Renamed runtime enum Rust/TypeScript vectors passed");

// The codec owns byte admission; the consumer owns numeric canonicality.
function magnitudeToBigInt(bytes) {
  if (!(bytes instanceof Uint8Array) || bytes.length > 64 || bytes[0] === 0)
    throw new Error("unsigned magnitude");
  let value = 0n;
  for (const byte of bytes) value = (value << 8n) | BigInt(byte);
  return value;
}
for (const vector of vectors.unsignedMagnitudes) {
  const value = vector.bytes === null ? null : { some: Uint8Array.from(vector.bytes) };
  assert.deepEqual([...unsignedMagnitudes.encodeRequest(value)], vector.request);
  assert.deepEqual([...unsignedMagnitudes.encodeResponse(value)], vector.response);
  for (const decoded of [unsignedMagnitudes.decodeRequest(Uint8Array.from(vector.request)),
      unsignedMagnitudes.decodeResponse(Uint8Array.from(vector.response))]) {
    if (vector.decimal === null) assert.equal(decoded, null);
    else assert.equal(magnitudeToBigInt(decoded.some), BigInt(vector.decimal));
  }
  assert.throws(() => unsignedMagnitudes.decodeResponse(Uint8Array.from([...vector.response, 0])));
  assert.throws(() => unsignedMagnitudes.decodeResponse(Uint8Array.from(vector.response.slice(0, -1))));
}
for (const bytes of [Uint8Array.of(0), Uint8Array.of(0, 1)]) {
  const frame = unsignedMagnitudes.encodeResponse({ some: bytes });
  const decoded = unsignedMagnitudes.decodeResponse(frame);
  assert.throws(() => magnitudeToBigInt(decoded.some));
}
assert.throws(() => unsignedMagnitudes.encodeResponse({ some: new Uint8Array(65) }));
console.log("Canic lossless unsigned magnitude client conformance passed");
