import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { composed } from "./compiled/bindings.js";
import { createCodec, WireError } from "./compiled/adapter.js";
const frame = JSON.parse(readFileSync(new URL("./frame.json", import.meta.url), "utf8"));
const account = {
  paid_canister: Uint8Array.of(4), payment_account: new Uint8Array(),
  spending_limit_per_day: "-1_000", current_period_spent: "1_234_567",
  current_period_start: 1n, added_timestamp: 2n, expiration_timestamp: { some: 3n },
  bandwidth_baseline_uploaded: "1_000", bandwidth_baseline_downloaded: "0",
  bandwidth_baseline_ts_ns: 4n,
};
const request = ["ab".repeat(32), account];
assert.deepEqual([...composed.encodeRequest(request)], frame);
const decoded = composed.decodeRequest(Uint8Array.from(frame));
assert.equal(decoded[0], request[0]);
assert.deepEqual({ ...decoded[1], expiration_timestamp: { ...decoded[1].expiration_timestamp } }, account);
for (const network of ["AB".repeat(32), request[0] + "\n"]) {
  assert.throws(() => composed.encodeRequest([network, account]));
}
for (const decimal of ["-0", "1000", "1_000\n"]) {
  assert.throws(() => composed.encodeRequest([request[0], { ...account, spending_limit_per_day: decimal }]));
}
assert.throws(() => createCodec({}, {}, { canicNetworkId: () => {} }), WireError);
console.log("Combined Canic/Blob generated codecs preserve both owners' scalar checks");
