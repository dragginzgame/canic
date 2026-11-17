import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { once } from 'node:events';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const require = createRequire(new URL('../../../crates/canic/browser/package.json', import.meta.url));
const { build } = require('esbuild');
const { chromium } = require('playwright-core');
const executablePath = process.env.CANIC_AUTH_BROWSER_EXECUTABLE;
assert(executablePath, 'Set CANIC_AUTH_BROWSER_EXECUTABLE to a prepared Chromium; no browser is downloaded.');
const config = JSON.parse(readFileSync(process.argv[2], 'utf8'));
const bundle = await build({ entryPoints: [fileURLToPath(new URL('profile.mjs', import.meta.url))],
  nodePaths: [fileURLToPath(new URL('../../../crates/canic/browser/node_modules', import.meta.url))],
  bundle: true, format: 'iife', platform: 'browser', write: false });
const server = createServer((request, response) => {
  if (!['/', '/test.js'].includes(request.url)) { response.writeHead(404).end(); return; }
  response.setHeader('Content-Type', request.url === '/test.js' ? 'text/javascript' : 'text/html');
  response.end(request.url === '/test.js' ? bundle.outputFiles[0].contents : '<!doctype html><script src="/test.js"></script>');
});
server.listen(0, '127.0.0.1');
await once(server, 'listening');
let browser;
let cases = 0;
const check = async (name, operation) => { await operation(); console.log(`ok ${++cases} - ${name}`); };
async function qualify() {
  browser = await chromium.launch({ executablePath, headless: true });
  console.log(`Headless Chromium ${browser.version()}`);
  const context = await browser.newContext();
  context.setDefaultTimeout(15000);
  const url = `http://127.0.0.1:${server.address().port}/`;
  const open = async options => {
    const page = await context.newPage();
    await page.goto(url);
    await page.evaluate(({ config, options }) => canicProfile.setup(config, options), { config, options });
    return page;
  };
  let a = await open({ loseIssue: true });
  let original;
  await check('lost issuance retains uncertainty, and a new renderer reconciles the original signature', async () => {
    const pending = await a.evaluate(async () => {
      let lost = false;
      try { await flow.coordinator.ensureSession(flow.targets[0]); }
      catch (error) { if (!String(error).includes('lost issuer reply')) throw error; lost = true; }
      const status = await flow.journal.inspect();
      return { lost, unknown: status.issuanceUnknown, sequence: String(status.sequence), original: flow.original, counts: flow.counts };
    });
    assert.equal(pending.lost, true); assert.equal(pending.unknown, true); assert.equal(pending.sequence, '0');
    assert.equal(pending.counts.issue, 1); assert.equal(pending.counts.admit, 0);
    original = pending.original;
    await a.close();
    a = await open({ reopen: true, loseAdmission: true });
    const result = await a.evaluate(async () => {
      const evidence = await canicProfile.retrieveCredential(flow.options);
      let lost = false;
      try { await flow.coordinator.ensureSession(flow.targets[0]); }
      catch (error) { if (!String(error).includes('lost admission reply')) throw error; lost = true; }
      return { bytes: Array.from(evidence.bytes), untilNs: String(evidence.untilNs), lost, counts: flow.counts };
    });
    assert.deepEqual({ bytes: result.bytes, untilNs: result.untilNs }, original);
    assert.equal(result.lost, true); assert.equal(result.counts.issue, 0); assert.equal(result.counts.lookup, 1);
  });
  await check('lost admission reopens exactly, and a second receiver requires independent admission', async () => {
    await a.close(); a = await open({ reopen: true });
    const result = await a.evaluate(async () => {
      const first = await flow.coordinator.ensureSession(flow.targets[0]);
      const before = await flow.raw('profile_get', flow.targets[1], flow.browser, flow.config.frames.getA);
      const second = await flow.coordinator.ensureSession(flow.targets[1]);
      flow.receipt = first;
      const result = await flow.write(flow.targets[0], flow.config.frames.putA);
      return { firstSubject: canicProfile.Principal.fromUint8Array(first.subject).toText(),
        secondSubject: canicProfile.Principal.fromUint8Array(second.subject).toText(),
        account: flow.account.getPrincipal().toText(), before: Array.from(before), result: Array.from(result), counts: flow.counts };
    });
    assert.equal(result.firstSubject, result.account); assert.equal(result.secondSubject, result.account);
    assert.deepEqual(result.before, config.frames.unauthGet);
    assert.deepEqual(result.result, config.frames.profileA); assert.equal(result.counts.writes, 1);
    assert.equal(result.counts.issue, 0); assert.equal(result.counts.lookup, 0); assert.equal(result.counts.admit, 2);
  });
  const b = await open({ device: 12, scopes: ['read'] });
  const c = await open({ device: 11, accountSeed: 10, loseIssue: true });
  await check('independent device and user sessions preserve immutable profile ownership', async () => {
    const bResult = await b.evaluate(async () => {
      await flow.coordinator.ensureSession(flow.targets[0]);
      return { deniedWrite: Array.from(await flow.write(flow.targets[0], flow.config.frames.changeA)),
        own: Array.from(await flow.get(flow.targets[0], flow.config.frames.getA)) };
    });
    assert.deepEqual(bResult.deniedWrite, config.frames.unauthPut);
    assert.deepEqual(bResult.own, config.frames.profileAGet);
    const cResult = await c.evaluate(async () => {
      let lost = false;
      try { await flow.coordinator.ensureSession(flow.targets[0]); }
      catch (error) { if (!String(error).includes('lost issuer reply')) throw error; lost = true; }
      const recovered = await canicProfile.retrieveCredential(flow.options);
      await flow.coordinator.ensureSession(flow.targets[0]);
      return { lost, recovered: { bytes: Array.from(recovered.bytes), untilNs: String(recovered.untilNs) },
        original: flow.original, counts: flow.counts,
        forbidden: Array.from(await flow.get(flow.targets[0], flow.config.frames.getA)),
        own: Array.from(await flow.write(flow.targets[0], flow.config.frames.putC)) };
    });
    assert.equal(cResult.lost, true); assert.deepEqual(cResult.recovered, cResult.original);
    assert.equal(cResult.counts.issue, 1); assert.equal(cResult.counts.lookup, 1);
    assert.deepEqual(cResult.forbidden, config.frames.forbiddenGet); assert.deepEqual(cResult.own, config.frames.profileC);
    const wrong = await a.evaluate(async () => ({
      account: Array.from(await flow.raw('profile_get', flow.targets[0], flow.account, flow.config.frames.getA)),
      other: Array.from(await flow.get(flow.targets[0], flow.config.frames.getC)),
      remap: Array.from(await flow.write(flow.targets[0], flow.config.frames.remapA)),
    }));
    assert.deepEqual(wrong.account, config.frames.unauthGet); assert.deepEqual(wrong.other, config.frames.forbiddenGet);
    assert.deepEqual(wrong.remap, config.frames.alreadyRegistered);
  });
  await check('reply encoding failure rolls back the real profile store', async () => {
    const result = await a.evaluate(async () => {
      await flow.control(flow.targets[0], 2);
      let trapped = false;
      try { await flow.write(flow.targets[0], flow.config.frames.changeA); }
      catch (error) { if (!String(error).includes('profile endpoint refused')) throw error; trapped = true; }
      await flow.control(flow.targets[0], 3);
      return { trapped, retained: Array.from(await flow.get(flow.targets[0], flow.config.frames.getA)) };
    });
    assert.equal(result.trapped, true); assert.deepEqual(result.retained, config.frames.profileAGet);
  });
  await check('renewal and lookup retain one original result and do not repeat application writes', async () => {
    const result = await a.evaluate(async () => {
      const renewed = await canicProfile.retrieveCredential({ ...flow.options, refreshThrough: 0n });
      const looked = await flow.adapter.transport.reconcile(flow.browser, 1n, AbortSignal.timeout(5000));
      await flow.coordinator.establish(flow.targets[0]);
      return { renewed: { bytes: Array.from(renewed.bytes), untilNs: String(renewed.untilNs) },
        looked: { bytes: Array.from(looked.bytes), untilNs: String(looked.untilNs) },
        profile: Array.from(await flow.get(flow.targets[0], flow.config.frames.getA)), renews: flow.counts.renew };
    });
    assert.deepEqual(result.renewed, result.looked); assert.equal(result.renews, 1);
    assert.deepEqual(result.profile, config.frames.profileAGet);
  });
  const retainedProof = await a.evaluate(() => Array.from(flow.admission));
  await check('logout survives renderer and network loss while preserving the other device', async () => {
    const first = await a.evaluate(async () => {
      const status = await flow.journal.beginLogout(flow.generation, canicProfile.nowNs() + 30_000_000_000n);
      let refused;
      try { await flow.coordinator.ensureSession(flow.targets[0]); } catch (error) { refused = error.code; }
      const cleanup = { journal: flow.journal, generation: flow.generation, browser: flow.browser,
        revoke: flow.adapter.revoke };
      const step = await canicProfile.cleanupOnce(cleanup);
      return { phase: status.phase, deadline: String(status.deadlineNs), refused, kind: step.kind };
    });
    assert.equal(first.phase, 'cleanup'); assert.equal(first.refused, 'not_current'); assert.equal(first.kind, 'settled');
    await a.close(); a = await open({ reopen: true });
    // Block the real HTTP request, not its decoded reply. The destination cannot
    // acknowledge this attempt; reopening must retain its count and original lease.
    let blocked = 0;
    await a.route(`**/canister/${config.receivers[0]}/**`, async route => {
      blocked++; await route.abort('connectionfailed');
    });
    const unavailable = await a.evaluate(async () => {
      let error;
      try { await canicProfile.cleanupOnce({ journal: flow.journal, generation: flow.generation,
        browser: flow.browser, revoke: flow.adapter.revoke }); }
      catch (cause) { error = String(cause); }
      const status = await flow.journal.inspect();
      const target = status.destinations.find(d => d.canister === flow.config.receivers[0]);
      return { error, phase: status.phase, deadline: String(status.deadlineNs), attempts: target.attempts,
        confirmed: target.confirmedUntilNs !== undefined };
    });
    assert.equal(blocked, 1); assert.match(unavailable.error, /fetch/i);
    assert.equal(unavailable.phase, 'cleanup'); assert.equal(unavailable.deadline, first.deadline);
    assert.equal(unavailable.attempts, 1); assert.equal(unavailable.confirmed, false);
    await a.close(); a = await open({ reopen: true });
    const finished = await a.evaluate(async () => {
      const before = await flow.journal.inspect();
      const result = await canicProfile.cleanupUntilFinished({ journal: flow.journal, generation: flow.generation,
        browser: flow.browser, revoke: flow.adapter.revoke });
      return { before: String(before.deadlineNs), deadline: String(result.deadlineNs), phase: result.phase,
        beforeAttempts: before.destinations.find(d => d.canister === flow.config.receivers[0]).attempts,
        afterAttempts: result.destinations.find(d => d.canister === flow.config.receivers[0]).attempts,
        confirmed: result.destinations.map(d => d.confirmedUntilNs !== undefined),
        refused: Array.from(await flow.raw('profile_get', flow.targets[0], flow.browser, flow.config.frames.getA)) };
    });
    assert.equal(finished.before, first.deadline); assert.equal(finished.deadline, first.deadline);
    assert.equal(finished.beforeAttempts, 1); assert.equal(finished.afterAttempts, 2);
    assert.equal(finished.phase, 'finished'); assert.deepEqual(finished.confirmed, [true, true, true]);
    assert.deepEqual(finished.refused, config.frames.unauthGet);
    assert.deepEqual(await a.evaluate(async frame => Array.from(await flow.raw(
      'establish_portable_session', flow.targets[0], flow.browser, frame)), retainedProof), config.frames.revoked);
    assert.deepEqual(await b.evaluate(async () => Array.from(await flow.get(flow.targets[0], flow.config.frames.getA))), config.frames.profileAGet);
  });
  await check('wrong-scope admission and real Unconfirmed logout remain visible through the cleanup deadline', async () => {
    const result = await b.evaluate(async () => {
      let refusal;
      try { await canicProfile.establishSession({ ...flow.options, target: flow.targets[1], scopes: ['admin'] }); }
      catch (error) { refusal = error.refusal; }
      const logout = await flow.journal.beginLogout(flow.generation, canicProfile.nowNs() + 8_000_000_000n);
      const results = [];
      const options = { journal: flow.journal, generation: flow.generation,
        browser: flow.browser, revoke: async (...args) => {
          const reply = await flow.adapter.revoke(...args);
          results.push(canicProfile.logoutCodec.decodeResponse(reply).tag);
          return reply;
        } };
      let status;
      try { status = await canicProfile.cleanupUntilFinished(options); }
      catch (error) {
        // A request at the cleanup boundary can time out instead of returning a
        // typed nonconfirmation. Resume the same journal; never extend its deadline.
        if (error.code !== 'timeout') throw error;
        if (canicProfile.nowNs() < logout.deadlineNs) throw error;
        status = await canicProfile.cleanupUntilFinished(options);
      }
      return { refusal, results, phase: status.phase, deadline: String(status.deadlineNs), original: String(logout.deadlineNs),
        unresolved: status.destinations.filter(d => d.confirmedUntilNs === undefined).map(d => d.canister) };
    });
    assert.equal(result.refusal, 'ScopeRejected'); assert.ok(result.results.includes('Unconfirmed'));
    assert.equal(result.deadline, result.original);
    assert.equal(result.phase, 'finished'); assert.deepEqual(result.unresolved, [config.receivers[1]]);
  });
  await check('durable logout in another renderer fences a suspended real SDK signer', async () => {
    await c.evaluate(async () => {
      const original = flow.browser.transformRequest.bind(flow.browser);
      let release, entered;
      const hold = new Promise(resolve => { release = resolve; });
      const ready = new Promise(resolve => { entered = resolve; });
      flow.browser.transformRequest = async request => { entered(); await hold; return original(request); };
      flow.resume = release;
      flow.pending = flow.write(flow.targets[0], flow.config.frames.changeC).then(
        () => 'escaped', error => error.code ?? String(error));
      await ready;
    });
    const c2 = await open({ device: 11, accountSeed: 10, reopen: true });
    assert.equal(await c2.evaluate(async () => (await flow.journal.beginLogout(
      flow.generation, canicProfile.nowNs() + 10_000_000_000n)).phase), 'cleanup');
    assert.equal(await c.evaluate(async () => { flow.resume(); return flow.pending; }), 'not_current');
    const cleaned = await c2.evaluate(async () => {
      const result = await canicProfile.cleanupUntilFinished({ journal: flow.journal, generation: flow.generation,
        browser: flow.browser, revoke: flow.adapter.revoke });
      return result.destinations.every(d => d.confirmedUntilNs !== undefined);
    });
    assert.equal(cleaned, true);
    await c2.close();
  });
  await check('logout fences a late real issuance result without admitting the receiver', async () => {
    const pending = await open({ device: 14, accountSeed: 10 });
    const before = await pending.evaluate(async () => {
      const issue = flow.options.transport.issue;
      let release, entered;
      const hold = new Promise(resolve => { release = resolve; });
      const ready = new Promise(resolve => { entered = resolve; });
      flow.options.transport.issue = async (...args) => {
        const result = await issue(...args);
        entered();
        await hold;
        return result;
      };
      // The coordinator deliberately captures its transport at construction.
      flow.coordinator = new canicProfile.SessionCoordinator(flow.options);
      flow.resume = release;
      flow.pending = flow.coordinator.ensureSession(flow.targets[0]).then(
        () => 'escaped', error => error.code ?? String(error));
      await ready;
      const status = await flow.journal.inspect();
      return { unknown: status.issuanceUnknown, counts: flow.counts, original: flow.original };
    });
    assert.equal(before.unknown, true); assert.equal(before.counts.issue, 1);
    assert.equal(before.counts.admit, 0);
    const cleanup = await open({ device: 14, accountSeed: 10, reopen: true });
    const revoked = await cleanup.evaluate(async () => {
      const status = await flow.journal.beginLogout(flow.generation, canicProfile.nowNs() + 10_000_000_000n);
      const step = await canicProfile.cleanupOnce({ journal: flow.journal, generation: flow.generation,
        browser: flow.browser, revoke: flow.adapter.revoke });
      return { phase: status.phase, deadline: String(status.deadlineNs), kind: step.kind };
    });
    assert.equal(revoked.phase, 'cleanup'); assert.equal(revoked.kind, 'settled');
    assert.equal(await pending.evaluate(async () => { flow.resume(); return flow.pending; }), 'not_current');
    const after = await cleanup.evaluate(async () => {
      const result = await canicProfile.cleanupUntilFinished({ journal: flow.journal, generation: flow.generation,
        browser: flow.browser, revoke: flow.adapter.revoke });
      const original = await flow.adapter.transport.reconcile(flow.account, 0n, AbortSignal.timeout(5000));
      return { phase: result.phase, deadline: String(result.deadlineNs),
        unknown: result.issuanceUnknown,
        confirmed: result.destinations.every(d => d.confirmedUntilNs !== undefined),
        original: { bytes: Array.from(original.bytes), untilNs: String(original.untilNs) },
        profile: Array.from(await flow.raw('profile_get', flow.targets[0], flow.browser, flow.config.frames.getC)) };
    });
    assert.equal(after.phase, 'finished'); assert.equal(after.confirmed, true);
    assert.equal(after.unknown, true, 'cleanup confirmation does not invent acceptance of the lost issuance result');
    assert.equal(after.deadline, revoked.deadline); assert.deepEqual(after.original, before.original);
    assert.deepEqual(after.profile, config.frames.unauthGet);
    assert.deepEqual(await pending.evaluate(() => flow.counts), { issue: 1, renew: 0, lookup: 0, admit: 0, writes: 0 });
    await pending.close(); await cleanup.close();
  });
  await check('replacement login preserves the owner while delayed old-key disposal stays bound to the old login', async () => {
    const replaced = await a.evaluate(async () => {
      const old = flow;
      const oldBinding = { generation: old.generation, caller: old.browser.getPrincipal().toText() };
      // Public fixed-seed fixture custody, not a production key store. Conditional
      // disposal belongs to the host; Auth supplies the exact terminal binding.
      let currentKey = { ...oldBinding, identity: old.browser };
      let release, entered, disposedBinding, disposedPhase;
      const hold = new Promise(resolve => { release = resolve; });
      const ready = new Promise(resolve => { entered = resolve; });
      const disposal = canicProfile.disposeLoginKey({ journal: old.journal, generation: old.generation,
        caller: old.browser.getPrincipal(), dispose: async (binding, status) => {
          disposedBinding = binding; disposedPhase = status.phase;
          entered(); await hold;
          if (currentKey?.generation === binding.generation && currentKey.caller === binding.caller)
            currentKey = undefined;
        } });
      await ready;
      let reuse;
      try { await old.journal.start({ generation: 'replacement', caller: canicProfile.key(15).getPrincipal(), issuer: old.issuer }); }
      catch (error) { reuse = error.code; }
      await canicProfile.setup(old.config, { device: 15 });
      currentKey = { generation: flow.generation, caller: flow.browser.getPrincipal().toText(), identity: flow.browser };
      const receipt = await flow.coordinator.ensureSession(flow.targets[0]);
      release(); await disposal;
      let early, invoked = false;
      try { await canicProfile.disposeLoginKey({ journal: flow.journal, generation: flow.generation,
        caller: flow.browser.getPrincipal(), dispose: async () => { invoked = true; } }); }
      catch (error) { early = error.code; }
      let stale;
      try { await old.coordinator.ensureSession(old.targets[0]); } catch (error) { stale = error.code; }
      // Replay actual old revocations after replacement admission. They must not
      // revoke the new login or its receiver session for the same account.
      const request = canicProfile.logoutCodec.encodeRequest(null);
      const issuerReply = await old.adapter.revoke(old.issuer, old.browser, request, AbortSignal.timeout(5000));
      const receiverReply = await old.adapter.revoke(old.targets[0], old.browser, request, AbortSignal.timeout(5000));
      return { reuse, oldBinding, disposedBinding, disposedPhase, early, invoked, stale,
        retained: currentKey?.identity.getPrincipal().toText(), caller: flow.browser.getPrincipal().toText(),
        subject: canicProfile.Principal.fromUint8Array(receipt.subject).toText(), account: flow.account.getPrincipal().toText(),
        issuerLogout: canicProfile.logoutCodec.decodeResponse(issuerReply).tag,
        receiverLogout: canicProfile.logoutCodec.decodeResponse(receiverReply).tag,
        oldStatus: (await old.journal.inspect()).phase, newStatus: (await flow.journal.inspect()).phase,
        oldRead: Array.from(await old.raw('profile_get', old.targets[0], old.browser, old.config.frames.getA)),
        own: Array.from(await flow.get(flow.targets[0], flow.config.frames.getA)),
        other: Array.from(await flow.get(flow.targets[0], flow.config.frames.getC)),
        counts: flow.counts };
    });
    assert.equal(replaced.reuse, 'login_exists');
    assert.deepEqual(replaced.disposedBinding, replaced.oldBinding); assert.equal(replaced.disposedPhase, 'finished');
    assert.equal(replaced.retained, replaced.caller); assert.notEqual(replaced.caller, replaced.oldBinding.caller);
    assert.equal(replaced.early, 'not_current'); assert.equal(replaced.invoked, false);
    assert.equal(replaced.stale, 'not_current'); assert.equal(replaced.subject, replaced.account);
    assert.equal(replaced.issuerLogout, 'Revoked'); assert.equal(replaced.receiverLogout, 'Revoked');
    assert.equal(replaced.oldStatus, 'finished'); assert.equal(replaced.newStatus, 'active');
    assert.deepEqual(replaced.oldRead, config.frames.unauthGet);
    assert.deepEqual(replaced.own, config.frames.profileAGet); assert.deepEqual(replaced.other, config.frames.forbiddenGet);
    assert.deepEqual(replaced.counts, { issue: 1, renew: 0, lookup: 0, admit: 1, writes: 0 });
  });
  await check('current Root revocation overrides cached browser readiness only at its selected receiver', async () => {
    const result = await a.evaluate(async () => {
      const target = flow.targets[1], selected = flow.config.authorities[1];
      const root = canicProfile.Principal.fromText(selected.canister), controller = canicProfile.key(13);
      await flow.coordinator.ensureSession(target);
      const before = Array.from(await flow.get(target, flow.config.frames.getA));
      const revoked = Array.from(await flow.raw('revoke_auth', root, controller, selected.revoke));
      // Root revocation is not instantly visible across canisters. Refresh through
      // the receiver's existing direct authority pull before asserting closure.
      const refreshed = Array.from(await flow.control(target, 1));
      const admissions = flow.counts.admit;
      const read = Array.from(await flow.get(target, flow.config.frames.getA));
      const write = Array.from(await flow.write(target, flow.config.frames.putA));
      const afterAdmissions = flow.counts.admit;
      const retry = Array.from(await flow.raw('configure_auth', root, controller, selected.configure));
      const retried = Array.from(await flow.control(target, 1));
      const stillClosed = Array.from(await flow.get(target, flow.config.frames.getA));
      const other = Array.from(await flow.get(flow.targets[0], flow.config.frames.getA));
      return { before, revoked, refreshed, read, write, admissions, afterAdmissions, retry, retried, stillClosed, other };
    });
    assert.deepEqual(result.before, config.frames.missingGet, 'authenticated read reached the empty application store');
    assert.deepEqual(result.revoked, config.frames.revokeOk); assert.deepEqual(result.refreshed, [1]);
    assert.deepEqual(result.read, config.frames.unauthGet); assert.deepEqual(result.write, config.frames.unauthPut);
    assert.equal(result.afterAdmissions, result.admissions, 'cached readiness cannot replace server authorization');
    assert.deepEqual(result.retry, config.frames.configureOk); assert.deepEqual(result.retried, [1]);
    assert.deepEqual(result.stillClosed, config.frames.unauthGet);
    assert.deepEqual(result.other, config.frames.profileAGet);
  });
  await context.close();
  console.log(`${cases} installed Canic profile/browser cases passed`);
}

// Let Playwright close its browser before the host's outer process deadline.
// The host owns Node directly; it does not claim ownership of browser descendants.
let timer;
try {
  await Promise.race([qualify(), new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error('installed browser deadline exceeded')), 75000);
  })]);
} finally {
  clearTimeout(timer);
  try { await browser?.close(); } finally { server.close(); await once(server, 'close'); }
}
