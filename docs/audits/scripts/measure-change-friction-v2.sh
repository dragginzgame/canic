#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 || ! $1 =~ ^[0-9a-f]{40}$ ]]; then
  echo "usage: $0 <full-source-commit>" >&2
  exit 2
fi

baseline=$1
repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"
git cat-file -e "${baseline}^{commit}"

fixture=docs/audits/fixtures/change-friction-v2-sample.tsv
expected_header=$'order\tcommit\tparent\tslice_type\tlabel\tflow_axes'
if [[ ! -f $fixture || $(head -n 1 "$fixture") != "$expected_header" ]]; then
  echo "invalid change-friction v2 sample fixture" >&2
  exit 3
fi

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT
inventory=$tmp_dir/inventory.tsv
slice_metrics=$tmp_dir/slice-metrics.tsv
scope_files=$tmp_dir/scope-files

classify_file() {
  local relative=$1
  local basename=${relative##*/}
  local component
  local subsystem
  local class=production
  local layer

  if [[ $basename == test.rs || $basename == tests.rs || $basename == test_support.rs ]]; then
    printf 'test-support\ttest\ttest\n'
    return
  fi

  IFS='/' read -r -a components <<<"$relative"
  for component in "${components[@]}"; do
    if [[ $component == test || $component == tests ]]; then
      printf 'test-support\ttest\ttest\n'
      return
    fi
  done

  if [[ $relative != */* ]]; then
    subsystem=root
    case $relative in
      control_plane_support.rs | protocol.rs | lib.rs)
        layer=endpoints
        ;;
      log.rs | perf.rs)
        layer=ops
        ;;
      error.rs | memory_macros.rs | shared_support.rs | state_contract.rs)
        layer=model-storage
        ;;
      *)
        echo "unmapped root change-friction scope: $relative" >&2
        exit 3
        ;;
    esac
    printf '%s\t%s\t%s\n' "$subsystem" "$class" "$layer"
    return
  fi

  case ${relative%%/*} in
    access | api | dispatch | dto | format | ingress)
      subsystem=${relative%%/*}
      layer=endpoints
      ;;
    bootstrap | lifecycle | workflow)
      subsystem=${relative%%/*}
      layer=workflow
      ;;
    config)
      subsystem=config
      layer=policy
      ;;
    replay_policy)
      subsystem=replay-policy
      layer=policy
      ;;
    role_contract)
      subsystem=role-contract
      layer=policy
      ;;
    domain)
      subsystem=domain
      if [[ $relative == domain/policy/* ]]; then
        layer=policy
      else
        layer=model-storage
      fi
      ;;
    cdk | infra | ops)
      subsystem=${relative%%/*}
      layer=ops
      ;;
    ids | memory | model | storage | view)
      subsystem=${relative%%/*}
      layer=model-storage
      ;;
    *)
      echo "unmapped change-friction scope: $relative" >&2
      exit 3
      ;;
  esac

  printf '%s\t%s\t%s\n' "$subsystem" "$class" "$layer"
}

git ls-tree -r --name-only "$baseline" -- crates/canic-core/src \
  | awk '/\.rs$/' \
  | LC_ALL=C sort >"$scope_files"
while IFS= read -r path; do
  relative=${path#crates/canic-core/src/}
  classify_file "$relative" >/dev/null
done <"$scope_files"
current_scope_files=$(wc -l <"$scope_files")
if [[ $current_scope_files -eq 0 ]]; then
  echo "change-friction v2 current scope is empty" >&2
  exit 3
fi

expected_order=1
while IFS=$'\t' read -r order commit parent slice_type label flow_axes extra; do
  [[ -z ${extra:-} ]] || {
    echo "unexpected extra sample field at order $order" >&2
    exit 3
  }
  [[ $order =~ ^[0-9]+$ && $order -eq $expected_order ]] || {
    echo "non-contiguous sample order: $order" >&2
    exit 3
  }
  [[ $commit =~ ^[0-9a-f]{40}$ && $parent =~ ^[0-9a-f]{40}$ ]] || {
    echo "sample commits must be full identities at order $order" >&2
    exit 3
  }
  [[ $slice_type == feature_slice || $slice_type == release_sweep ]] || {
    echo "invalid slice type at order $order: $slice_type" >&2
    exit 3
  }
  [[ $label =~ ^[a-z0-9_]+$ && $flow_axes =~ ^[a-z0-9_]+(,[a-z0-9_]+)*$ ]] || {
    echo "invalid label or flow-axis list at order $order" >&2
    exit 3
  }
  git cat-file -e "${commit}^{commit}"
  git cat-file -e "${parent}^{commit}"
  [[ $(git rev-parse "${commit}^") == "$parent" ]] || {
    echo "fixture parent is not the first parent at order $order" >&2
    exit 3
  }
  git merge-base --is-ancestor "$commit" "$baseline" || {
    echo "sample commit is not an ancestor of baseline at order $order" >&2
    exit 3
  }

  axis_count=$(awk -F ',' '{ print NF }' <<<"$flow_axes")
  while IFS= read -r path; do
    [[ -n $path ]] || continue
    relative=${path#crates/canic-core/src/}
    IFS=$'\t' read -r subsystem class layer < <(classify_file "$relative")
    if [[ $relative == */* ]]; then
      module=${relative%/*}
    else
      module=root
    fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
      "$order" "$commit" "$parent" "$slice_type" "$label" "$axis_count" \
      "$path" "$subsystem" "$class" "$layer" "$module" "$flow_axes" \
      >>"$inventory"
  done < <(git diff --name-only "$parent" "$commit" -- crates/canic-core/src \
    | awk '/\.rs$/' \
    | LC_ALL=C sort)

  if ! awk -F '\t' -v order="$order" '$1 == order { found = 1 } END { exit !found }' "$inventory"; then
    echo "sample contains no canic-core Rust files at order $order" >&2
    exit 3
  fi
  expected_order=$((expected_order + 1))
done < <(tail -n +2 "$fixture")

if [[ $expected_order -ne 6 ]]; then
  echo "change-friction v2 requires exactly five frozen sample rows" >&2
  exit 3
fi

echo -e "identity\tvalue"
echo -e "method\tCANIC-CHANGE-FRICTION-001/v2"
echo -e "source_commit\t$baseline"
echo -e "fixture\t$fixture"
echo -e "current_scope_files_mapped\t$current_scope_files"

echo -e "order\tcommit\ttype\tlabel\tfiles\tsubsystems\tlayers\taxes\tcaf\tdensity\tels\tlocality\tcontainment"
for order in 1 2 3 4 5; do
  row=$tmp_dir/row-$order.tsv
  awk -F '\t' -v order="$order" '$1 == order' "$inventory" >"$row"

  commit=$(awk -F '\t' 'NR == 1 { print $2 }' "$row")
  slice_type=$(awk -F '\t' 'NR == 1 { print $4 }' "$row")
  label=$(awk -F '\t' 'NR == 1 { print $5 }' "$row")
  axes=$(awk -F '\t' 'NR == 1 { print $6 }' "$row")
  files=$(wc -l <"$row")
  subsystems=$(awk -F '\t' '{ print $8 }' "$row" | LC_ALL=C sort -u | wc -l)
  layers=$(awk -F '\t' '$9 == "production" { print $10 }' "$row" | LC_ALL=C sort -u | wc -l)
  primary_subsystem_files=$(awk -F '\t' '{ count[$8]++ } END { for (name in count) print name "\t" count[name] }' "$row" \
    | LC_ALL=C sort -t $'\t' -k2,2nr -k1,1 \
    | awk -F '\t' 'NR == 1 { print $2 }')
  primary_module_files=$(awk -F '\t' '{ count[$11]++ } END { for (name in count) print name "\t" count[name] }' "$row" \
    | LC_ALL=C sort -t $'\t' -k2,2nr -k1,1 \
    | awk -F '\t' 'NR == 1 { print $2 }')
  if (( subsystems > layers )); then
    amplification=$subsystems
  else
    amplification=$layers
  fi
  caf=$((amplification * axes))

  awk -v order="$order" -v commit="$commit" -v type="$slice_type" \
    -v label="$label" -v files="$files" -v subsystems="$subsystems" \
    -v layers="$layers" -v axes="$axes" -v caf="$caf" \
    -v primary_subsystem_files="$primary_subsystem_files" \
    -v primary_module_files="$primary_module_files" \
    'BEGIN {
      printf "%d\t%s\t%s\t%s\t%d\t%d\t%d\t%d\t%d\t%.3f\t%.3f\t%.3f\t%.3f\n",
        order, commit, type, label, files, subsystems, layers, axes, caf,
        files / subsystems, primary_subsystem_files / files,
        primary_module_files / files, subsystems / 23
    }' | tee -a "$slice_metrics"
done

echo -e "aggregate\tvalue"
awk -F '\t' '
  { files[NR] = $5; sum += $5; if ($9 > max_caf) max_caf = $9 }
  END {
    n = NR
    for (i = 1; i <= n; i++) {
      for (j = i + 1; j <= n; j++) {
        if (files[i] > files[j]) {
          tmp = files[i]; files[i] = files[j]; files[j] = tmp
        }
      }
    }
    if (n % 2 == 1) median = files[(n + 1) / 2]
    else median = (files[n / 2] + files[(n / 2) + 1]) / 2
    rank = int((95 * n + 99) / 100)
    printf "average_files\t%.3f\n", sum / n
    printf "median_files\t%.3f\n", median
    printf "p95_files_nearest_rank\t%d\n", files[rank]
    printf "max_caf\t%d\n", max_caf
  }
' "$slice_metrics"

echo -e "file_map\torder\tpath\tsubsystem\tclass\tlayer\tmodule"
awk -F '\t' '{ print "file_map\t" $1 "\t" $7 "\t" $8 "\t" $9 "\t" $10 "\t" $11 }' "$inventory"
