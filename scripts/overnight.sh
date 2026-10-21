#!/usr/bin/env bash
# Overnight agent harness: works through GitHub issues in roadmap order, one
# headless Claude Code run per issue, each in an isolated git worktree. Changes
# that pass the full check gate are pushed to fix/issue-<n> and opened as a PR.
#
#   scripts/overnight.sh --dry-run         # show the queue, change nothing
#   scripts/overnight.sh --only 79 --no-push   # one issue, commit locally only
#   scripts/overnight.sh                   # the real thing
#
# Stop gracefully: touch "$LOG_ROOT/STOP" (checked between issues).
# Details: context/testing.md#overnight-harness
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WT="${WT:-$REPO/../gecko-overnight}"                 # dedicated worktree
LOG_ROOT="${LOG_ROOT:-${XDG_STATE_HOME:-$HOME/.local/state}/gecko-overnight}"
ISSUE_TIMEOUT="${ISSUE_TIMEOUT:-3h}"                 # wall clock per agent run
MAX_ATTEMPTS="${MAX_ATTEMPTS:-10}"                   # rate-limit retries per issue
LIMIT_SLEEP="${LIMIT_SLEEP:-900}"                    # fallback wait when no reset time is given
REPAIR_ROUNDS="${REPAIR_ROUNDS:-1}"                  # times the agent may fix a failed gate
CLAUDE_BIN="${CLAUDE_BIN:-claude}"
CLAUDE_EXTRA_ARGS="${CLAUDE_EXTRA_ARGS:-}"           # e.g. "--model claude-opus-5-5"
export PATH="$HOME/.cargo/bin:$PATH"

DRY_RUN=false; PUSH=true; ONLY=""; MAX_ISSUES=0
while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) DRY_RUN=true ;;
    --no-push) PUSH=false ;;
    --only) ONLY="${2//,/ }"; shift ;;
    --max) MAX_ISSUES="$2"; shift ;;
    -h|--help) sed -n '2,12p' "$0"; exit 0 ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
  shift
done

RUN_DIR="$LOG_ROOT/$(date '+%Y%m%d-%H%M%S')"
SUMMARY="$RUN_DIR/summary.tsv"
NWO=""

log() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$RUN_DIR/harness.log"; }
record() { printf '%s\t%s\t%s\t%s\n' "$1" "$2" "$3" "$4" >> "$SUMMARY"; }  # issue outcome detail cost

preflight() {
  local missing=""
  for c in git gh jq timeout cargo npm "$CLAUDE_BIN"; do command -v "$c" >/dev/null || missing+=" $c"; done
  [ -z "$missing" ] || { echo "missing tools:$missing" >&2; exit 1; }
  gh auth status >/dev/null 2>&1 || { echo "gh is not authenticated (gh auth login)" >&2; exit 1; }
  NWO="$(gh repo view --json nameWithOwner --jq .nameWithOwner)" || exit 1
  git -C "$REPO" fetch -q origin main || { echo "git fetch failed" >&2; exit 1; }
  # Agents only see what is on origin/main.
  local ahead dirty
  ahead="$(git -C "$REPO" rev-list --count origin/main..main 2>/dev/null || echo 0)"
  dirty="$(git -C "$REPO" status --porcelain -- AGENTS.md CLAUDE.md context research | head -5)"
  [ "$ahead" = 0 ] || echo "WARNING: local main is $ahead commit(s) ahead of origin/main; agents will not see them." >&2
  [ -z "$dirty" ] || printf 'WARNING: uncommitted agent docs (agents will not see these):\n%s\n' "$dirty" >&2
}

