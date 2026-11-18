// Fixed consumer of the actual Canic adapter and Auth lifecycle. No journal imitation.
import { LoginJournal, SessionCoordinator, retrieveCredential, establishSession, cleanupOnce,
  cleanupUntilFinished, disposeLoginKey, httpUpdateTransport } from '@dragginzgame/ic-auth-client';
import { Ed25519KeyIdentity } from '@tokostudios/ic-platform/identity';
import { Principal } from '@tokostudios/ic-platform/principal';
import { httpIssuerAdapter } from '../../../crates/canic/browser/src/issuer.ts';
import { codec as logoutCodec } from '../../../crates/canic/browser/src/generated/receiver-logout.ts';

const key = seed => Ed25519KeyIdentity.generate(new Uint8Array(32).fill(seed));
const nowNs = () => BigInt(Date.now()) * 1_000_000n;
const bounded = () => AbortSignal.timeout(5000);
const counts = () => ({ issue: 0, renew: 0, lookup: 0, admit: 0, writes: 0 });

async function setup(config, { device = 9, accountSeed = 8, reopen = false,
  loseIssue = false, loseAdmission = false, scopes = ['read', 'write'] } = {}) {
  const account = key(accountSeed), browser = key(device), issuer = Principal.fromText(config.issuer);
  const targets = config.receivers.map(value => Principal.fromText(value));
  const generation = `device-${device}`, name = `canic-profile-${device}`;
  const journal = await LoginJournal.open(indexedDB, name, { id: 'fixture-unix', nowNs });
  if (!reopen) await journal.start({ generation, caller: browser.getPrincipal(), issuer });
  const trust = { host: new URL(config.gateway), rootKey: new Uint8Array(config.root) };
  const adapter = httpIssuerAdapter({ issuer, account: account.getPrincipal(), browserKey: browser.getPublicKey(),
    deadlineOffsetNs: -1_000_000_000n,
    intent: { epoch: 1n, operation_id: new Array(32).fill(device), login_epoch: 1n,
      login_id: new Array(32).fill(device), browser_public_key: [...browser.getPublicKey().rawKey],
      grants: [{ target: 'user', scopes: ['read', 'write'] }] } }, trust);
  const flow = { config, account, browser, issuer, targets, generation, journal, adapter,
    counts: counts(), admission: undefined, original: undefined, receipt: undefined };
  const raw = methodName => httpUpdateTransport({ ...trust, methodName });
  const issue = adapter.transport.issue;
  const transport = {
    issue: async (...args) => {
      flow.counts.issue++;
      const result = await issue(...args);
      flow.original = { bytes: Array.from(result.bytes), untilNs: String(result.untilNs) };
      if (loseIssue) { loseIssue = false; throw new Error('lost issuer reply'); }
      return result;
    },
    renew: async (...args) => { flow.counts.renew++; return adapter.transport.renew(...args); },
    reconcile: async (...args) => { flow.counts.lookup++; return adapter.transport.reconcile(...args); },
  };
  const options = { journal, generation, browser, account, issuer, transport,
    scopes, ttlSecs: 60n,
    admit: async (...args) => {
      flow.counts.admit++;
      flow.admission = new Uint8Array(args[2]);
      const bytes = await raw('establish_portable_session')(...args);
      if (loseAdmission) { loseAdmission = false; throw new Error('lost admission reply'); }
      return bytes;
    },
    refreshAt: receipt => receipt.expires_at_ns - 1_000_000_000n,
  };
  flow.options = options;
  flow.coordinator = new SessionCoordinator(options);
  flow.raw = (method, target, identity, request) => raw(method)(target, identity, new Uint8Array(request), bounded());
  flow.write = async (target, frame) => flow.coordinator.call(target, (identity, signal) => {
    flow.counts.writes++;
    return raw('profile_put')(target, identity, new Uint8Array(frame), signal);
  });
  flow.get = async (target, frame) => flow.coordinator.call(target, (identity, signal) =>
    raw('profile_get')(target, identity, new Uint8Array(frame), signal));
  flow.control = (target, value) => flow.raw('control', target, key(13), [value]);
  globalThis.flow = flow;
}

globalThis.canicProfile = { setup, key, nowNs, retrieveCredential, establishSession, SessionCoordinator,
  cleanupOnce, cleanupUntilFinished, disposeLoginKey, logoutCodec, Principal };
