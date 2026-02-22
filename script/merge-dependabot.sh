#!/usr/bin/env bash
set -euo pipefail

REPO="${1:?Usage: $0 owner/repo [main]}"
BASE="${2:-main}"

SLEEP_SECS="${SLEEP_SECS:-20}"
TIMEOUT_SECS="${TIMEOUT_SECS:-1800}"

log() { printf '%s\n' "$*"; }

wait_until_ready() {
  pr="$1"
  start="$(date +%s)"

  while true; do
    now="$(date +%s)"
    elapsed=$((now - start))
    if [ "$elapsed" -gt "$TIMEOUT_SECS" ]; then
      log "Timeout waiting for PR #$pr"
      return 1
    fi

    state="$(gh pr view "$pr" -R "$REPO" --json mergeStateStatus -q '.mergeStateStatus')"
    log "PR #$pr state=$state"

    if [ "$state" = "BEHIND" ]; then
      gh pr update-branch "$pr" -R "$REPO" || true
      sleep "$SLEEP_SECS"
      continue
    fi

    if [ "$state" = "DIRTY" ]; then
      log "Conflicts in PR #$pr. Skipping."
      return 2
    fi

    if [ "$state" = "UNKNOWN" ] || [ "$state" = "UNSTABLE" ]; then
      sleep "$SLEEP_SECS"
      continue
    fi

    if [ "$state" = "CLEAN" ] || [ "$state" = "BLOCKED" ]; then
      conclusions="$(gh pr view "$pr" -R "$REPO" --json statusCheckRollup \
        -q '(.statusCheckRollup // []) | map(select(.conclusion != null) | .conclusion) | unique | .[]' || true)"

      if echo "$conclusions" | grep -q '^FAILURE$'; then
        log "Failing checks on PR #$pr"
        return 3
      fi

      if echo "$conclusions" | grep -Evq '^(SUCCESS|NEUTRAL|SKIPPED)$'; then
        sleep "$SLEEP_SECS"
        continue
      fi

      return 0
    fi

    sleep "$SLEEP_SECS"
  done
}

PRS="$(
  gh pr list -R "$REPO" --state open --base "$BASE" \
    --search 'author:app/dependabot OR author:dependabot[bot]' \
    --json number \
    -q '.[].number' \
  | sort -n
)"

if [ -z "$PRS" ]; then
  log "No open Dependabot PRs."
  exit 0
fi

log "Processing PRs: $PRS"

for pr in $PRS; do
  log "=== PR #$pr ==="

  if ! wait_until_ready "$pr"; then
    log "Skipping PR #$pr"
    continue
  fi

  gh pr merge "$pr" -R "$REPO" \
    --squash \
    --delete-branch \
    --admin

  log "Merged PR #$pr"
done