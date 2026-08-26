#!/usr/bin/env bash
# MODE: DEV
# test-writer-hardening — W14/W16/W18 pins for update-plan-content.sh:
# document ids and paragraph ids reject command substitution, traversal and
# empty values before any file is opened; the review-approval pair write
# leaves no torn pair across kill loops.
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
scripts="$root/planning/scripts"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# ---- fixture plan: minimal review + description with Status fields --------
mkdir -p "$tmp/p"
printf '# Review\n\n- Status: `💤 pending`\n\n## Findings\n\n| ID | Sev | Finding | Resolution |\n|---|---|---|---|\n\n| AR-1 | minor | probe | resolved |\n' > "$tmp/p/adversarial-review.md"
printf '# Plan\n\n- Status: 💤 pending\n\n## Objective\n\n§ 1.1\nProbe objective line.\n' > "$tmp/p/plan-description.md"

# ---- W16 pins: hostile ids are refused before any write ------------------
for hostile in '$(touch pwned)' '../../etc/passwd' '' 'goal:../../../x'; do
    out="$("$scripts/update-plan-content.sh" -t "$tmp/p" "$hostile" Title X 2>&1 || true)"
    case "$out" in
        *unknown*|*Document*|*usage*|*id*|*'not found'*|*must*) : ;;
        *) t_fail "hostile id accepted silently: [$hostile] -> $out" ;;
    esac
    [ ! -f "$tmp/pwned" ] || t_fail "command substitution executed for: [$hostile]"
done
for hostile_para in '1.$(x)' 'x.y' '9.9; rm p' ; do
    out="$("$scripts/update-plan-content.sh" --delete-paragraph "$tmp/p" plan "$hostile_para" 2>&1 || true)"
    case "$out" in
        *'N.N'*|*usage*|*Paragraph*) : ;;
        *) t_fail "hostile paragraph id accepted: [$hostile_para] -> $out" ;;
    esac
done
t_assert_eq "description untouched by hostile ids" \
  "$(grep -c 'Status: 💤 pending' "$tmp/p/plan-description.md")" "1"

# ---- W14/W18 pin: approval never leaves a torn pair under kills ----------
git -C "$tmp" init -q && git -C "$tmp" config user.email t@t && git -C "$tmp" config user.name t
git -C "$tmp" add -A && git -C "$tmp" commit -qm base
rounds=0; torn=0
while [ "$rounds" -lt 15 ]; do
    rounds=$((rounds + 1))
    # reset to approved-able pending state
    sed -i 's/Status: `✅ approved`/Status: `💤 pending`/' "$tmp/p/adversarial-review.md" 2>/dev/null || true
    sed -i 's/Status: ✅ approved/Status: 💤 pending/' "$tmp/p/plan-description.md" 2>/dev/null || true
    "$scripts/update-plan-content.sh" -rv "$tmp/p" pending >/dev/null 2>&1 || true
    "$scripts/update-plan-content.sh" -rv "$tmp/p" approved >/dev/null 2>&1 &
    killer=$!
    # kill at a random point in the write window
    sleep "0.00$((RANDOM % 9 + 1))"
    kill -9 "$killer" 2>/dev/null || true
    wait "$killer" 2>/dev/null || true
    r="$(sed -n 's/^- Status: //p' "$tmp/p/adversarial-review.md")"
    d="$(sed -n 's/^- Status: //p' "$tmp/p/plan-description.md")"
    case "$r$d" in
        *pending*pending*|*approved*approved*) : ;;
        *) torn=$((torn + 1)) ;;
    esac
done
[ "$torn" -eq 0 ] || t_fail "torn pair observed $torn/15 rounds (review=$r description=$d)"

t_end
