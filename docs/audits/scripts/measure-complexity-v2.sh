#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 || ! $1 =~ ^[0-9a-f]{40}$ ]]; then
  echo "usage: $0 <full-source-commit>" >&2
  exit 2
fi

commit=$1
repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"
git cat-file -e "${commit}^{commit}"

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT

files_path="$tmp_dir/files"
inventory_path="$tmp_dir/inventory.tsv"
refs_path="$tmp_dir/refs.tsv"

git ls-tree -r --name-only "$commit" -- crates/canic-core/src \
  | awk '/\.rs$/' \
  | LC_ALL=C sort >"$files_path"

fixed_symbols=(
  Request
  Response
  RequestFamily
  CapabilityProof
  CapabilityService
  BuiltinPredicate
  RootCapability
  RootCapabilityMetricKey
  RootCapabilityMetricEventType
  RootCapabilityMetricProofMode
  VerifyDelegatedTokenError
  ChainKeyRootProofError
  InternalErrorClass
  InfraError
  InternalError
  IcOps
  InternalErrorOrigin
  ConfigOps
  SubnetRegistryOps
  AuthStateOps
  DelegatedTokenConfig
  RootCapabilityMetricOutcome
  StateAllocationKey
)
symbol_list=${fixed_symbols[*]}

classify_file() {
  local relative=$1
  local basename=${relative##*/}
  local component

  if [[ $basename == test.rs || $basename == tests.rs || $basename == test_support.rs ]]; then
    printf 'test-support\ttest\n'
    return
  fi

  IFS='/' read -r -a components <<<"$relative"
  for component in "${components[@]}"; do
    if [[ $component == test || $component == tests ]]; then
      printf 'test-support\ttest\n'
      return
    fi
  done

  if [[ $relative != */* ]]; then
    printf 'root\tproduction\n'
    return
  fi

  case ${relative%%/*} in
    access | api | bootstrap | cdk | config | dispatch | domain | dto | format | ids | infra | ingress | lifecycle | memory | model | ops | storage | view | workflow)
      printf '%s\tproduction\n' "${relative%%/*}"
      ;;
    replay_policy)
      printf 'replay-policy\tproduction\n'
      ;;
    role_contract)
      printf 'role-contract\tproduction\n'
      ;;
    *)
      echo "unmapped complexity scope: $relative" >&2
      exit 3
      ;;
  esac
}

while IFS= read -r path; do
  relative=${path#crates/canic-core/src/}
  IFS=$'\t' read -r subsystem class < <(classify_file "$relative")
  blob_path="$tmp_dir/blob"
  analysis_path="$tmp_dir/analysis"
  git show "${commit}:${path}" >"$blob_path"

  awk -v symbols="$symbol_list" '
      BEGIN {
        count = split(symbols, ordered, " ")
        for (idx = 1; idx <= count; idx++) {
          wanted[ordered[idx]] = 1
        }
      }
      {
        line = $0
        trimmed = line
        sub(/^[[:space:]]+/, "", trimmed)
        if (trimmed == "" || trimmed ~ /^\/\//) {
          next
        }

        loc++
        tokenized = line
        gsub(/[^[:alnum:]_]/, " ", tokenized)
        token_count = split(tokenized, tokens, /[[:space:]]+/)
        previous = ""
        for (token_index = 1; token_index <= token_count; token_index++) {
          token = tokens[token_index]
          if (token == "") {
            continue
          }
          if (token == "match") {
            match_count++
          }
          if (token == "if") {
            if_count++
            if (previous == "else") {
              else_if_count++
            }
          }
          if (token in wanted) {
            seen[token] = 1
          }
          if (token ~ /^Capability[[:alnum:]_]*$/) {
            capability = 1
          }
          previous = token
        }
      }
      END {
        printf "METRICS\t%d\t%d\t%d\t%d\t%d\n", loc, match_count, if_count, else_if_count, capability
        for (idx = 1; idx <= count; idx++) {
          symbol = ordered[idx]
          if (seen[symbol]) {
            printf "REF\t%s\n", symbol
          }
        }
        if (capability) {
          print "CAPABILITY"
        }
      }
    ' "$blob_path" >"$analysis_path"

  while IFS=$'\t' read -r kind first second third fourth fifth; do
    if [[ $kind == METRICS ]]; then
      printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$path" "$subsystem" "$class" "$first" "$second" "$third" \
        "$fourth" "$fifth" "$((second + third + fourth))" >>"$inventory_path"
    elif [[ $kind == REF ]]; then
      printf '%s\t%s\t%s\n' "$first" "$path" "$class" >>"$refs_path"
    elif [[ $kind == CAPABILITY ]]; then
      printf 'CapabilityMention\t%s\t%s\n' "$path" "$class" >>"$refs_path"
    fi
  done <"$analysis_path"
done <"$files_path"

echo -e "identity\tvalue"
echo -e "method\tCANIC-COMPLEXITY-001/v2"
echo -e "source_commit\t$commit"

awk -F '\t' '
  BEGIN {
    print "metric\tvalue"
  }
  {
    files++
    loc += $4
    if ($4 >= 600) large++
    if ($3 == "production") {
      production_files++
      production_loc += $4
      if ($4 >= 600) production_large++
    }
  }
  END {
    print "total_files\t" files
    print "total_logical_loc\t" loc
    print "files_at_least_600_loc\t" large
    print "non_test_files\t" production_files
    print "non_test_logical_loc\t" production_loc
    print "non_test_files_at_least_600_loc\t" production_large
  }
' "$inventory_path"

echo -e "subsystem\tfiles\tlogical_loc"
awk -F '\t' '{ files[$2]++; loc[$2] += $4 } END { for (name in files) print name "\t" files[name] "\t" loc[name] }' "$inventory_path" \
  | LC_ALL=C sort

echo -e "large_file\tclass\tlogical_loc\tmatch\tif\telse_if\tbranch_density"
awk -F '\t' '$4 >= 600 { printf "%s\t%s\t%d\t%d\t%d\t%d\t%.2f\n", $1, $3, $4, $5, $6, $7, (($9 / $4) * 100) }' "$inventory_path" \
  | LC_ALL=C sort

echo -e "reference_symbol\tall_files\tnon_test_files"
for symbol in "${fixed_symbols[@]}" CapabilityMention; do
  awk -F '\t' -v symbol="$symbol" '
    $1 == symbol {
      all++
      if ($3 == "production") production++
    }
    END { print symbol "\t" (all + 0) "\t" (production + 0) }
  ' "$refs_path"
done