# Issue numbers in roadmap order (falls back to ascending issue number).
queue() {
  local open order
  open="$(gh issue list --state open --limit 500 --json number --jq '.[].number' | sort -n)"
  if [ -f "$REPO/context/roadmap.md" ]; then
    order="$(grep -oE '^[0-9]+\. \[ \] \[#[0-9]+\]' "$REPO/context/roadmap.md" | grep -oE '#[0-9]+' | tr -d '#')"
  else
    order="$open"
  fi
  [ -n "$ONLY" ] && order="$(printf '%s\n' $ONLY)"
  for n in $order; do grep -qx "$n" <<<"$open" && echo "$n"; done
}

open_blockers() {
  gh api "repos/$NWO/issues/$1/dependencies/blocked_by" --jq '[.[] | select(.state=="open") | "#\(.number)"] | join(" ")' 2>/dev/null
}

# Why an issue can't be taken now (empty = ready).
skip_reason() {
  local n="$1" b
  b="$(open_blockers "$n")"; [ -z "$b" ] || { echo "blocked by $b"; return; }
  [ -z "$(gh pr list --state open --head "fix/issue-$n" --json number --jq '.[].number')" ] || { echo "open PR exists"; return; }
  git -C "$REPO" ls-remote --exit-code --heads origin "fix/issue-$n" >/dev/null 2>&1 && { echo "remote branch fix/issue-$n exists"; return; }
  true
}

setup_worktree() {
  if [ ! -e "$WT/.git" ]; then
    log "creating worktree $WT"
    git -C "$REPO" worktree add -q --detach "$WT" origin/main || exit 1
  fi
  [ -d "$WT/web/node_modules" ] || (cd "$WT/web" && npm install --no-audit --no-fund >"$RUN_DIR/npm-install.log" 2>&1) \
    || { log "npm install failed, see $RUN_DIR/npm-install.log"; exit 1; }
}

reset_worktree() {  # $1 = branch to (re)create from origin/main
  git -C "$WT" fetch -q origin main &&
    git -C "$WT" checkout -q -f -B "$1" origin/main &&
    git -C "$WT" reset -q --hard origin/main &&
    git -C "$WT" clean -fdq   # keeps ignored build caches (target, node_modules, wasm pkg)
}

# Full check gate: same as `make test` plus a release build and the wasm package.
gate() {  # $1 = log file
  (
    cd "$WT" &&
      cargo build --release &&
      cargo test --release &&
      cargo clippy --all-targets -- -D warnings &&
      cd web && npm run wasm && npm run typecheck && npm test
  ) >"$1" 2>&1
}

# Seconds to wait for a usage-limit reset, from "…limit reached|<epoch>" if present.
limit_wait() {
  local epoch now
  epoch="$(grep -oE 'limit reached\|[0-9]{9,}' "$1" | head -1 | cut -d'|' -f2)"
  now="$(date +%s)"
  if [ -n "$epoch" ] && [ "$epoch" -gt "$now" ]; then echo $((epoch - now + 60)); else echo "$LIMIT_SLEEP"; fi
}

