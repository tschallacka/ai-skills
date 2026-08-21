#!/usr/bin/env bash
# MODE: DEV
# test-context-json-control-chars.sh — the reader's JSON must be parseable for
# any document content. JSON forbids every character in U+0000-U+001F inside a
# string, so one tab made the whole payload unreadable: jq stops with "control
# characters ... must be escaped" and yields nothing.
#
# Reachable through the sanctioned flow, not only by hand-editing a document:
# update-plan-content.sh -dp keeps a tab in the paragraph text verbatim. That
# writer does refuse a carriage return, because it requires content to be one
# line, so a tab is the control character a plan realistically ends up holding.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts_dir="$repo_root/planning/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

command -v jq >/dev/null 2>&1 || {
    printf 'test-context-json-control-chars: UNCONFIGURED (jq) — the point is that jq can read it\n' >&2
    exit 0
}

work="$(mktemp -d "${TMPDIR:-/tmp}/context-json-controls.XXXXXX")"
trap 'rm -rf "$work"' EXIT
plan="$work/plan"
cp -R "$repo_root/benchmark/planning/tests/fixtures/self-hosted-plan" "$plan"

# Built here rather than written literally: a control character in a source file
# is invisible to review.
tab="$(printf '\t')"
vertical_tab="$(printf '\013')"
tab_on_the_wire="$(printf 'tab[\\t]')"
vertical_on_the_wire="$(printf 'vertical[\\u000b]')"

read_json() { "$scripts_dir/plan-context.sh" read --plan-dir "$plan" "$@" --format json 2>/dev/null; }
reinit() { rm -rf "$plan/context"; "$scripts_dir/plan-context.sh" init --plan-dir "$plan" >/dev/null 2>&1; }

# ---- a clean document is passed through untouched ---------------------------
reinit
clean_content="$(read_json --document plan | jq -r '.content')"
[ -n "$clean_content" ] || t_fail 'could not read the content of a clean document'

# ---- a tab and an exotic control character, written by the helper -----------
"$scripts_dir/update-plan-content.sh" -dp "$plan" 2.1 \
    "tab[${tab}] vertical[${vertical_tab}] done." >/dev/null 2>&1 \
    || t_fail 'the sanctioned writer refused the content, so this document cannot be built'
grep -q "$tab" "$plan/plan-description.md" \
    || t_fail 'the writer did not keep the tab, so the reader is not being exercised'

reinit
payload="$(read_json --document plan)"
if printf '%s' "$payload" | jq . >/dev/null 2>&1; then :; else
    t_fail 'jq cannot parse the payload for a document containing a tab'
fi

# The named escape for a tab, and the six-character form for one that has none.
case "$payload" in
    *"$tab_on_the_wire"*) ;;
    *) t_fail 'the tab was not emitted as its named JSON escape' ;;
esac
case "$payload" in
    *"$vertical_on_the_wire"*) ;;
    *) t_fail 'the vertical tab was not emitted in the escaped unicode form' ;;
esac

# Escaping belongs on the wire only: what jq hands back must be the document's
# own bytes, or the reader is lossy instead of merely well-formed.
decoded="$(printf '%s' "$payload" | jq -r '.content')"
case "$decoded" in
    *"tab[${tab}] vertical[${vertical_tab}] done."*) ;;
    *) t_fail 'the decoded content no longer holds the original control bytes' ;;
esac

# ---- every view stays parseable --------------------------------------------
for view_spec in '--document inventory' '--document coverage' '--document progress' '--unit W01'; do
    # shellcheck disable=SC2086
    if read_json $view_spec | jq . >/dev/null 2>&1; then :; else
        t_fail "jq cannot parse the payload for: $view_spec"
    fi
done

t_end
