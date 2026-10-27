// Match the Rust identifier parsers; storage and authorization stay with their owners.
function canonicalHex(value: unknown): void {
  if (typeof value !== "string" || value.length !== 64 || !/^[0-9a-f]{64}$/.test(value)) {
    throw new Error("invalid canonical identity");
  }
}
export function canicNetworkId(value: unknown): void { canonicalHex(value); }
export function canicReleaseBuildId(value: unknown): void { canonicalHex(value); }
export function canicFleetId(value: unknown): void { canonicalHex(value); }
export function canicComponentInstanceId(value: unknown): void { canonicalHex(value); }
export function canicComponentSpecId(value: unknown): void {
  if (typeof value !== "string" || value.length === 0 || value.length > 40 ||
      /[^A-Za-z0-9_-]/.test(value)) {
    throw new Error("invalid component specification identity");
  }
}

export function canicDeploymentName(value: unknown): void {
  if (typeof value !== "string" || value.length === 0 || value.length > 40 ||
      /[^A-Za-z0-9_-]/.test(value)) {
    throw new Error("invalid component deployment identity");
  }
}
export function canicMemberPath(value: unknown): void {
  if (!Array.isArray(value) || value.length === 0 || value.length > 16) {
    throw new Error("invalid component member path");
  }
  // With at most sixteen forty-byte ASCII names the canonical byte bound follows.
  for (const member of value) canicDeploymentName(member);
}

export function canicLabelKey(value: unknown): void {
  if (typeof value !== "string" || value.length === 0 || value.length > 40 || /[^A-Za-z0-9_-]/.test(value)) {
    throw new Error("invalid deployment label key");
  }
}
export function canicLabelValue(value: unknown): void {
  if (typeof value !== "string" || value.length === 0 ||
      new TextEncoder().encode(value).length > 128 || /[\u0000-\u001f\u007f-\u009f]/.test(value)) {
    throw new Error("invalid deployment label value");
  }
}
