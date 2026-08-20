#!/usr/bin/env bash
# test-mermaid-accuracy — the ARCHITECTURE mermaid diagrams may not name a
# script, artifact, node id or function that does not exist.
#
# Usage: test-mermaid-accuracy.sh
#
# Four mechanical checks, and each one's limit:
#   1. Structure, in awk: balanced quotes and brackets, a recognised header, at
#      least one statement, every referenced node id defined. Only the shape awk
#      can see. Mermaid's own verdict comes from `mmdc`, a dev-flake tool and not
#      a suite dependency, so that portion reports UNCONFIGURED when it is
#      absent. Rendering proves syntax, never accuracy.
#   2. Every `*.sh` named in a block exists, or a redirect in some script writes
#      it (`benchmark-env.sh` is generated).
#   3. Every artifact filename named in a block is a real path or is named by
#      some script. "Written by" is not decidable here — plan paths are composed
#      from variables — so a name only prose knows is a WARN, not a FAIL.
#   4. Every backticked lower_snake identifier resolves: a `name()` definition
#      where the document writes it as a function, else a mention in a script.
#
# Not checkable, hence CODE-STYLE.md §11's rule for the author: whether an arrow
# is the real control flow, a diamond the true condition, a stage order the code.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
self_path="$repo_root/planning/tests/test-mermaid-accuracy.sh"
# Every tracked document holding a mermaid block, so a new diagram cannot be
# added outside the checked set.
docs="planning/ARCHITECTURE.md benchmark/planning/ARCHITECTURE.md
brainstorm/SKILL.md post-implementation-review/SKILL.md"

work="$(mktemp -d "${TMPDIR:-/tmp}/mermaid-accuracy.XXXXXX")"
trap 'rm -rf -- "$work"' EXIT

group=0
note_fail() { printf 'mermaid-accuracy: FAIL: %s\n' "$1" >&2; t_record "$1"; }
note_warn() { printf 'mermaid-accuracy: WARN: %s\n' "$1" >&2; }
# A group reports PASS only when it added no finding of its own, so one broken
# check cannot hide behind another group's PASS line.
group_start() { group="$(t_failures)"; }
group_done() {
    local label="$1"
    if [ "$(t_failures)" -eq "$group" ]; then
        printf 'test-mermaid-accuracy: %s: PASS\n' "$label"
    else
        printf 'test-mermaid-accuracy: %s: FAIL (%d finding(s))\n' "$label" "$(( $(t_failures) - group ))"
    fi
}

# ── The searchable tree ──────────────────────────────────────────────────────
# benchmark/results/ holds archived copies of whole older trees, so a script
# deleted from the live tree still "exists" there. `.plans/` is one developer's
# untracked working data and would make a result unreproducible. Prune both, and
# exclude this file from the haystacks so its own examples cannot satisfy a check.
find "$repo_root" -name .git -prune -o -path "$repo_root/benchmark/results" -prune \
    -o -path "$repo_root/.plans" -prune -o -path "$repo_root/.claude" -prune \
    -o -type f -print > "$work/tree.txt"
grep '\.sh$' "$work/tree.txt" | grep -vF "$self_path" > "$work/scripts.txt"
grep '\.md$' "$work/tree.txt" > "$work/markdown.txt"
for d in $docs; do
    grep -vF "$repo_root/$d" "$work/markdown.txt" > "$work/markdown.next"
    mv -f "$work/markdown.next" "$work/markdown.txt"
done
xargs cat < "$work/scripts.txt" > "$work/script-text.txt"
xargs cat < "$work/markdown.txt" > "$work/markdown-text.txt"

# Escape a filename for a BRE, and expand `*` into a path-safe wildcard, so
# `validate-plan-*-lib.sh` and a literal name go through one code path.
name_regex() {
    printf '%s' "$1" | sed -e 's/[.[]/\\&/g' -e 's/\*/[^\/]*/g'
}

tree_has() {
    local pattern="$1"
    case "$pattern" in
        *'*'*|-*) grep -q "/[^/]*$(name_regex "$pattern")\$" "$work/tree.txt" ;;
        *) grep -q "/$(name_regex "$pattern")\$" "$work/tree.txt" ;;
    esac
}

