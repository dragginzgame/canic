// Generated-code interoperability with installed Blob endpoints; Rust owns transport.
import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const [action, directory, service, tenant, namespace] = process.argv.slice(2);
const { blob_tenant: query, blob_update_tenant: update } =
  await import(pathToFileURL(join(directory, 'bindings.js')));
const scope = {
  service: Uint8Array.from(JSON.parse(service)),
  tenant: Uint8Array.from(JSON.parse(tenant)), namespace: BigInt(namespace),
};
const read = name => readFile(join(directory, name));
const wrongScope = { ...scope, service: new Uint8Array() };
if (action === 'query') {
  await writeFile(join(directory, 'query.request'), query.encodeRequest(scope));
  await writeFile(join(directory, 'query-binding.request'), query.encodeRequest(wrongScope));
} else if (action === 'update') {
  const original = query.decodeResponse(await read('query.response'));
  assert.ok(Object.hasOwn(original, 'ok'));
  assert.deepEqual({ ...original.ok.scope }, scope);
  assert.equal(original.ok.fenced, false);
  assert.ok(original.ok.enrollment?.some);
  await writeFile(join(directory, 'update.request'), update.encodeRequest({
    scope, expected: original.ok.enrollment, active: original.ok.enrollment.some.active,
  }));
  await writeFile(join(directory, 'update-binding.request'), update.encodeRequest({
    scope: wrongScope, expected: original.ok.enrollment, active: !original.ok.enrollment.some.active,
  }));
} else if (action === 'verify') {
  const original = query.decodeResponse(await read('query.response'));
  for (const name of ['update.response', 'replay.response']) {
    assert.deepEqual(update.decodeResponse(await read(name)), original);
  }
  for (const [codec, name, tag] of [
    [query, 'query-denied.response', 'Denied'], [update, 'update-denied.response', 'Denied'],
    [query, 'query-binding.response', 'Binding'], [update, 'update-binding.response', 'Binding'],
  ]) {
    const denied = codec.decodeResponse(await read(name));
    assert.deepEqual(Object.keys(denied), ['err']);
    assert.deepEqual({ ...denied.err }, { tag, value: null });
  }
} else {
  throw new Error('Unknown installed Blob qualification action');
}
