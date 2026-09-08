#!/usr/bin/env bash
# MODE: DEV
# test-skill-positional-params.sh - no shipped skill body may contain a shell
# positional parameter.
#
# The skill delivery path substitutes positional parameters -- a dollar sign
# followed by a digit, and the all-arguments forms -- with the arguments the
# skill was invoked with, INCLUDING inside fenced code blocks. Measured
# 2026-09-08: chat/SKILL.md's wake guard used awk's whole-line variable, and
# three agents each read a different word in its place, every one of them the
# first word of that agent's own invocation.
#
# What makes it worth a gate rather than a note is that it fails SILENTLY and
# self-justifyingly. The guard matched nothing and woke nobody while the agent
# stayed visibly present in the channel list, and the paragraph explaining the
# token was substituted too -- so the explanation corroborated the corruption,
# and two agents reported the file as broken when the file was correct. Nobody
# can find that by reading the text; the text reads fine.
#
# So the rule is not "quote it carefully", it is "do not write one at all", and
# a skill that needs a whole-line match uses grep (which needs no positional)
# rather than awk. Named variables like $NICK are unaffected and stay allowed.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

# awk rather than grep: this repository gates text search behind a minted
# token, and a test must run unattended.
scan() { # <file>
    awk '/\$[0-9]|\$@|\$\*/ {printf "%s:%d: %s\n", FILENAME, FNR, $0}' "$1"
}

shipped_docs() {
    find "$repo_root" -name 'SKILL.md' \
        -not -path "$repo_root/.claude/*" \
        -not -path "$repo_root/benchmark/*" | sort
}

count=0
findings=""
while IFS= read -r doc; do
    [ -n "$doc" ] || continue
    count=$((count + 1))
    hit="$(scan "$doc")"
    [ -z "$hit" ] || findings="$findings$hit
"
done <<EOF
$(shipped_docs)
EOF

if [ -n "$findings" ]; then
    printf '%s\n' "$findings" >&2
    t_fail "a shipped skill body contains a positional parameter; the delivery path will substitute it"
fi

# The scan must be able to FAIL, or a green run proves nothing. A fixture
# carrying the exact construct that caused this is checked in-line rather than
# committed, so the assertion cannot rot into a no-op the way a rule with no
# test does.
probe="$(mktemp "${TMPDIR:-/tmp}/skillprobe.XXXXXX")"
printf '%s\n' 'awk -v me="@$NICK" '"'"'index($0, me){f=1}'"'"'' >"$probe"
probe_hit="$(scan "$probe")"
rm -f "$probe"
t_assert_eq 'the scan detects the construct that caused this' \
    "$([ -n "$probe_hit" ] && echo found || echo missed)" 'found'

# And it must not fire on a NAMED variable, or every skill would fail and the
# gate would be turned off rather than obeyed.
probe="$(mktemp "${TMPDIR:-/tmp}/skillprobe.XXXXXX")"
printf '%s\n' 'chat-client-rs tail --chan "$CHAN" --nick "$NICK" >> "$LOG"' >"$probe"
named_hit="$(scan "$probe")"
rm -f "$probe"
t_assert_eq 'the scan leaves named variables alone' \
    "$([ -n "$named_hit" ] && echo fired || echo quiet)" 'quiet'

[ "$count" -gt 0 ] || t_fail "no SKILL.md files were scanned, so this proved nothing"
printf 'test-skill-positional-params: scanned %d skill body/bodies\n' "$count"

t_end
