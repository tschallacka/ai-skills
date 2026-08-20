#!/usr/bin/env bash
# test-context-summary-excerpt.sh — the summary view must say it is an excerpt.
#
# `summary` is `sed -n '1,12p'`: a fixed slice of the head of the file, applied
# BEFORE paging. So the page reports no withheld records and no next_token,
# while SKILL.md tells readers that no next_token means the document is fully
# read. A reviewer following that rule on the default view read the first two
# sections of a plan and believed it had read the plan — and reviewers are
# steered to this view by default, so a gated review could pass on a stub.
#
# Reported by a reviewer subagent. The excerpt line is the signal that makes the
# documented paging rule true again.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts_dir="$repo_root/planning/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/context-summary-excerpt.XXXXXX")"
trap 'rm -rf "$work"' EXIT
plan="$work/plan"
cp -R "$repo_root/benchmark/planning/tests/fixtures/self-hosted-plan" "$plan"
"$scripts_dir/plan-context.sh" init --plan-dir "$plan" >/dev/null 2>&1

document="$plan/plan-description.md"
total="$(wc -l < "$document" | tr -d ' ')"
[ "$total" -gt 12 ] || t_fail "the fixture plan is too short to be truncated ($total lines)"

# ---- the default view announces the elision ---------------------------------
default_out="$("$scripts_dir/plan-context.sh" read --plan-dir "$plan" --document plan 2>&1)"
case "$default_out" in
    *'excerpt=summary'*) ;;
    *) t_fail 'the default view elided the document without saying so' ;;
esac
case "$default_out" in
    *"of $total line(s)"*) ;;
    *) t_fail "the excerpt notice did not name the document's real length ($total)" ;;
esac
case "$default_out" in
    *'--view full'*) ;;
    *) t_fail 'the excerpt notice did not name the view that reads the whole document' ;;
esac

# The notice has to be true: the excerpt really is shorter than the document.
shown="$(printf '%s\n' "$default_out" | grep -c . || true)"
[ "$shown" -lt "$total" ] || t_fail 'the excerpt was not actually shorter than the document'

# ---- the full view does not carry the notice --------------------------------
# It pages instead, which is the mechanism the reader is told to trust.
full_out="$("$scripts_dir/plan-context.sh" read --plan-dir "$plan" --document plan --view full 2>&1)"
case "$full_out" in
    *'excerpt=summary'*) t_fail 'the full view reported itself as a summary excerpt' ;;
esac

# ---- a document shorter than the excerpt gets no notice ---------------------
# Otherwise the notice cries wolf on every short document and gets ignored.
short_plan="$work/short"
cp -R "$repo_root/benchmark/planning/tests/fixtures/self-hosted-plan" "$short_plan"
printf '# Plan: short\n\n## Current state\n\n§ 2.1\nOne line.\n' > "$short_plan/plan-description.md"
"$scripts_dir/plan-context.sh" init --plan-dir "$short_plan" >/dev/null 2>&1
short_out="$("$scripts_dir/plan-context.sh" read --plan-dir "$short_plan" --document plan 2>&1)"
case "$short_out" in
    *'excerpt=summary'*) t_fail 'a document shorter than the excerpt was reported as truncated' ;;
esac

t_end
