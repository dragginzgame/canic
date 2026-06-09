#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat >&2 <<'USAGE'
usage: scripts/dev/gh-ci.sh [--branch <branch>] [--workflow <name>] [--run <id>] [--failed] [--logs] [--list] [--limit <n>]

Inspect GitHub Actions CI through an authenticated local GitHub CLI session.

Options:
  --branch <branch>   Branch or tag name to inspect. Defaults to the current git branch.
  --workflow <name>   Workflow name to inspect. Defaults to CI.
  --run <id>          Inspect a specific workflow run id.
  --failed            Select the latest failed run for the branch.
  --logs              Print failed-step logs after the run summary.
  --list              List recent runs instead of opening one run.
  --limit <n>         Number of runs to list. Defaults to 10.
  -h, --help          Show this help.

Examples:
  scripts/dev/gh-ci.sh
  scripts/dev/gh-ci.sh --failed --logs
  scripts/dev/gh-ci.sh --branch main --list
  scripts/dev/gh-ci.sh --run 123456789 --logs
USAGE
}

require_command() {
    local command_name="$1"

    if ! command -v "$command_name" >/dev/null 2>&1; then
        echo "missing required tool: $command_name" >&2
        exit 1
    fi
}

require_gh_auth() {
    if ! gh auth status >/dev/null 2>&1; then
        echo "gh is not authenticated; run: gh auth login -h github.com --git-protocol https --web --scopes repo,workflow" >&2
        exit 1
    fi
}

current_branch() {
    git branch --show-current 2>/dev/null || true
}

resolve_run_id() {
    local workflow="$1"
    local branch="$2"
    local status_filter="$3"
    local -a args=(
        run list
        --workflow "$workflow"
        --limit 1
        --json databaseId
        --jq '.[0].databaseId // ""'
    )

    if [ -n "$branch" ]; then
        args+=(--branch "$branch")
    fi

    if [ -n "$status_filter" ]; then
        args+=(--status "$status_filter")
    fi

    gh "${args[@]}"
}

workflow="CI"
branch=""
run_id=""
status_filter=""
print_logs=0
list_runs=0
limit=10

while [ "$#" -gt 0 ]; do
    case "$1" in
        --branch)
            if [ "$#" -lt 2 ]; then
                usage
                exit 2
            fi
            branch="$2"
            shift 2
            ;;
        --workflow)
            if [ "$#" -lt 2 ]; then
                usage
                exit 2
            fi
            workflow="$2"
            shift 2
            ;;
        --run)
            if [ "$#" -lt 2 ]; then
                usage
                exit 2
            fi
            run_id="$2"
            shift 2
            ;;
        --failed)
            status_filter="failure"
            shift
            ;;
        --logs)
            print_logs=1
            shift
            ;;
        --list)
            list_runs=1
            shift
            ;;
        --limit)
            if [ "$#" -lt 2 ]; then
                usage
                exit 2
            fi
            limit="$2"
            shift 2
            ;;
        -h | --help)
            usage
            exit 0
            ;;
        *)
            echo "unknown argument: $1" >&2
            usage
            exit 2
            ;;
    esac
done

require_command gh
require_command git
require_gh_auth

if [ -z "$branch" ]; then
    branch="$(current_branch)"
fi

if [ "$list_runs" -eq 1 ]; then
    list_args=(run list --workflow "$workflow" --limit "$limit")
    if [ -n "$branch" ]; then
        list_args+=(--branch "$branch")
    fi
    if [ -n "$status_filter" ]; then
        list_args+=(--status "$status_filter")
    fi

    gh "${list_args[@]}"
    exit 0
fi

if [ -z "$run_id" ]; then
    run_id="$(resolve_run_id "$workflow" "$branch" "$status_filter")"
fi

if [ -z "$run_id" ]; then
    if [ -n "$branch" ]; then
        echo "no matching $workflow runs found for $branch" >&2
    else
        echo "no matching $workflow runs found" >&2
    fi
    exit 1
fi

gh run view "$run_id" --verbose

if [ "$print_logs" -eq 1 ]; then
    gh run view "$run_id" --log-failed
fi
