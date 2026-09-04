def item_kind($name):
  if ($name | test("^(data|elem|global|memory|table|type)\\[[0-9]+\\]"))
      or ($name | test(" section headers$"))
      or ($name | test("^(export|import) "))
      or ($name == "memory")
      or ($name == "section headers")
      or ($name == "wasm magic bytes") then
    "wasm_structural_and_abi"
  elif ($name | test("^code\\[[0-9]+\\]$")) then
    "unattributed_code"
  else
    "named_code"
  end;

def is_canic_owned($name):
  $name | test("(^|[^[:alnum:]_])(__canic|canic($|[^[:alnum:]_])|canic_(core|control_plane|metrics(_core|_runtime|_security)?|wasm_store))");

def category($name; $kind):
  if $kind != "named_code" then
    $kind
  elif ($name | test("k256|ecdsa|secp256|canister[_-]?sig|signature_verification|verify_bls|bls12|sha2|sha256"; "i")) then
    "cryptography"
  elif (is_canic_owned($name) | not) then
    "application_and_upstream"
  elif ($name | test("canic_metrics|::metrics::|metrics_(core|runtime|security)"; "i")) then
    "metrics"
  elif ($name | test("auth|delegat|admission|authori[sz]|caller|credential|permission|token|verif"; "i")) then
    "authentication_and_admission"
  elif ($name | test("canic_control_plane|child|placement|provision|topolog|management_canister|pool_ledger"; "i")) then
    "child_provisioning"
  else
    "canic_runtime"
  end;

def total_bytes($items):
  [$items[].shallow_size] | add // 0;

def category_record($items; $name; $artifact_bytes):
  [$items[] | select(.category == $name)] as $selected
  | (total_bytes($selected)) as $bytes
  | {
      category: $name,
      shallow_bytes: $bytes,
      item_count: ($selected | length),
      artifact_fraction:
        (if $artifact_bytes == 0 then null else ($bytes / $artifact_bytes) end),
      top_items:
        ($selected
          | sort_by(-.shallow_size, .name)
          | .[:20]
          | map({name, shallow_size}))
    };

. as $input
| [
    $input.items[]
    | (item_kind(.name)) as $kind
    | . + {
        item_kind: $kind,
        category: category(.name; $kind)
      }
  ] as $classified
| (total_bytes($classified)) as $measured_bytes
| ([$classified[] | select(.item_kind == "named_code") | .shallow_size] | add // 0) as $named_code_bytes
| ([$classified[] | select(.item_kind == "unattributed_code") | .shallow_size] | add // 0) as $unattributed_code_bytes
| ($named_code_bytes + $unattributed_code_bytes) as $code_bytes
| [
    "cryptography",
    "authentication_and_admission",
    "metrics",
    "child_provisioning",
    "canic_runtime",
    "application_and_upstream",
    "unattributed_code",
    "wasm_structural_and_abi"
  ] as $category_order
| {
    schema: "canic.wasm_capability_size.v1",
    artifact: $input.artifact,
    context: $input.context,
    analysis: {
      method: "twiggy_shallow_symbol_attribution",
      classification_revision: 2,
      tool: $input.tool,
      measured_bytes: $measured_bytes,
      artifact_bytes_match: ($measured_bytes == $input.artifact.bytes),
      named_code_bytes: $named_code_bytes,
      unattributed_code_bytes: $unattributed_code_bytes,
      named_code_fraction:
        (if $code_bytes == 0 then null else ($named_code_bytes / $code_bytes) end),
      symbol_attribution:
        (if $named_code_bytes == 0 and $unattributed_code_bytes > 0 then
           "unavailable"
         elif $unattributed_code_bytes == 0 then
           "complete"
         else
           "partial"
         end),
      limitations: [
        "Categories are disjoint shallow-byte ownership estimates from symbol names.",
        "cryptography is identified from retained symbol names and may include Canic, application, or upstream ownership.",
        "application_and_upstream combines remaining application code, dependencies, and shared generic instantiations.",
        "Stripped code[N] items remain unattributed instead of being assigned heuristically.",
        "Compare reports only when the build profile, toolchain, role capabilities, metrics tiers, and classification revision match."
      ]
    },
    categories:
      [$category_order[] | category_record($classified; .; $input.artifact.bytes)]
  }
