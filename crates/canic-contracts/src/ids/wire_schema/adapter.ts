import { createCodec as base, WireError } from "./wire-runtime.js";
import { canicNetworkId, canicReleaseBuildId, canicFleetId, canicComponentInstanceId, canicComponentSpecId, canicDeploymentName, canicMemberPath, canicLabelKey, canicLabelValue } from "./identities.js";
export { WireError };
const identities = { canicNetworkId, canicReleaseBuildId, canicFleetId, canicComponentInstanceId, canicComponentSpecId, canicDeploymentName, canicMemberPath, canicLabelKey, canicLabelValue };
export function createCodec(
  method: unknown,
  types: unknown = {},
  checks: Readonly<Record<string, (value: unknown) => void>> = {},
) {
  for (const name of Object.keys(identities)) {
    if (name in checks) throw new WireError("value");
  }
  return base(method, types, { ...checks, ...identities });
}