# Run the agent until it finishes, resuming its session after usage limits.
# Sets AGENT_SESSION, AGENT_RESULT, AGENT_COST. Returns 0 on success.
run_agent() {  # $1 = issue, $2 = prompt, $3 = tag
  local n="$1" prompt="$2" tag="$3" out attempt=0 code is_err
  while [ "$attempt" -lt "$MAX_ATTEMPTS" ]; do
    attempt=$((attempt + 1))
    out="$RUN_DIR/issue-$n.$tag.$attempt.json"
    local resume=()
    [ -n "$AGENT_SESSION" ] && resume=(--resume "$AGENT_SESSION")
    # shellcheck disable=SC2086
    (cd "$WT" && timeout --kill-after=60 "$ISSUE_TIMEOUT" "$CLAUDE_BIN" -p "$prompt" "${resume[@]}" \
      --dangerously-skip-permissions --output-format json $CLAUDE_EXTRA_ARGS) >"$out" 2>"$out.stderr"
    code=$?
    local sid; sid="$(jq -r '.session_id // empty' "$out" 2>/dev/null)"
    [ -n "$sid" ] && AGENT_SESSION="$sid"
    is_err="$(jq -r 'if has("is_error") then (.is_error|tostring) else "unparsed" end' "$out" 2>/dev/null || echo unparsed)"
    AGENT_COST="$(jq -r '.total_cost_usd // 0' "$out" 2>/dev/null || echo 0)"
    if [ "$code" = 0 ] && [ "$is_err" = false ]; then
      AGENT_RESULT="$(jq -r '.result // ""' "$out")"
      return 0
    fi
    [ "$code" = 124 ] && { log "#$n agent timed out after $ISSUE_TIMEOUT"; return 1; }
    # Only look for limit messages in failed runs, so an issue about rate limits can't trip this.
    if grep -qiE 'usage limit|limit reached|rate.?limit|too many requests|quota|overloaded|hit your limit|429|529' "$out" "$out.stderr"; then
      local wait; wait="$(limit_wait "$out")"
      log "#$n usage/rate limit (attempt $attempt/$MAX_ATTEMPTS); sleeping ${wait}s"
      sleep "$wait"
      prompt="You were interrupted by a usage limit. Continue the task where you left off."
      [ -n "$AGENT_SESSION" ] || prompt="$2"
      continue
    fi
    log "#$n agent failed (exit $code, is_error=$is_err); see $out"
    return 1
  done
  log "#$n gave up after $MAX_ATTEMPTS attempts"
  return 1
}

save_patch() {  # keep failed work for inspection
  git -C "$WT" add -A >/dev/null 2>&1
  git -C "$WT" diff --cached > "$RUN_DIR/issue-$1.patch"
  git -C "$WT" reset -q
}

