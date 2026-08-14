#!/usr/bin/env bash
# Capsule-access contract tests for the benchmark runtime.
#
# The capsule-isolation guarantee moved from inline --add-dir strings in
# setup-benchmark.sh into the codex driver argv builder. These tests assert the
# driver emits the capsule/workspace --add-dir flags (and -C on the workspace),
# keep the SRC_ROOT non-exposure negative on the driver and setup, and preserve
# the prompt-side anchors.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
setup="$root/setup-benchmark.sh"
runtime="$root/runtime"
codex_driver="$runtime/codex/agent.sh"

grep -Fq 'CAPSULE_BASE="${PLANNING_AGENT_TMPDIR:-${TMPDIR:-/tmp}/planning-agent}/ai-skills-capsules"' "$setup"
grep -Fq 'CAPSULE_ROOT="$CAPSULE_BASE/$RUN_ID/$REVISION/worker"' "$setup"
grep -Fq 'worker-manifest.json' "$setup"

# The codex driver owns the worker capsule/workspace argv. Emit a worker argv
# against a temp capsule/workspace and assert both roots are granted via
# --add-dir and that -C points at the workspace. SRC_ROOT must never appear.
tmp="$(mktemp -d "${TMPDIR:-/tmp}/capsule-access.XXXXXX")"
trap 'rm -rf -- "$tmp"' EXIT
mkdir -p "$tmp/ws" "$tmp/cap"
printf 'prompt' > "$tmp/prompt.md"
out="$(bash -c '
    set -euo pipefail
    source "$1/lib-agent.sh"
    agent_argv_worker "$2/ws" "$2/cap" "$2/prompt.md"
    printf "%s\\n" "${AGENT_ARGV[@]}"
' _ "$runtime" "$tmp")"

[ "$(printf '%s\n' "$out" | grep -c -- '--add-dir')" -ge 2 ] || { echo 'worker argv does not grant capsule+workspace via --add-dir' >&2; exit 1; }
grep -Fq -- '--add-dir' "$codex_driver"
# -C must point at the workspace (element immediately following "-C").
if ! printf '%s\n' "$out" | awk -v want="$tmp/ws" '$0=="-C"{getline; if ($0==want) found=1} END{exit !found}'; then
    echo 'worker argv -C does not target the workspace' >&2
    exit 1
fi
if printf '%s\n' "$out" | grep -Fq "$tmp/cap"; then :; else echo 'worker argv does not include the capsule path' >&2; exit 1; fi
if printf '%s\n' "$out" | grep -Fq "$tmp/ws"; then :; else echo 'worker argv does not include the workspace path' >&2; exit 1; fi

# SRC_ROOT non-exposure: neither the driver source nor the emitted argv may
# reference SRC_ROOT, and the setup launch path must not add-dir it.
if grep -Fq -- '--add-dir "$SRC_ROOT"' "$setup"; then
    echo 'worker launch still exposes SRC_ROOT' >&2
    exit 1
fi
if grep -Fq 'SRC_ROOT' "$codex_driver"; then
    echo 'codex driver references SRC_ROOT' >&2
    exit 1
fi
if printf '%s\n' "$out" | grep -Fq 'SRC_ROOT'; then
    echo 'emitted worker argv references SRC_ROOT' >&2
    exit 1
fi

# Prompt-side anchors preserved.
grep -Fq 'Do not inspect `SRC_ROOT`' "$root/worker-prompt.md"
grep -Fq 'only its run instructions, harness summary, current run' "$root/analyzer-prompt.md"
printf 'Capsule access contract tests passed.\n'