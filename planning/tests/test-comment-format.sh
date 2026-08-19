#!/usr/bin/env bash
# test-comment-format — quoted material in comments is fenced, and fences are
# not used to smuggle prose past the three-line limit.
#
# Usage: test-comment-format.sh
#
# CODE-STYLE.md §11 caps in-body comment prose at three lines and exempts a
# fenced quoted block, because verbatim material (an output contract, a data
# shape, an example invocation) does not fit in three lines and must not be
# deleted for it. The exemption only works if the fence is well-formed, so this
# checks the fence rather than trusting it.
set -euo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fail=0
note_fail() { printf 'comment-format: %s\n' "$1" >&2; fail=1; }

script_list() {
    ( cd "$repo_root" && find . -name '*.sh' -type f \
        -not -path './benchmark/results/*' -not -path './.git/*' -not -path './.plans/*' \
        -not -name 'test-comment-format.sh' | LC_ALL=C sort )
}

# Every fence must open with a named subject, close exactly once, and contain
# only quoted lines. An unbalanced fence would silently exempt the rest of a file.
while IFS= read -r file; do
    awk -v f="$file" '
        function flag(msg) { printf "%s:%d %s\n", f, FNR, msg }
        /^[[:space:]]*#[[:space:]]*----[[:space:]]*quoted:/ {
            if (open) flag("a quoted fence opened while one was already open")
            open = 1; opened_at = FNR
            subject = $0
            sub(/^.*quoted:[[:space:]]*/, "", subject)
            sub(/[[:space:]]*----.*$/, "", subject)
            if (subject == "") flag("the opening fence does not name what is quoted")
            next
        }
        /^[[:space:]]*#[[:space:]]*----[[:space:]]*end quoted[[:space:]]*----/ {
            if (!open) flag("a quoted fence closed without opening")
            open = 0
            next
        }
        open && $0 !~ /^[[:space:]]*#/ {
            flag("a quoted fence was left open at a non-comment line")
            open = 0
        }
        END { if (open) printf "%s:%d a quoted fence was never closed\n", f, opened_at }
    ' "$repo_root/$file"
done < <(script_list) > "$repo_root/.comment-format.tmp" 2>/dev/null || true

while IFS= read -r line; do
    [ -n "$line" ] || continue
    note_fail "$line"
done < "$repo_root/.comment-format.tmp"
rm -f "$repo_root/.comment-format.tmp"

# The fence exempts quoted lines, not prose. A fenced block whose lines look like
# sentences is the loophole: verbatim material rarely ends in a full stop.
while IFS= read -r file; do
    awk -v f="$file" '
        /^[[:space:]]*#[[:space:]]*----[[:space:]]*quoted:/ { open = 1; prose = 0; start = FNR; next }
        /^[[:space:]]*#[[:space:]]*----[[:space:]]*end quoted[[:space:]]*----/ {
            if (open && prose >= 3)
                printf "%s:%d a quoted fence holds %d prose-looking lines; move the explanation outside it\n", f, start, prose
            open = 0; next
        }
        open {
            line = $0
            sub(/^[[:space:]]*#[[:space:]]?/, "", line)
            # A sentence: starts with a capital or lowercase word and ends in a
            # full stop. Verbatim output, shapes and commands almost never do.
            if (line ~ /^[A-Za-z][A-Za-z0-9 ,;()'"'"'`-]*\.$/) prose++
        }
    ' "$repo_root/$file"
done < <(script_list) > "$repo_root/.comment-prose.tmp" 2>/dev/null || true

while IFS= read -r line; do
    [ -n "$line" ] || continue
    note_fail "$line"
done < "$repo_root/.comment-prose.tmp"
rm -f "$repo_root/.comment-prose.tmp"

# ── --help is interface, not a comment dump (CODE-STYLE.md §4) ───────────────
# A script serving help from its own docblock must strip the `# ` and skip the
# shebang; printing raw comment syntax at a user is a defect.
while IFS= read -r file; do
    case "$file" in
        *-lib.sh|*/tests/*|./run-tests.sh) continue ;;
    esac
    grep -q -- '--help' "$repo_root/$file" || continue
    help_out="$( cd "$repo_root" && bash "$file" --help 2>/dev/null )" || continue
    [ -n "$help_out" ] || continue
    case "$help_out" in
        '#!'*) note_fail "$file --help prints its shebang" ;;
    esac
    # PORTABILITY(pipefail-grep-q): the anchor is per line, so grep stays — with
    # -c, which drains the pipe instead of closing it on the first match.
    if printf '%s\n' "$help_out" | grep -c '^[[:space:]]*# ' >/dev/null; then
        note_fail "$file --help prints raw '# ' comment lines; strip them"
    fi
done < <(script_list)

# A fixed line window truncates help the moment the docblock grows. A docblock-
# served help must print the whole leading comment block and stop at the first
# non-comment line.
while IFS= read -r file; do
    grep -qE 'sed -n .[0-9]+,[0-9]+p. "\$0"' "$repo_root/$file" || continue
    note_fail "$file serves help from a fixed line window; print the whole leading comment block instead"
done < <(script_list)

[ "$fail" -eq 0 ] || exit 1
printf '%s\n' 'test-comment-format: PASS'
