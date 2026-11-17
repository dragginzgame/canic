import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { Ed25519KeyIdentity } from '@tokostudios/ic-platform/identity';
import { Principal } from '@tokostudios/ic-platform/principal';
import { inspectCallIngress } from '@tokostudios/ic-platform/agent';
import { issuerAdapter, httpIssuerAdapter, IssuerError, type IssuerBinding,
  type IssuerUpdate } from '../src/issuer.js';
import { codec as issue, type Request, type Response } from '../src/generated/issue.js';
import { codec as renew } from '../src/generated/renew.js';
import { codec as lookup } from '../src/generated/reconcile.js';
import { codec as logout } from '../src/generated/logout.js';
import { codec as receiver } from '../src/generated/receiver-logout.js';
import { codec as credential } from '../src/generated/credential.js';

const account = Ed25519KeyIdentity.generate(new Uint8Array(32).fill(11));
const browser = Ed25519KeyIdentity.generate(new Uint8Array(32).fill(12));
const issuer = Principal.fromText('rrkah-fqaaa-aaaaa-aaaaq-cai');
const receiverId = Principal.fromText('ryjl3-tyaaa-aaaaa-aaaba-cai');
const signal = () => new AbortController().signal;
const material = { claims: new Uint8Array([1, 2, 3]),
  proof: { tag: 'Ed25519' as const, value: new Uint8Array(64).fill(4) } };
const issued: Response = { ok: { tag: 'Issued', value: {
  credential: material, admission_deadline_ns: 900n } } };
function binding(): IssuerBinding {
  const intent: Request = { epoch: 1n, operation_id: new Array(32).fill(2),
    login_epoch: 3n, login_id: new Array(32).fill(4),
    browser_public_key: [...browser.getPublicKey().rawKey],
    grants: [{ target: 'profile', scopes: ['read'] }] };
  return { issuer, account: account.getPrincipal(), browser: browser.getPrincipal(),
    intent, deadlineOffsetNs: -100n };
}
const code = (value: string) => (error: unknown) => error instanceof IssuerError && error.code === value;

test('generated codecs match native Rust Wire bytes', () => {
  const vectors = JSON.parse(readFileSync('src/generated/native-vectors.json', 'utf8')) as {
    request: number[]; response: number[] };
  const expected = { ...binding().intent, browser_public_key: new Array(32).fill(5) };
  assert.deepEqual(issue.encodeRequest(expected), new Uint8Array(vectors.request));
  assert.deepEqual(issue.encodeResponse(issued), new Uint8Array(vectors.response));
  const decoded = issue.decodeResponse(new Uint8Array(vectors.response));
  assert.ok('ok' in decoded && decoded.ok.tag === 'Issued');
  assert.deepEqual(credential.encodeRequest(decoded.ok.value.credential), credential.encodeRequest(material));
  assert.equal(decoded.ok.value.admission_deadline_ns, 900n);
});

test('original selectors, signer roles, canonical bytes and deadlines survive lookup', async () => {
  const calls: string[] = [], selected = binding();
  const original = issue.encodeRequest(selected.intent);
  const update: IssuerUpdate = async (method, target, identity, bytes, abort) => {
    calls.push(method); assert.equal(target.toText(), issuer.toText()); assert.ok(abort);
    if (method === 'issue_portable_credential') {
      assert.equal(identity, account); assert.deepEqual(bytes, original);
      return issue.encodeResponse(issued);
    }
    if (method === 'renew_portable_credential') {
      assert.equal(identity, browser);
      const request = renew.decodeRequest(bytes);
      assert.equal(request.sequence, 1n); assert.equal(request.login_epoch, 3n);
      assert.deepEqual(request.login_id, new Array(32).fill(4));
      assert.equal(request.grants.length, 1);
      assert.equal(request.grants[0]!.target, 'profile');
      assert.deepEqual(request.grants[0]!.scopes, ['read']);
      assert.deepEqual(request.account, account.getPrincipal().toUint8Array());
      return renew.encodeResponse(issued);
    }
    const request = lookup.decodeRequest(bytes);
    assert.equal(identity, request.tag === 'Issue' ? account : browser);
    if (request.tag === 'Issue') {
      assert.equal(request.value.epoch, 1n);
      assert.deepEqual(request.value.operation_id, new Array(32).fill(2));
    } else assert.equal(request.value.sequence, 1n);
    return lookup.encodeResponse(issued);
  };
  const { transport } = issuerAdapter(selected, update);
  selected.deadlineOffsetNs = 999n;
  (selected.intent.operation_id as number[])[0] = 99;
  (selected.intent.grants[0]!.scopes as string[])[0] = 'write';
  for (const value of [await transport.issue(account, signal()),
    await transport.renew(browser, 1n, signal()),
    await transport.reconcile(account, 0n, signal()),
    await transport.reconcile(browser, 1n, signal())]) {
    assert.equal(value.untilNs, 800n);
    assert.deepEqual(value.bytes, credential.encodeRequest(material));
  }
  assert.equal(calls.length, 4);
  await assert.rejects(transport.issue(browser, signal()), code('binding'));
  await assert.rejects(transport.renew(account, 1n, signal()), code('binding'));
  await assert.rejects(transport.renew(browser, 0n, signal()), code('sequence'));
  await assert.rejects(transport.reconcile(browser, 0n, signal()), code('binding'));
  await assert.rejects(transport.reconcile(account, 1n, signal()), code('binding'));
  await assert.rejects(transport.reconcile(browser, -1n, signal()), code('sequence'));
  assert.equal(calls.length, 4);
});

