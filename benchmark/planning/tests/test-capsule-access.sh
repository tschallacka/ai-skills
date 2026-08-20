#!/usr/bin/env bash
# Capsule-access contract tests for the benchmark runtime.
#
# The capsule-isolation guarantee lives in the driver argv builders, so these
# tests assert the *property* every driver must satisfy rather than one driver's
# flag spelling: the capsule the role was handed is reachable from the emitted
# argv, the workspace is the agent's cwd (AGENT_CWD), and SRC_ROOT is never
# exposed. They run against the active BENCHMARK_AGENT — the previous version
# asserted codex's `--add-dir`/`-C` spelling and therefore hard-failed for
# claude and opencode, i.e. for the standard `BENCHMARK_AGENT=<agent>`
# smoke-test recipe in AGENTS.md.
#
# Codex-specific spellings are still asserted, but only when codex is active.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../planning/tests" && pwd)/lib-test.sh"
# Report the failing expression: these assertions are bare `[ ... ]` under
# set -e, which otherwise exits 1 in silence.
t_trap_assertions


root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
setup="$root/setup-benchmark.sh"
runtime="$root/runtime"
codex_driver="$runtime/codex/agent.sh"

grep -Fq 'CAPSULE_BASE="${PLANNING_AGENT_TMPDIR:-${TMPDIR:-/tmp}/planning-agent}/ai-skills-capsules"' "$setup"
grep -Fq 'CAPSULE_ROOT="$CAPSULE_BASE/$RUN_ID/$REVISION/worker"' "$setup"
grep -Fq 'worker-manifest.json' "$setup"

agent="$(BENCHMARK_AGENT="${BENCHMARK_AGENT:-}" "$BASH" -c '
    set -euo pipefail
    source "$1/agent-env.sh"
    resolve_active_agent "$1"
    printf "%s\n" "$AGENT_DRIVER"
' _ "$runtime")"
echo "capsule access: active driver $agent"

tmp="$(mktemp -d "${TMPDIR:-/tmp}/capsule-access.XXXXXX")"
trap 'rm -rf -- "$tmp"' EXIT
printf 'prompt' > "$tmp/prompt.md"

# Two realistic capsule shapes. The worker capsule nests planning/; the reviewer
# capsule is flat and carries the reviewed plan under plan/ — the layout the
# opencode driver's old hardcoded relative-path list could not grant.
mkdir -p "$tmp/ws" "$tmp/cap/planning/scripts" "$tmp/rev-ws" "$tmp/rev-cap/plan"
printf 'spec\n' > "$tmp/cap/task-spec.md"
printf 'skill\n' > "$tmp/cap/planning/SKILL.md"
printf 'validator\n' > "$tmp/cap/planning/scripts/validate-plan.sh"
printf 'spec\n' > "$tmp/rev-cap/task-spec.md"
printf 'plan\n' > "$tmp/rev-cap/plan/plan.md"

# emit_argv <role> <workspace> <capsule>: prints AGENT_CWD then one argv element
# per line.
emit_argv() {
    "$BASH" -c '
        set -euo pipefail
        source "$1/lib-agent.sh"
        "agent_argv_$2" "$3" "$4" "$5"
        printf "%s\n" "${AGENT_CWD:-}"
        printf "%s\n" "${AGENT_ARGV[@]}"
    ' _ "$runtime" "$1" "$2" "$3" "$tmp/prompt.md"
}

assert_capsule_granted() {
    local role="$1" workspace="$2" capsule="$3" out cwd
    out="$(emit_argv "$role" "$workspace" "$capsule")"
    cwd="$(printf '%s\n' "$out" | sed -n '1p')"
    [ "$cwd" = "$workspace" ] || { echo "$role: AGENT_CWD is '$cwd', want $workspace" >&2; exit 1; }
    case "$out" in
        *"$capsule"*) ;;
        *) echo "$role argv does not reference the capsule $capsule" >&2; exit 1 ;;
    esac
    case "$out" in
        *"$workspace"*) ;;
        *) echo "$role argv does not reference the workspace $workspace" >&2; exit 1 ;;
    esac
    case "$out" in
        *SRC_ROOT*) echo "$role argv references SRC_ROOT" >&2; exit 1 ;;
    esac
    printf '%s\n' "$out"
}

worker_argv="$(assert_capsule_granted worker "$tmp/ws" "$tmp/cap")"
reviewer_argv="$(assert_capsule_granted reviewer "$tmp/rev-ws" "$tmp/rev-cap")"
analyzer_argv="$(assert_capsule_granted analyzer "$tmp/ws" "$tmp/cap")"

# The reviewer must be able to reach the plan it reviews. Either the capsule
# root is granted as a directory (codex/claude --add-dir) or the plan's files
# are granted individually (opencode -f attachments).
grants_capsule_root=false
while IFS= read -r argv_word; do
    if [ "$argv_word" = "$tmp/rev-cap" ]; then grants_capsule_root=true; break; fi
done <<ARGV
$reviewer_argv
ARGV
grants_plan_file=false
case "$reviewer_argv" in
    *"$tmp/rev-cap/plan/plan.md"*) grants_plan_file=true ;;
esac
if [ "$grants_capsule_root" = false ] && [ "$grants_plan_file" = false ]; then
    echo 'reviewer argv grants neither the capsule root nor the reviewed plan' >&2
    exit 1
fi

# Codex flag spellings, asserted only for codex.
if [ "$agent" = codex ]; then
    [ "$(printf '%s\n' "$worker_argv" | { grep -c -- '--add-dir' || true; })" -ge 2 ] ||
        { echo 'codex worker argv does not grant capsule+workspace via --add-dir' >&2; exit 1; }
    grep -Fq -- '--add-dir' "$codex_driver"
    if ! printf '%s\n' "$worker_argv" | awk -v want="$tmp/ws" '$0=="-C"{getline; if ($0==want) found=1} END{exit !found}'; then
        echo 'codex worker argv -C does not target the workspace' >&2
        exit 1
    fi
fi

# SRC_ROOT non-exposure in the sources as well as the emitted argv.
if grep -Fq -- '--add-dir "$SRC_ROOT"' "$setup"; then
    echo 'worker launch still exposes SRC_ROOT' >&2
    exit 1
fi
for driver in "$runtime"/*/agent.sh; do
    if grep -Fq 'SRC_ROOT' "$driver"; then
        echo "driver references SRC_ROOT: $driver" >&2
        exit 1
    fi
done

# Prompt-side anchors preserved.
grep -Fq 'Do not inspect `SRC_ROOT`' "$root/worker-prompt.md"
grep -Fq 'only its run instructions, harness summary, current run' "$root/analyzer-prompt.md"
printf 'Capsule access contract tests passed.\n'
