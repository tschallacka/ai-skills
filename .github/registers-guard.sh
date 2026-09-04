#!/usr/bin/env bash
# MODE: DEV
# registers-guard.sh — decide whether a `registers` push may reach master.
#
# The register branch exists because BUGS.json and TODO.json are append-mostly
# arrays: two branches that each file an entry both take the same next free id,
# and git cannot see that collision. The additions land at different array
# positions, so it merges them textually with NO conflict and the result carries
# two unrelated entries under one id. Eight of those landed in one merge on
# 2026-09-04, invisible until reg_findings ran. A single writer removes the
# class, and this script is what makes that branch safe to fast-forward.
#
# Two refusals, and the second is the load-bearing one:
#
#   1. Every changed path must be BUGS.json or TODO.json. Register changes reach
#      master without review by design, so a branch that could carry anything
#      else is a protection bypass, not a convenience.
#   2. Neither register may carry a duplicate id, and every parent must resolve.
#      This is the check git cannot perform, so it is the reason CI runs at all.
#
# Deliberately NOT checked here: whether an entry is well formed. `bugs check`
# and `todo check` own that, they ship in the repo, and duplicating their rules
# in a second implementation is how the two drift apart. The workflow runs them
# alongside this script when a built tree is available; this script is the part
# that needs no binaries, so it can refuse a bad push on a bare runner.
#
# Usage:
#   registers-guard.sh --base <ref>        compare HEAD against <ref>
#   registers-guard.sh --files-from <file> read the change set from a file
#   registers-guard.sh --help
#
# Exit codes: 0 may merge; 1 refused, with the reason on stderr; 64 bad usage.

set -uo pipefail
export LC_ALL=C

usage() {
    sed -n '3,30p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

registers="BUGS.json TODO.json"
base=""
files_from=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        --base) base="${2:-}"; shift 2 || exit 64 ;;
        --files-from) files_from="${2:-}"; shift 2 || exit 64 ;;
        --help|-h) usage; exit 0 ;;
        *) printf '%s: unknown argument: %s\n' "${0##*/}" "$1" >&2; exit 64 ;;
    esac
done

refuse() { printf 'registers-guard: REFUSED: %s\n' "$1" >&2; }

changed_paths() {
    if [ -n "$files_from" ]; then
        [ -r "$files_from" ] || { refuse "cannot read the change set from $files_from"; exit 1; }
        cat "$files_from"
        return
    fi
    [ -n "$base" ] || { printf '%s: --base or --files-from is required\n' "${0##*/}" >&2; exit 64; }
    git rev-parse --verify "$base" >/dev/null 2>&1 \
        || { refuse "the base ref $base does not resolve; refusing rather than guessing"; exit 1; }
    git diff --name-only "$base..HEAD"
}

# ---- 1. nothing but the registers ------------------------------------------
# An empty change set is refused too: a push that changes nothing has no
# business fast-forwarding master, and it is far more likely to mean the base
# was resolved wrongly than that someone pushed an empty commit on purpose.
stray=""
count=0
while IFS= read -r path; do
    [ -n "$path" ] || continue
    count=$((count + 1))
    case " $registers " in
        *" $path "*) : ;;
        *) stray="$stray$path
" ;;
    esac
done <<EOF
$(changed_paths)
EOF

if [ "$count" -eq 0 ]; then
    refuse "the change set is empty; a registers push must change a register"
    exit 1
fi
if [ -n "$stray" ]; then
    refuse "the registers branch may only change BUGS.json and TODO.json"
    printf '%s' "$stray" | sed 's/^/  /' >&2
    printf 'registers-guard: register changes reach master without review, so a branch\n' >&2
    printf 'registers-guard: that can carry anything else is a protection bypass.\n' >&2
    exit 1
fi

# ---- 2. the collision git cannot see ---------------------------------------
# python3 rather than the register binaries: this must be able to refuse on a
# runner with nothing built. CODE-STYLE.md §1 allows python3 for development
# tooling, and this never ships to a skill.
command -v python3 >/dev/null 2>&1 \
    || { refuse "python3 is required to check the registers for duplicate ids"; exit 1; }

for register in $registers; do
    [ -f "$register" ] || continue
    python3 - "$register" <<'PY' || exit 1
import json, sys

path = sys.argv[1]
key = "bugs" if path.startswith("BUGS") else "tasks"

def refuse(msg):
    sys.stderr.write("registers-guard: REFUSED: %s: %s\n" % (path, msg))
    sys.exit(1)

try:
    with open(path) as fh:
        doc = json.load(fh)
except (OSError, ValueError) as exc:
    refuse("does not parse as JSON (%s)" % exc)

entries = doc.get(key)
if not isinstance(entries, list):
    refuse("has no %s array" % key)

ids = [e.get("id") for e in entries]
missing = [n for n, i in enumerate(ids) if not isinstance(i, str) or not i]
if missing:
    refuse("entries at positions %s have no id" % missing[:5])

seen, dupes = set(), []
for i in ids:
    if i in seen and i not in dupes:
        dupes.append(i)
    seen.add(i)
if dupes:
    refuse(
        "duplicate ids: %s -- two branches took the same next free id, and a "
        "textual merge cannot see it" % " ".join(dupes)
    )

dangling = sorted({
    "%s->%s" % (e["id"], e["parent"])
    for e in entries
    if e.get("parent") and e["parent"] not in seen
})
if dangling:
    refuse("parents that do not resolve: %s" % " ".join(dangling))

print("registers-guard: %s: %d entries, no duplicate ids, every parent resolves" % (path, len(ids)))
PY
done

printf 'registers-guard: may merge\n'
