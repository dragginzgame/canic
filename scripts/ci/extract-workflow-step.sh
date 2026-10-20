#!/usr/bin/env bash
set -euo pipefail
# Read only: callers own YAML selection, workflow policy and script execution.
if [[ $# != 2 || -z "$2" ]]; then
    echo 'usage: extract-workflow-step.sh STEPS_JSON STEP_ID' >&2
    exit 2
fi
# Slurp before publishing so malformed trailing input cannot expose a partial
# script. A step ID must identify exactly one executable run body, not an action.
jq -ers --arg step "$2" '
    if length != 1 then error("expected one JSON document")
    elif (.[0] | type) != "array" then error("expected a workflow step array")
    else .[0]
    end
    | if all(.[]; type == "object") then . else error("expected step objects") end
    | map(select(.id == $step))
    | if length == 1 then .[0] else error("expected exactly one matching step") end
    | if has("uses") then error("selected step is an action")
      elif (.run | type) != "string" then error("selected step has no run body")
      elif (.run | test("\\S") | not) then error("selected run body is empty")
      else .run
      end
' -- "$1"