test('lost, unresolved, refused and malformed evidence never triggers issuance fallback', async () => {
  const methods: string[] = [];
  let response = lookup.encodeResponse({ ok: { tag: 'Unresolved', value: null } });
  const { transport } = issuerAdapter(binding(), async method => {
    methods.push(method);
    if (method === 'issue_portable_credential') throw new Error('lost reply');
    return response;
  });
  await assert.rejects(transport.issue(account, signal()), /lost reply/);
  await assert.rejects(transport.reconcile(account, 0n, signal()), code('unresolved'));
  response = lookup.encodeResponse({ err: { tag: 'Unavailable', value: null } });
  await assert.rejects(transport.reconcile(account, 0n, signal()), code('refused'));
  for (const bytes of [issue.encodeResponse(issued),
    lookup.encodeRequest({ tag: 'Issue', value: { epoch: 1n, operation_id: new Array(32).fill(2) } }),
    new Uint8Array([0]), new Uint8Array(131073)]) {
    response = bytes;
    await assert.rejects(transport.reconcile(account, 0n, signal()));
  }
  assert.equal(methods.filter(m => m === 'issue_portable_credential').length, 1);
  response = lookup.encodeResponse(issued);
  assert.equal((await transport.reconcile(account, 0n, signal())).untilNs, 800n);
  const badClock = issuerAdapter({ ...binding(), deadlineOffsetNs: -901n }, async () => issue.encodeResponse(issued));
  await assert.rejects(badClock.transport.issue(account, signal()), code('deadline'));
});

test('logout binds original login and preserves the receiver unit protocol', async () => {
  const seen: string[] = [];
  const receiverReply = receiver.encodeResponse({ tag: 'Revoked', value: 700n });
  const { revoke } = issuerAdapter(binding(), async (method, target, identity, bytes) => {
    seen.push(method); assert.equal(identity, browser);
    if (target.toText() === issuer.toText()) {
      const request = logout.decodeRequest(bytes);
      assert.equal(request.login_epoch, 3n);
      assert.deepEqual(request.login_id, new Array(32).fill(4));
      assert.deepEqual(request.account, account.getPrincipal().toUint8Array());
      return logout.encodeResponse({ ok: { login_deadline_ns: 1000n } });
    }
    assert.deepEqual(bytes, receiver.encodeRequest(null));
    return receiverReply;
  });
  const unit = receiver.encodeRequest(null);
  assert.deepEqual(receiver.decodeResponse(await revoke(issuer, browser, unit, signal())),
    { tag: 'Revoked', value: 1000n });
  assert.deepEqual(await revoke(receiverId, browser, unit, signal()), receiverReply);
  await assert.rejects(revoke(issuer, account, unit, signal()), code('binding'));
  await assert.rejects(revoke(issuer, browser, new Uint8Array([0]), signal()));
  assert.deepEqual(seen, ['revoke_portable_login', 'logout_portable_session']);
});

test('cancellation fences both sides of the update', async () => {
  const controller = new AbortController(); let calls = 0;
  const { transport } = issuerAdapter(binding(), async () => {
    calls++; controller.abort(new Error('cancelled')); return issue.encodeResponse(issued);
  });
  await assert.rejects(transport.issue(account, controller.signal), /cancelled/);
  await assert.rejects(transport.issue(account, controller.signal), /cancelled/);
  assert.equal(calls, 1);
});

test('real Auth HTTP transport signs the selected method once and keeps lost replies unresolved', async () => {
  let submitted = 0;
  const abort = signal();
  const { transport } = httpIssuerAdapter(binding(), {
    host: new URL('https://gateway.example/'), rootKey: new Uint8Array([1]),
    fetch: async (input, init) => {
      submitted++;
      assert.equal(String(input), `https://gateway.example/api/v2/canister/${issuer.toText()}/call`);
      assert.equal(init?.signal, abort); assert.equal(init?.redirect, 'error');
      const inspected = inspectCallIngress(new Uint8Array(init!.body as ArrayBuffer));
      assert.equal(inspected.sender.toText(), account.getPrincipal().toText());
      assert.equal(inspected.methodName, 'issue_portable_credential');
      assert.deepEqual(inspected.arg, issue.encodeRequest(binding().intent));
      throw new Error('lost HTTP reply');
    },
  });
  await assert.rejects(transport.issue(account, abort), /lost HTTP reply/);
  assert.equal(submitted, 1);
});