work_issue() {
  local n="$1" branch="fix/issue-$1" title body prompt gate_log status
  title="$(gh issue view "$n" --json title --jq .title)" || { record "$n" error "gh issue view failed" 0; return; }
  body="$(gh issue view "$n" --json body --jq .body)"
  log "==== #$n $title"
  reset_worktree "$branch" || { log "#$n worktree reset failed"; record "$n" error "worktree reset" 0; return; }

  prompt="You are resolving GitHub issue #$n: $title

$body

---
Harness rules (these override any instruction above about branches or PRs):
- You are running unattended overnight in a dedicated worktree, already on branch $branch.
  Follow CLAUDE.md. Do not ask questions; make sound decisions and record non-obvious
  ones in context/decisions.md.
- Do NOT commit, push, create branches, or open PRs. The harness commits your working
  tree, runs the gate, and opens the PR.
- Before finishing, run: cargo test --release && cargo clippy --all-targets -- -D warnings
  and, if web/ or crates/gecko-wasm changed: cd web && npm run wasm && npm run typecheck && npm test
- Do not start long-running servers (gecko serve, npm run dev). Keep exploratory
  optimize/bench runs short (--time 30s or less).
- Stay within the issue's scope. Tick the issue's item in context/roadmap.md only if it exists in the repo.
- End your final message with a short summary of what changed (it becomes the PR body),
  then a last line that is exactly 'STATUS: DONE', or 'STATUS: BLOCKED <reason>' if the
  issue cannot be completed (unclear, needs a human decision, or depends on missing work)."

  AGENT_SESSION=""; AGENT_RESULT=""; AGENT_COST=0
  local started=$SECONDS
  if ! run_agent "$n" "$prompt" main; then
    save_patch "$n"; record "$n" agent-failed "patch: issue-$n.patch" "$AGENT_COST"; return
  fi
  status="$(tail -n 5 <<<"$AGENT_RESULT" | grep -oE 'STATUS: (DONE|BLOCKED.*)' | tail -1)"
  if [[ "$status" == "STATUS: BLOCKED"* ]]; then
    log "#$n $status"; save_patch "$n"; record "$n" blocked "${status#STATUS: BLOCKED }" "$AGENT_COST"; return
  fi
  if [ -z "$(git -C "$WT" status --porcelain)" ]; then
    log "#$n no changes"; record "$n" no-op "" "$AGENT_COST"; return
  fi
  # A self-commit would bypass the gate's diff check; fold it back into the working tree.
  git -C "$WT" reset -q --soft origin/main

  local round=0
  while :; do
    gate_log="$RUN_DIR/issue-$n.gate.$round.log"
    log "#$n gate (round $round)"
    gate "$gate_log" && break
    if [ "$round" -ge "$REPAIR_ROUNDS" ]; then
      log "#$n gate failed; see $gate_log"; save_patch "$n"; record "$n" gate-failed "patch: issue-$n.patch" "$AGENT_COST"; return
    fi
    round=$((round + 1))
    local fix="The harness check gate failed. Fix the cause, re-run the checks, and end with the same summary + STATUS line. Last 80 lines of output:

$(tail -n 80 "$gate_log")"
    run_agent "$n" "$fix" "repair$round" || { save_patch "$n"; record "$n" agent-failed "repair" "$AGENT_COST"; return; }
    git -C "$WT" reset -q --soft origin/main
  done

  local summary
  summary="$(sed '/^STATUS: /d' <<<"$AGENT_RESULT")"
  git -C "$WT" add -A
  git -C "$WT" commit -q -F - <<EOF || { log "#$n commit failed"; record "$n" error "commit failed" "$AGENT_COST"; return; }
$title (#$n)

Closes #$n

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
  local mins=$(( (SECONDS - started) / 60 ))
  if ! $PUSH; then
    log "#$n committed locally on $branch (--no-push), ${mins}m"; record "$n" committed "$branch" "$AGENT_COST"; return
  fi
  git -C "$WT" push -q -u origin "$branch" || { log "#$n push failed"; record "$n" error "push failed" "$AGENT_COST"; return; }
  local url
  url="$(gh pr create --repo "$NWO" --base main --head "$branch" --title "$title" --body "Closes #$n

$summary

---
Opened by \`scripts/overnight.sh\`; passed cargo build/test/clippy and web typecheck/test.

🤖 Generated with [Claude Code](https://claude.com/claude-code)")" \
    || { log "#$n PR creation failed (branch pushed)"; record "$n" error "pr create failed" "$AGENT_COST"; return; }
  log "#$n PR $url (${mins}m)"; record "$n" pr "$url" "$AGENT_COST"
}

main() {
  mkdir -p "$RUN_DIR"
  preflight
  local q; q="$(queue)"
  if $DRY_RUN; then
    for n in $q; do
      r="$(skip_reason "$n")"
      printf '#%-4s %-6s %s%s\n' "$n" "$([ -n "$r" ] && echo skip || echo ready)" "$(gh issue view "$n" --json title --jq .title)" "${r:+  ($r)}"
    done
    rm -rf "$RUN_DIR"; return
  fi
  setup_worktree
  log "run dir $RUN_DIR; worktree $WT; push=$PUSH"
  printf 'issue\toutcome\tdetail\tcost_usd\n' > "$SUMMARY"
  local done_count=0
  for n in $q; do
    [ -e "$LOG_ROOT/STOP" ] && { log "STOP file found; stopping"; rm -f "$LOG_ROOT/STOP"; break; }
    [ "$MAX_ISSUES" -gt 0 ] && [ "$done_count" -ge "$MAX_ISSUES" ] && break
    r="$(skip_reason "$n")"
    if [ -n "$r" ]; then log "#$n skip: $r"; record "$n" skipped "$r" 0; continue; fi
    work_issue "$n"
    done_count=$((done_count + 1))
  done
  git -C "$WT" checkout -q --detach origin/main 2>/dev/null
  log "finished; summary:"; column -t -s $'\t' "$SUMMARY" | tee -a "$RUN_DIR/harness.log"
}

main
