import { httpUpdateTransport, type HttpUpdateOptions, type IssuedCredential,
  type IssuerTransport, type LogoutTransport } from '@dragginzgame/ic-auth-client';
import type { Identity } from '@tokostudios/ic-platform/agent';
import type { Ed25519KeyIdentity } from '@tokostudios/ic-platform/identity';
import { Principal } from '@tokostudios/ic-platform/principal';
import { codec as issueCodec, type Request as Issue, type Response } from './generated/issue.js';
import { codec as renewCodec } from './generated/renew.js';
import { codec as lookupCodec } from './generated/reconcile.js';
import { codec as logoutCodec } from './generated/logout.js';
import { codec as receiverLogout } from './generated/receiver-logout.js';
import { codec as credentialCodec } from './generated/credential.js';

/** Immutable application selection, not authority obtained from request fields. */
export interface IssuerBinding {
  issuer: Principal;
  account: Principal;
  /** SDK public key from the selected device; the adapter never takes its signing key. */
  browserKey: ReturnType<Ed25519KeyIdentity['getPublicKey']>;
  intent: Issue;
  /** Explicit conservative journal-minus-issuer clock offset. The application
   * must establish this bound; Canic never estimates it from response arrival.
   * Persist the same offset with the original login across reloads. */
  deadlineOffsetNs: bigint;
}
/** One authenticated raw update. The caller owns trust and cancellation. */
export type IssuerUpdate = (method: string, target: Principal, identity: Identity,
  request: Uint8Array, signal: AbortSignal) => Promise<Uint8Array>;
export type IssuerFailure = 'binding' | 'sequence' | 'unresolved' | 'refused' | 'deadline';
export class IssuerError extends Error {
  readonly code: IssuerFailure;
  constructor(code: IssuerFailure) { super(`Canic issuer: ${code}`); this.code = code; }
}
export interface IssuerAdapter {
  transport: IssuerTransport;
  /** Supply to Auth cleanup. Receiver logout retains its unit-input contract. */
  revoke: LogoutTransport;
}

/** Bind Canic schemas to Auth's existing coordinator. No journal, scheduler,
 * signer, retry or issue-after-lookup fallback is introduced here. */
export function issuerAdapter(binding: IssuerBinding, update: IssuerUpdate): IssuerAdapter {
  const issuer = binding.issuer.toText(), account = binding.account.toText();
  const browser = Principal.selfAuthenticating(binding.browserKey.toDer()).toText();
  const offset = binding.deadlineOffsetNs;
  if ([binding.issuer, binding.account].some(p => p.isAnonymous())
    || typeof offset !== 'bigint') throw new IssuerError('binding');
  // Codec round-trip owns nested grants and identity arrays, before any await.
  const issueBytes = issueCodec.encodeRequest(binding.intent);
  const intent = issueCodec.decodeRequest(issueBytes);
  const raw = binding.browserKey.rawKey;
  if (intent.browser_public_key.length !== raw.length
    || !intent.browser_public_key.every((byte, index) => byte === raw[index]))
    throw new IssuerError('binding');
  const selectors = { account: Principal.fromText(account).toUint8Array(),
    login_epoch: intent.login_epoch, login_id: intent.login_id };
  const check = (identity: Identity, expected: string, signal: AbortSignal) => {
    signal.throwIfAborted();
    if (identity.getPrincipal().toText() !== expected) throw new IssuerError('binding');
  };
  const deadline = (original: bigint) => {
    const mapped = original + offset;
    if (original <= 0n || mapped <= 0n || mapped > 0xffff_ffff_ffff_ffffn)
      throw new IssuerError('deadline');
    return mapped;
  };
  const result = (reply: Response): IssuedCredential => {
    if ('err' in reply) throw new IssuerError('refused');
    if (reply.ok.tag === 'Unresolved') throw new IssuerError('unresolved');
    // Only the outer supplier envelope is framed again. Canonical claims and
    // proof bytes are retained exactly; receiver verification remains mandatory.
    return { bytes: credentialCodec.encodeRequest(reply.ok.value.credential),
      untilNs: deadline(reply.ok.value.admission_deadline_ns) };
  };
  const send = async (method: string, identity: Identity, expected: string,
    bytes: Uint8Array, signal: AbortSignal) => {
    check(identity, expected, signal);
    const reply = await update(method, Principal.fromText(issuer), identity, bytes, signal);
    check(identity, expected, signal);
    return reply;
  };
  const renewal = (sequence: bigint) => {
    if (sequence <= 0n || sequence > 0xffff_ffff_ffff_ffffn) throw new IssuerError('sequence');
    return { ...selectors, sequence, grants: intent.grants };
  };
  return {
    transport: {
      issue: async (identity, signal) => result(issueCodec.decodeResponse(await send(
        'canic_issue_portable_credential', identity, account, new Uint8Array(issueBytes), signal))),
      renew: async (identity, sequence, signal) => result(renewCodec.decodeResponse(await send(
        'canic_renew_portable_credential', identity, browser,
        renewCodec.encodeRequest(renewal(sequence)), signal))),
      reconcile: async (identity, sequence, signal) => {
        const request = sequence === 0n
          ? lookupCodec.encodeRequest({ tag: 'Issue', value: {
            epoch: intent.epoch, operation_id: intent.operation_id } })
          : lookupCodec.encodeRequest({ tag: 'Renew', value: renewal(sequence) });
        return result(lookupCodec.decodeResponse(await send('canic_reconcile_portable_credential',
          identity, sequence === 0n ? account : browser, request, signal)));
      },
    },
    revoke: async (target, identity, request, signal) => {
      check(identity, browser, signal);
      receiverLogout.decodeRequest(request);
      if (target.toText() !== issuer) {
        // Auth's journal supplies the admitted receiver. No application permission
        // is inferred here, and its original unit request/reply stay unchanged.
        const reply = await update('canic_logout_portable_session', target, identity, request, signal);
        check(identity, browser, signal);
        return reply;
      }
      const reply = logoutCodec.decodeResponse(await send('canic_revoke_portable_login', identity,
        browser, logoutCodec.encodeRequest(selectors), signal));
      if ('err' in reply) throw new IssuerError('refused');
      if (reply.ok.login_deadline_ns === 0n) throw new IssuerError('deadline');
      // Auth keeps acknowledgement timestamps in the destination clock domain;
      // this is not its separately selected local cleanup deadline.
      return receiverLogout.encodeResponse({ tag: 'Revoked', value: reply.ok.login_deadline_ns });
    },
  };
}

/** Real Auth HTTP updates with explicit gateway/root trust and no automatic retry.
 * Feed the returned methods to Auth's SessionCoordinator and cleanup functions. */
export function httpIssuerAdapter(binding: IssuerBinding,
  options: Omit<HttpUpdateOptions, 'methodName'>): IssuerAdapter {
  // Capture transport trust now, not from a mutable object during a later call.
  const methods = new Map(['canic_issue_portable_credential', 'canic_renew_portable_credential',
    'canic_reconcile_portable_credential', 'canic_revoke_portable_login', 'canic_logout_portable_session']
    .map(methodName => [methodName, httpUpdateTransport({ ...options, methodName })]));
  return issuerAdapter(binding, (method, target, identity, request, signal) => {
    const transport = methods.get(method);
    if (!transport) throw new IssuerError('binding');
    return transport(target, identity, request, signal);
  });
}
