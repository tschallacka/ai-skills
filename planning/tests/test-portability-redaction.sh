#!/usr/bin/env bash
# MODE: DEV
# test-portability-redaction.sh — the construct scan reads instructions, not prose.
#
# src/tony-the-pony decides what counts as an instruction, for this scan and for
# the grep gate both. Its own unit tests pin the classification; this pins the
# thing the scan depends on, which is that a rule from the registry applied to
# redacted text reports the instruction and not the prose about it.
#
# Both directions of getting it wrong have happened: a warning message naming an
# in-place rewrite was reported as a use of one, and a line-oriented comment
# strip cut a line at a `#` inside a string, hiding a real violation after it.
#
# The rule regex comes from portability-rules.json rather than being restated
# here, so this tests the classification and not a copy of the pattern.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
redactor="$repo_root/target/release/tony-the-pony"
rules="$repo_root/portability-rules.json"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

command -v rjq >/dev/null 2>&1 || {
    printf 'portability-redaction: UNCONFIGURED (rjq)\n' >&2
    exit 64
}
if [ ! -x "$redactor" ] && command -v cargo >/dev/null 2>&1; then
    (cd "$repo_root" && cargo build --release -p tony-the-pony) >/dev/null 2>&1 || true
fi
[ -x "$redactor" ] || {
    t_record "missing $redactor (cargo build --release -p tony-the-pony)"
    t_end
}

work="$(mktemp -d "${TMPDIR:-/tmp}/portability-redaction.XXXXXX")"
trap 'rm -rf "$work"' EXIT

detect="$(rjq -r '.rules[] | select(.id == "sed-inplace") | .detect' "$rules")"
[ -n "$detect" ] || { t_record 'the sed-inplace rule has no detect pattern'; t_end; }

# The line numbers a scan would report for one fixture, space separated.
hits() { # <file>
    "$redactor" --redact "$1" \
        | awk -v pattern="$detect" '$0 ~ pattern { printf "%s ", FNR }'
}

fixture="$work/cases.sh"
# The construct is assembled from a variable so this fixture writer does not
# itself carry a literal the scan would report against this file.
verb='sed -i'
{
    printf '#!/usr/bin/env bash\n'
    printf '# a comment naming %s must not count\n' "$verb"
    printf "echo 'single quoted %s must not count'\\n" "$verb"
    printf 'echo "double quoted %s must not count"\n' "$verb"
    printf "cat <<'BODY'\\n"
    printf '  a quoted heredoc naming %s must not count\n' "$verb"
    printf 'BODY\n'
    printf 'cat <<-TABBED\n'
    printf '\tan indented heredoc naming %s must not count\n' "$verb"
    printf '\tTABBED\n'
    printf '%s %s file\n' "$verb" "'s/a/b/'"
} >"$fixture"

t_assert_eq 'only the real invocation is reported' "$(hits "$fixture")" '11 '

# A `#` inside a string used to truncate the line, so a construct written after
# it disappeared from the scan entirely.
fixture="$work/hash-in-string.sh"
{
    printf 'grep %s file && %s %s later\n' "'#hash'" "$verb" "'s/x/y/'"
} >"$fixture"
t_assert_eq 'a hash inside a string does not hide what follows it' \
    "$(hits "$fixture")" '1 '

# A left shift is not a heredoc. Read as one, every following line was treated
# as body until a line matched the shift count.
fixture="$work/shift.sh"
{
    printf 'width=$(( 1 << 2 ))\n'
    printf '%s %s after-the-shift\n' "$verb" "'s/a/b/'"
} >"$fixture"
t_assert_eq 'an arithmetic shift does not swallow the following lines' \
    "$(hits "$fixture")" '2 '

# A parameter expansion is not a comment, so code after one still scans.
fixture="$work/expansion.sh"
{
    printf 'trimmed=${name#leading}\n'
    printf '%s %s after-the-expansion\n' "$verb" "'s/a/b/'"
} >"$fixture"
t_assert_eq 'a parameter expansion does not open a comment' \
    "$(hits "$fixture")" '2 '

# Position and length survive redaction, or a reported line number would name
# the wrong line in the source.
fixture="$work/offsets.sh"
{
    printf '# padding\n'
    printf "echo 'padding'\\n"
    printf '%s %s third-line\n' "$verb" "'s/a/b/'"
} >"$fixture"
t_assert_eq 'the reported line is the line in the source' \
    "$(hits "$fixture")" '3 '
t_assert_eq 'redaction preserves the line count' \
    "$("$redactor" --redact "$fixture" | awk 'END {print NR}')" '3'
t_assert_eq 'redaction preserves each line length' \
    "$("$redactor" --redact "$fixture" | awk '{printf "%s ", length($0)}')" \
    "$(awk '{printf "%s ", length($0)}' "$fixture")"

t_end