# A script writes the name if a redirect targets a path ending in it. The path
# may not contain a space, or prose after a `<placeholder>` matches too. This is
# the only "is it produced?" question that is mechanically answerable.
script_writes() {
    grep -qE ">[[:space:]]*\"?[^\"[:space:]]*$(name_regex "$1")\"?([[:space:]]|\$)" \
        "$work/script-text.txt"
}

group_start
# ── Check 1: structure ───────────────────────────────────────────────────────
# The parser lives in its own .awk file rather than inline: CODE-STYLE.md §3
# caps an inline awk program at 15 lines. It also emits every filename-shaped
# token it sees inside a block, so checks 2 and 3 need no second parser.
cat > "$work/mermaid-lint.awk" <<'AWK'
function reset() {
    inq = 0; sb = 0; cb = 0; pr = 0; kind = ""; stmts = 0; innote = 0
    split("", def); split("", ref); split("", refline)
}
function emit(sev, line, msg) { printf "%s\t%s\t%s\t%s\n", sev, FILENAME, line, msg }
function trim(s) { sub(/^[[:space:]]+/, "", s); sub(/[[:space:]]+$/, "", s); return s }
# Drop quoted spans, carrying the open-quote state across lines because mermaid
# allows a multi-line label, and count brackets on what is left.
function strip(s,   out, i, c) {
    out = ""
    for (i = 1; i <= length(s); i++) {
        c = substr(s, i, 1)
        if (c == "\"") { inq = !inq; continue }
        if (inq) continue
        if (c == "[") sb++
        else if (c == "]") sb--
        else if (c == "{") cb++
        else if (c == "}") cb--
        else if (c == "(") pr++
        else if (c == ")") pr--
        out = out c
    }
    return out
}
function tokens(s, lno,   rest) {
    rest = s
    while (match(rest, "[A-Za-z0-9_/*.-]+\\.(sh|md|json|tsv|txt|jsonl)")) {
        emit("TOKEN", lno, substr(rest, RSTART, RLENGTH))
        rest = substr(rest, RSTART + RLENGTH)
    }
}
function add_ref(id, line) {
    if (id != "" && id != "[*]" && !(id in ref)) { ref[id] = 1; refline[id] = line }
}
function ref_list(ids, line,   n, a, i) {
    n = split(ids, a, ",")
    for (i = 1; i <= n; i++) add_ref(trim(a[i]), line)
}
function flowchart(s, line,   n, a, i, p, id) {
    if (s ~ /^(classDef|style|linkStyle|click|direction|end)([[:space:]]|$)/ || s ~ /^%%/) return
    if (s ~ /^class([[:space:]]|$)/) { split(s, a, /[[:space:]]+/); ref_list(a[2], line); return }
    if (s ~ /^subgraph([[:space:]]|$)/) {
        split(s, a, /[[:space:]]+/); id = a[2]; sub(/[[({].*$/, "", id)
        if (id != "") def[id] = 1
        return
    }
    gsub(/\|[^|]*\|/, "", s)
    gsub(/-\.[^.]*\.->/, ";", s)
    gsub(/--[^->]*-->/, ";", s)
    gsub(/-\.->|-\.-|==>|===|-->|---|--x|--o|--/, ";", s)
    n = split(s, a, ";")
    for (i = 1; i <= n; i++) {
        p = trim(a[i])
        if (p == "" || p !~ /[A-Za-z0-9]/) continue
        stmts++
        if (p ~ /^[A-Za-z0-9_]+[[({]/) { id = p; sub(/[[({].*$/, "", id); def[id] = 1 }
        else if (p ~ /^[A-Za-z0-9_]+$/) add_ref(p, line)
        else emit("WARN", line, "unparsed flowchart fragment: " p)
    }
}
function sequence(s, line,   n, a, i, p) {
    if (s ~ /^(autonumber|end|else|and|activate|deactivate|loop|alt|opt|par|rect|box|critical|break|link|links)([[:space:]]|$)/) return
    if (s ~ /^(participant|actor)[[:space:]]/) { split(s, a, /[[:space:]]+/); def[a[2]] = 1; return }
    if (s ~ /^Note([[:space:]]|$)/) {
        sub(/:.*$/, "", s)
        sub(/^Note[[:space:]]+(over|(right|left)[[:space:]]+of)[[:space:]]+/, "", s)
        ref_list(trim(s), line); return
    }
    sub(/:.*$/, "", s)
    gsub(/-->>|->>|-->|->|--x|-x|--o|-o/, ";", s)
    n = split(s, a, ";")
    for (i = 1; i <= n; i++) {
        p = trim(a[i])
        if (p == "" || p !~ /[A-Za-z0-9]/) continue
        stmts++
        if (p ~ /^[A-Za-z0-9_]+$/) add_ref(p, line)
        else emit("WARN", line, "unparsed sequence fragment: " p)
    }
}
# A state is declared by being mentioned in a transition, so transitions define
# and only `note … of <state>` references.
function state(s, line,   n, a, i, p) {
    if (s ~ /^end note([[:space:]]|$)/) { innote = 0; return }
    if (s ~ /^note[[:space:]]/) {
        innote = 1; sub(/:.*$/, "", s)
        sub(/^note[[:space:]]+(right|left)[[:space:]]+of[[:space:]]+/, "", s)
        ref_list(trim(s), line); return
    }
    if (innote || s !~ /-->/) return
    n = split(s, a, /-->/)
    for (i = 1; i <= n; i++) {
        p = trim(a[i]); sub(/[[:space:]]*:.*$/, "", p); p = trim(p)
        if (p == "" || p == "[*]") continue
        stmts++
        if (p ~ /^[A-Za-z0-9_]+$/) def[p] = 1
        else emit("WARN", line, "unparsed state fragment: " p)
    }
}
function finish(   id) {
    if (kind == "") { emit("FAIL", start, "mermaid block declares no diagram type"); return }
    if (stmts == 0) emit("FAIL", start, "mermaid block has a header but no statements")
    if (inq) emit("FAIL", start, "unbalanced double quote in mermaid block")
    if (sb != 0) emit("FAIL", start, "unbalanced [] in mermaid block (delta " sb ")")
    if (cb != 0) emit("FAIL", start, "unbalanced {} in mermaid block (delta " cb ")")
    if (pr != 0) emit("FAIL", start, "unbalanced () in mermaid block (delta " pr ")")
    for (id in ref) if (!(id in def)) emit("FAIL", refline[id], "node id never defined: " id)
}
!inblock && /^```mermaid[[:space:]]*$/ { inblock = 1; start = NR; reset(); next }
inblock && /^```/ { finish(); inblock = 0; blocks++; next }
inblock {
    tokens($0, NR)
    s = trim(strip($0))
    if (s == "") next
    if (kind == "") {
        if (s ~ /^(flowchart|graph)[[:space:]]+(TD|TB|BT|LR|RL)$/) kind = "flow"
        else if (s ~ /^sequenceDiagram$/) kind = "seq"
        else if (s ~ /^stateDiagram(-v2)?$/) kind = "state"
        else { emit("FAIL", NR, "unrecognised diagram header: " s); kind = "unknown" }
        next
    }
    if (kind == "flow") flowchart(s, NR)
    else if (kind == "seq") sequence(s, NR)
    else if (kind == "state") state(s, NR)
}
END {
    if (inblock) emit("FAIL", start, "unterminated mermaid fence")
    emit("BLOCKS", 0, blocks + 0)
}
AWK

: > "$work/findings.txt"
for d in $docs; do
    awk -f "$work/mermaid-lint.awk" "$repo_root/$d" >> "$work/findings.txt"
done

while IFS="$(printf '\t')" read -r sev doc line msg; do
    case "$sev" in
        FAIL) note_fail "${doc#"$repo_root"/}:$line: $msg" ;;
        WARN) note_warn "${doc#"$repo_root"/}:$line: $msg" ;;
    esac
done < <(grep -E '^(FAIL|WARN)	' "$work/findings.txt" || true)

# The parser must have seen every fence, or a silently skipped block would make
# every other check vacuous for it.
fences=0
parsed=0
for d in $docs; do
    n="$(grep -c '^```mermaid' "$repo_root/$d" || true)"
    fences=$((fences + n))
    [ "$n" -gt 0 ] || note_fail "$d has no mermaid diagram"
done
parsed="$(awk -F'\t' '$1 == "BLOCKS" { total += $4 } END { print total + 0 }' "$work/findings.txt")"
[ "$parsed" -eq "$fences" ] \
    || note_fail "parsed $parsed mermaid blocks but the documents open $fences fences"
group_done "structure of $parsed mermaid blocks"

group_start
# ── Checks 2 and 3: named scripts and artifacts ──────────────────────────────
awk -F'\t' '$1 == "TOKEN" { print $4 "\t" $2 ":" $3 }' "$work/findings.txt" \
    | sort -u -k1,1 > "$work/named.txt"

scripts_seen=0
artifacts_seen=0
while IFS="$(printf '\t')" read -r name where; do
    # Repo-relative path, not basename: brainstorm/SKILL.md and
    # post-implementation-review/SKILL.md are both in the checked set.
    where="${where#"$repo_root"/}"
    case "$name" in
        *.sh)
            scripts_seen=$((scripts_seen + 1))
            tree_has "$name" && continue
            script_writes "$name" && continue
            note_fail "$where: diagram names script $name — no such file, and no script writes it"
            ;;
        *)
            artifacts_seen=$((artifacts_seen + 1))
            tree_has "$name" && continue
            grep -qF "$name" "$work/script-text.txt" && continue
            if grep -qF "$name" "$work/markdown-text.txt"; then
                note_warn "$where: artifact $name is documented but no script names it — agent-authored or dead"
            else
                note_fail "$where: diagram names artifact $name — no such file, and nothing in the repo names it"
            fi
            ;;
    esac
done < "$work/named.txt"
group_done "$scripts_seen scripts and $artifacts_seen artifacts named in diagrams"

group_start
# ── Check 4: functions and identifiers named in the documents ───────────────
# `name()` in the prose is a claim that a function exists; a bare backticked
# lower_snake token is a claim that some script knows the identifier.
funcs_seen=0
for d in $docs; do
    while IFS= read -r fn; do
        [ -n "$fn" ] || continue
        funcs_seen=$((funcs_seen + 1))
        grep -qE "^[[:space:]]*(function[[:space:]]+)?$fn\(\)" "$work/script-text.txt" \
            || note_fail "$d: names $fn() but no script defines it"
    done < <(grep -ohE '[a-z][a-z0-9_]*\(\)' "$repo_root/$d" | sed 's/()$//' | sort -u)

    while IFS= read -r id; do
        [ -n "$id" ] || continue
        # Whole-word, or a truncated name matches the real one as a substring.
        # No `\b`: it is GNU-only (CODE-STYLE.md §1).
        grep -qE "([^A-Za-z0-9_]|^)$id([^A-Za-z0-9_]|\$)" "$work/script-text.txt" \
            || note_fail "$d: names identifier $id but no script mentions it"
    done < <(grep -ohE '`[a-z][a-z0-9_]*[^`]*`' "$repo_root/$d" \
        | sed 's/^`\([a-z][a-z0-9_]*\).*/\1/' | grep '_' | sort -u)
done
group_done "$funcs_seen function names in the diagram documents"

group_start
# ── Render check: the only proof that mermaid itself accepts a block ─────────
# `mmdc` comes from the dev flake and is not a suite dependency, so this portion
# reports UNCONFIGURED when it is absent (CODE-STYLE.md §12) rather than passing
# quietly. The `mermaid-render` CI job is the leg that cannot be skipped.
mkdir -p "$work/mmd"
for d in $docs; do
    tag="$(printf '%s' "$d" | tr '/' '-')"
    awk -v out="$work/mmd" -v tag="$tag" '
        !inb && /^```mermaid[[:space:]]*$/ { inb = 1; f = sprintf("%s/%s@%d.mmd", out, tag, NR); next }
        inb && /^```/ { inb = 0; next }
        inb { print > f }
    ' "$repo_root/$d"
done

if command -v mmdc >/dev/null 2>&1; then
    rendered=0
    for f in "$work"/mmd/*.mmd; do
        rendered=$((rendered + 1))
        where="$(basename "$f" .mmd | tr '@' ':')"
        render_log="$work/render.log"
        if ! mmdc -q -i "$f" -o "$f.svg" >"$render_log" 2>&1; then
            note_fail "mmdc rejected the diagram at $where: $(tr '\n' ' ' < "$render_log" | cut -c1-200)"
        fi
    done
    group_done "mmdc rendered $rendered diagrams"
else
    printf '%s\n' 'test-mermaid-accuracy: UNCONFIGURED (mmdc) — the structural checks ran, mermaid syntax itself is unverified; `nix develop` provides mmdc, and the mermaid-render CI job always runs it' >&2
fi

[ "$(t_failures)" -eq 0 ] || exit 1
printf '%s\n' 'test-mermaid-accuracy: PASS'
