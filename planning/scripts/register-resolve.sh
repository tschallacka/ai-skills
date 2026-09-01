#!/usr/bin/env bash
# MODE: DEV
# register-resolve.sh — resolve id collisions in a conflicted BUGS.json or
# TODO.json in one pass, behind a confirm hash.
#
# Two branches that both append to a register both take the same next free id,
# so the conflict is semantic: one id, two unrelated entries. Git cannot resolve
# that, and neither side is wrong. This reads the clean copy of each side out of
# the index, reports every decision that has to be made, applies the decisions
# given on one command line, and prints the resolved register.
#
# Usage:
#   register-resolve.sh --check <register>
#   register-resolve.sh <register> [<side>:<old-id>:<new-id> ...]
#   register-resolve.sh --help
#
#   <side> is `ours` or `theirs`; the branch names git reports for HEAD and
#   MERGE_HEAD are accepted too. Every collision needs a decision in the SAME
#   invocation — a missing one is an error naming what is missing.
#
# The resolved register goes to stdout and its confirm hash to stderr, so
# `… > preview.json` keeps the file and leaves the hash on the terminal.
# Nothing is written in place until that hash is handed back.
#
# Exit codes: 1 --check found a conflict; 64 usage; 65 the register or a
# decision is unusable; 66 no such register; 69 rjq or a digest tool missing.

set -euo pipefail
export LC_ALL=C

# CONFIRM is deliberately absent from the usage above. It is never typed from
# scratch — only copied from the previous run's output, which prints the exact
# prefix to use — and documenting a spelling invites an invented one. usage()
# prints the comment block above and stops at the first non-comment line, so
# this note stays out of --help.
#
# An environment variable rather than a flag, for two reasons: the value is a
# token this tool produced rather than an option a caller picks, and a flag
# would have to be either documented (defeating the point) or exempted from
# tests/test-flag-coverage.sh's every-flag-is-documented contract.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=planning/scripts/plan-document-lib.sh
source "$script_dir/plan-document-lib.sh"
# shellcheck source=planning/scripts/register-lib.sh
source "$script_dir/register-lib.sh"

usage() {
    awk 'NR == 1 { next }
         /^#/ {
             sub(/^#[[:space:]]?/, "")
             print; next
         }
         { exit }' "$0"
    exit "${1:-64}"
}

note() { printf 'register-resolve: %s\n' "$*" >&2; }

# ─────────────────────────────────────────────────────────────────────────────
# Arguments
# ─────────────────────────────────────────────────────────────────────────────
check_only=false
# The confirmation arrives in the environment rather than as a flag: it is not
# an option a caller chooses, it is a token this tool printed, and an
# environment prefix reads that way at the call site.
confirm_hash="${CONFIRM:-}"
register=""
# Parallel arrays rather than an associative one: bash 3.2 has no `declare -A`
# (CODE-STYLE.md section 1), and the decision list is short.
map_sides=()
map_olds=()
map_news=()

while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --check) check_only=true; shift ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *:*:*)
            map_sides+=("${1%%:*}")
            map_olds+=("$(printf '%s' "$1" | awk -F: '{print $2}')")
            map_news+=("${1##*:}")
            shift
            ;;
        *)
            [ -z "$register" ] || usage
            register="$1"
            shift
            ;;
    esac
done

[ -n "$register" ] || usage
plan_require_file "$register"
reg_require_jq

# ─────────────────────────────────────────────────────────────────────────────
# The register's kind, read from its own shape rather than its filename, so a
# fixture copy under any name resolves the same way.
# ─────────────────────────────────────────────────────────────────────────────
kind=""
if rjq -e 'has("bugs")' "$register" >/dev/null 2>&1; then
    kind=bug
elif rjq -e 'has("tasks")' "$register" >/dev/null 2>&1; then
    kind=todo
fi
if [ -z "$kind" ]; then
    # A conflicted file is not valid JSON, so the shape cannot be read from it.
    # Fall back to the name, and say that is what happened.
    case "${register##*/}" in
        BUGS.json) kind=bug ;;
        TODO.json) kind=todo ;;
        *) plan_die "cannot tell whether $register is a bug or todo register: it is neither valid JSON with a bugs/tasks key nor named BUGS.json/TODO.json" 65 ;;
    esac
fi
entries_key=tasks
id_prefix=T
if [ "$kind" = bug ]; then entries_key=bugs; id_prefix=B; fi

# ─────────────────────────────────────────────────────────────────────────────
# Conflict detection. `git ls-files -u` is the authority rather than scanning
# for conflict markers: a register carrying the word "<<<<<<<" inside a quoted
# reproduction is not a conflicted file, and a conflicted file that a tool has
# already partly cleaned still has its stages in the index.
# ─────────────────────────────────────────────────────────────────────────────
repo_root="$(git rev-parse --show-toplevel 2>/dev/null)" \
    || plan_die "not inside a git work tree: $register" 65
register_abs="$(cd "$(dirname "$register")" && pwd -P)/$(basename "$register")"
case "$register_abs" in
    "$repo_root"/*) register_rel="${register_abs#"$repo_root"/}" ;;
    *) plan_die "register is outside the work tree at $repo_root: $register" 65 ;;
esac

unmerged="$(git -C "$repo_root" ls-files -u -- "$register_rel")"

if [ "$check_only" = true ]; then
    if [ -n "$unmerged" ]; then
        printf 'conflicted %s\n' "$register_rel"
        note "run ${0##*/} $register_rel to see the decisions it needs"
        exit 1
    fi
    printf 'clean %s\n' "$register_rel"
    exit 0
fi

if [ -z "$unmerged" ]; then
    printf 'clean %s\n' "$register_rel"
    note 'no conflict to resolve; nothing was written'
    exit 0
fi

# ─────────────────────────────────────────────────────────────────────────────
# The three clean copies. Stage 1 is the merge base and is absent when both
# sides added the file, which is why its failure is tolerated and the others
# are not.
# ─────────────────────────────────────────────────────────────────────────────
# plan_register_temp_file records a path for the cleanup and prints nothing, so
# the assignment comes first and the registration second.
base_file="$(mktemp "${TMPDIR:-/tmp}/reg-base.XXXXXX")"
ours_file="$(mktemp "${TMPDIR:-/tmp}/reg-ours.XXXXXX")"
theirs_file="$(mktemp "${TMPDIR:-/tmp}/reg-theirs.XXXXXX")"
plan_register_temp_file "$base_file"
plan_register_temp_file "$ours_file"
plan_register_temp_file "$theirs_file"

git -C "$repo_root" show ":1:$register_rel" > "$base_file" 2>/dev/null \
    || printf '{"%s":[]}\n' "$entries_key" > "$base_file"
git -C "$repo_root" show ":2:$register_rel" > "$ours_file" 2>/dev/null \
    || plan_die "no 'ours' stage for $register_rel in the index; is this a merge conflict?" 65
git -C "$repo_root" show ":3:$register_rel" > "$theirs_file" 2>/dev/null \
    || plan_die "no 'theirs' stage for $register_rel in the index; is this a merge conflict?" 65

for f in "$base_file" "$ours_file" "$theirs_file"; do
    rjq -e '.' "$f" >/dev/null 2>&1 \
        || plan_die "a clean side of $register_rel is not valid JSON; the conflict is not only an id collision, so resolve it by hand" 65
done

# Branch labels, so a caller may name the branch instead of ours/theirs.
# `rev-parse --abbrev-ref MERGE_HEAD` answers "MERGE_HEAD" rather than the
# branch, so name-rev does the naming and MERGE_HEAD is only the last resort.
ours_label="$(git -C "$repo_root" rev-parse --abbrev-ref HEAD 2>/dev/null || echo HEAD)"
theirs_label="$(git -C "$repo_root" name-rev --name-only MERGE_HEAD 2>/dev/null || true)"
case "${theirs_label:-}" in
    ''|undefined|*MERGE_HEAD*) theirs_label=MERGE_HEAD ;;
    # name-rev decorates a non-branch commit as tags/x or remotes/origin/x~2;
    # the trailing ~N or ^N is not part of a name a caller would type.
    *) theirs_label="${theirs_label%%[~^]*}" ;;
esac

# ─────────────────────────────────────────────────────────────────────────────
# What each side did since the merge base. An entry a branch ADDED is ordinary;
# an entry it MODIFIED is where a hand edit hides, so the changed keys are named
# rather than counted. Printed before any refusal below, because when a side is
# unsound this list is what says where to look.
#
# The comparison is by id, and `from_entries` keeps the last of a repeated id,
# so a register carrying the same id twice under-reports here. That is not this
# report's job to catch: the soundness check below refuses on `duplicate ids`.
# ─────────────────────────────────────────────────────────────────────────────
# The filter is a module-level constant so since_base() stays under the
# 40-line function cap (CODE-STYLE.md); the interpolations are jq's \(...),
# which a quoted here-doc leaves alone.
SINCE_BASE_JQ=$(cat <<'JQ'
        def byid($d): (($d[0][$key]) // []) | map({key: .id, value: .}) | from_entries;
        def changed_keys($a; $b):
            [ ((($a | keys) + ($b | keys)) | unique)[] as $k
              | select(($a[$k]) != ($b[$k])) | $k ];
        byid($base) as $b | byid($ours) as $o | byid($theirs) as $t
        | (($o | keys) - ($b | keys)) as $o_added
        | (($t | keys) - ($b | keys)) as $t_added
        | (($b | keys) - ($o | keys)) as $o_gone
        | (($b | keys) - ($t | keys)) as $t_gone
        | [ ($b | keys)[] as $id
            | select(($o | has($id)) and ($t | has($id)))
            | select(($o[$id]) != ($b[$id]))
            | select(($t[$id]) != ($b[$id]))
            | select(($o[$id]) == ($t[$id]))
            | $id ] as $shared
        | [ ($b | keys)[] as $id
            | select($o | has($id)) | select(($o[$id]) != ($b[$id]))
            | select(($shared | index($id)) == null)
            | { id: $id, keys: changed_keys($o[$id]; $b[$id]) } ] as $o_mod
        | [ ($b | keys)[] as $id
            | select($t | has($id)) | select(($t[$id]) != ($b[$id]))
            | select(($shared | index($id)) == null)
            | { id: $id, keys: changed_keys($t[$id]; $b[$id]) } ] as $t_mod
        | (if ($o_added | length) > 0
           then "  \($ours_label) added \(($o_added | length)): \($o_added | join(" "))" else empty end),
          (if ($t_added | length) > 0
           then "  \($theirs_label) added \(($t_added | length)): \($t_added | join(" "))" else empty end),
          (if ($o_gone | length) > 0
           then "  \($ours_label) REMOVED \(($o_gone | length)): \($o_gone | join(" "))" else empty end),
          (if ($t_gone | length) > 0
           then "  \($theirs_label) REMOVED \(($t_gone | length)): \($t_gone | join(" "))" else empty end),
          (if ($shared | length) > 0
           then "  modified identically on both sides, so the base is simply older: \($shared | join(" "))"
           else empty end),
          (if ($o_mod | length) > 0
           then "  modified on \($ours_label) ONLY — read these keys:" else empty end),
          ($o_mod[] | "      \(.id): \(.keys | join(", "))"),
          (if ($t_mod | length) > 0
           then "  modified on \($theirs_label) ONLY — read these keys:" else empty end),
          ($t_mod[] | "      \(.id): \(.keys | join(", "))")
JQ
)

since_base() {
    rjq -n -r \
        --slurpfile base "$base_file" \
        --slurpfile ours "$ours_file" \
        --slurpfile theirs "$theirs_file" \
        --arg key "$entries_key" \
        --arg ours_label "$ours_label" \
        --arg theirs_label "$theirs_label" \
        "$SINCE_BASE_JQ"
}
note "changed since the merge base:"
since_base >&2

# Each side is checked on its own before any decision is asked for, so an
# unsound side is reported as that side's problem. Without this the post-merge
# check fires instead and blames the decisions for damage they did not cause.
side_findings() { # <label> <file>
    local label="$1" file="$2" found
    found="$(reg_findings "$kind" "$file")" || true
    [ -n "$found" ] || return 0
    note "the '$label' side of $register_rel is not a sound $kind register on its own:"
    printf '%s\n' "$found" | sed 's/^/    /' >&2
    return 1
}
unsound=0
side_findings "ours ($ours_label)" "$ours_file" || unsound=1
side_findings "theirs ($theirs_label)" "$theirs_file" || unsound=1
if [ "$unsound" -eq 1 ]; then
    note 'fix that side on its own branch first: no id renaming can make a register'
    note 'sound that was already unsound, and merging it would carry the fault in.'
    plan_die "refusing to resolve into an unsound register" 65
fi

# ─────────────────────────────────────────────────────────────────────────────
# Classify. A collision is one id carrying two unrelated entries, which a
# rename fixes. A divergence is one entry edited differently on each side,
# which a rename would not fix and this tool refuses to guess at.
# ─────────────────────────────────────────────────────────────────────────────
classify() {
    rjq -n -r \
        --slurpfile base "$base_file" \
        --slurpfile ours "$ours_file" \
        --slurpfile theirs "$theirs_file" \
        --arg key "$entries_key" '
        def items($doc): (($doc[0][$key]) // []);
        def byid($doc): items($doc) | map({key: .id, value: .}) | from_entries;
        byid($base) as $b | byid($ours) as $o | byid($theirs) as $t
        # $id is bound before the `has` tests: inside `select($t | has(.))` the
        # `.` rebinds to $t, so the id has to be named to be asked about.
        | ($o | keys)[] as $id
        | select($t | has($id))
        | select(($o[$id]) != ($t[$id]))
        # An entry only one side touched is not a conflict: the changed side
        # wins, the way git merges a line only one branch edited. A divergence
        # is BOTH sides changing a base entry, differently.
        | if ($b | has($id))
          then (if (($o[$id]) != ($b[$id])) and (($t[$id]) != ($b[$id]))
                then "divergence\t\($id)" else empty end)
          else "collision\t\($id)" end
    '
}

collisions=()
divergences=()
while IFS=$'\t' read -r sort id; do
    [ -n "${id:-}" ] || continue
    case "$sort" in
        collision) collisions+=("$id") ;;
        divergence) divergences+=("$id") ;;
    esac
done < <(classify)

if [ "${#divergences[@]}" -gt 0 ]; then
    note "$register_rel has entries edited differently on both sides. A new id would not"
    note 'resolve those: the two versions are the same entry, so one content has to win.'
    for id in ${divergences[@]+"${divergences[@]}"}; do
        note "  divergence: $id"
    done
    plan_die "resolve the divergent entries by hand, then re-run" 65
fi

if [ "${#collisions[@]}" -eq 0 ]; then
    note 'no id collisions between the two sides; the conflict is textual only.'
    note "take either side and re-run: git checkout --theirs -- $register_rel"
    plan_die 'nothing for this tool to decide' 65
fi

# ─────────────────────────────────────────────────────────────────────────────
# Decisions. Every collision needs one, in this invocation.
# ─────────────────────────────────────────────────────────────────────────────
side_of() { # <label> -> ours|theirs, or empty when unknown
    case "$1" in
        ours|OURS) printf 'ours\n' ;;
        theirs|THEIRS) printf 'theirs\n' ;;
        HEAD) printf 'ours\n' ;;
        MERGE_HEAD) printf 'theirs\n' ;;
        "$ours_label") printf 'ours\n' ;;
        "$theirs_label") printf 'theirs\n' ;;
        *) printf '\n' ;;
    esac
}

# Highest id already spoken for anywhere, so a suggestion cannot land on an id
# either side is already using.
next_free="$(rjq -n -r \
    --slurpfile ours "$ours_file" \
    --slurpfile theirs "$theirs_file" \
    --arg key "$entries_key" '
    def nums($doc): (($doc[0][$key]) // []) | map(.id | capture("^[A-Za-z]*(?<n>[0-9]+)$").n | tonumber);
    (nums($ours) + nums($theirs)) | max // 0 | . + 1
')"

mapped_ours='{}'
mapped_theirs='{}'
i=0
while [ "$i" -lt "${#map_sides[@]}" ]; do
    label="${map_sides[$i]}"
    old="${map_olds[$i]}"
    new="${map_news[$i]}"
    side="$(side_of "$label")"
    [ -n "$side" ] || plan_die "unknown side '$label' in $label:$old:$new — use ours, theirs, $ours_label or $theirs_label" 64
    case "$new" in
        "$id_prefix"[0-9]*) : ;;
        *) plan_die "new id '$new' does not look like a $kind id (expected ${id_prefix}NN)" 64 ;;
    esac
    # A rename onto an id either side already uses would trade one collision
    # for another, silently.
    taken="$(rjq -n -r \
        --slurpfile ours "$ours_file" --slurpfile theirs "$theirs_file" \
        --arg key "$entries_key" --arg new "$new" '
        def ids($doc): (($doc[0][$key]) // []) | map(.id);
        if ((ids($ours) + ids($theirs)) | index($new)) then "yes" else "no" end')"
    [ "$taken" = no ] || plan_die "new id $new is already used by one of the two sides; pick a free id (next free is ${id_prefix}${next_free})" 64
    if [ "$side" = ours ]; then
        mapped_ours="$(printf '%s' "$mapped_ours" | rjq --arg o "$old" --arg n "$new" '.[$o] = $n')"
    else
        mapped_theirs="$(printf '%s' "$mapped_theirs" | rjq --arg o "$old" --arg n "$new" '.[$o] = $n')"
    fi
    i=$((i + 1))
done

missing=()
for id in ${collisions[@]+"${collisions[@]}"}; do
    have="$(printf '%s\n%s' "$mapped_ours" "$mapped_theirs" \
        | rjq -s -r --arg id "$id" 'if (.[0] | has($id)) or (.[1] | has($id)) then "yes" else "no" end')"
    [ "$have" = yes ] || missing+=("$id")
done

if [ "${#missing[@]}" -gt 0 ]; then
    printf 'These ids exist on both sides as different entries and each needs a decision:\n\n' >&2
    suggestion="$next_free"
    for id in ${missing[@]+"${missing[@]}"}; do
        printf '  %s\n' "$id" >&2
        printf '      ours   (%s): %s\n' "$ours_label" \
            "$(rjq -r --arg id "$id" --arg key "$entries_key" '(.[$key][] | select(.id == $id) | .title) // "(no title)"' "$ours_file")" >&2
        printf '      theirs (%s): %s\n' "$theirs_label" \
            "$(rjq -r --arg id "$id" --arg key "$entries_key" '(.[$key][] | select(.id == $id) | .title) // "(no title)"' "$theirs_file")" >&2
        printf '      decide with: theirs:%s:%s%s   (or ours:%s:%s%s)\n\n' \
            "$id" "$id_prefix" "$suggestion" "$id" "$id_prefix" "$suggestion" >&2
        suggestion=$((suggestion + 1))
    done
    plan_die "${#missing[@]} undecided collision(s); pass a decision for each one in a single run" 65
fi

# ─────────────────────────────────────────────────────────────────────────────
# Build the resolved register: every entry from ours, then every entry from
# theirs that ours does not already carry, with each side's renames applied to
# its own ids and to the parent/blocked_on references that point at them.
# ─────────────────────────────────────────────────────────────────────────────
resolved_file="$(mktemp "${TMPDIR:-/tmp}/reg-resolved.XXXXXX")"
plan_register_temp_file "$resolved_file"
rjq -n \
    --slurpfile base "$base_file" \
    --slurpfile ours "$ours_file" \
    --slurpfile theirs "$theirs_file" \
    --argjson ren_ours "$mapped_ours" \
    --argjson ren_theirs "$mapped_theirs" \
    --arg key "$entries_key" '
    # Each field is bound before the map is consulted: inside `$map | has(.id)`
    # the `.` rebinds to $map, so asking about .id there asks about the map.
    def apply($map):
        (.id) as $i | (.parent) as $p | (.blocked_on) as $bo
        | (if ($map | has($i)) then .id = $map[$i] else . end)
        | (if ($p != null) and ($map | has($p)) then .parent = $map[$p] else . end)
        | (if ($bo != null) and ($map | has($bo)) then .blocked_on = $map[$bo] else . end);
    def byid($list): $list | map({key: .id, value: .}) | from_entries;
    ($ours[0]) as $o
    | ($theirs[0]) as $t
    | (($base[0][$key]) // []) as $be
    | ((($o[$key]) // []) | map(apply($ren_ours))) as $oe
    | ((($t[$key]) // []) | map(apply($ren_theirs))) as $te
    | byid($be) as $bm | byid($oe) as $om | byid($te) as $tm
    # Per id, and NOT ours-first: an entry the base carries that only theirs
    # changed has to come from theirs, or that edit is silently dropped.
    # Renamed ids are new on both sides, so they are never in the base and fall
    # through to the "take the side that has it" arms.
    | def pick($id):
        if ($om | has($id)) and (($tm | has($id)) | not) then $om[$id]
        elif ($tm | has($id)) and (($om | has($id)) | not) then $tm[$id]
        elif ($om[$id]) == ($tm[$id]) then $om[$id]
        elif ($bm | has($id)) and (($om[$id]) == ($bm[$id])) then $tm[$id]
        elif ($bm | has($id)) and (($tm[$id]) == ($bm[$id])) then $om[$id]
        else $om[$id] end;
    # Ours order first, then the ids only theirs has, so the result is stable
    # before reg_sort reorders it.
    ((($oe | map(.id)) + ($te | map(.id))) | unique) as $all
    | (($oe | map(.id)) + ((($te | map(.id)) - ($oe | map(.id))) | sort)) as $order
    | $o
    | .[$key] = [ $order[] as $id | pick($id) ]
' > "$resolved_file"

findings="$(reg_findings "$kind" "$resolved_file")"
if [ -n "$findings" ]; then
    printf '%s\n' "$findings" >&2
    plan_die "the resolved register is not sound; the decisions given do not produce a valid $kind register" 65
fi
reg_sort "$kind" "$resolved_file"

# Prose references are reported, never rewritten: an id inside a sentence may
# belong to the other register, or be a prefix of a longer id, and a blind
# substitution corrupts text a person wrote.
prose="$(rjq -r \
    --argjson ren_ours "$mapped_ours" \
    --argjson ren_theirs "$mapped_theirs" \
    --arg key "$entries_key" '
    ($ren_ours + $ren_theirs) as $all
    | [ $all | keys[] ] as $olds
    | .[$key][] as $e
    | $olds[] as $old
    | [ $e.detail, $e.note, $e.notes, $e.mechanism, $e.fix, $e.verification,
        $e.observed, $e.expected, $e.reproduce, $e.title, $e.blocked_on ]
    | map(select(type == "string")) | join(" ") as $text
    | select($text | test("(^|[^A-Za-z0-9])" + $old + "($|[^0-9])"))
    | "  \($e.id) still mentions \($old) in its prose"
' "$resolved_file" | LC_ALL=C sort -u)"

cat "$resolved_file"

hash="$(plan_sha256_hex < "$resolved_file" | cut -c1-16)"

note "resolved $register_rel: $(rjq -r --arg key "$entries_key" '.[$key] | length' "$resolved_file") entries"
i=0
while [ "$i" -lt "${#map_sides[@]}" ]; do
    note "  renamed ${map_olds[$i]} -> ${map_news[$i]} on $(side_of "${map_sides[$i]}")"
    i=$((i + 1))
done
if [ -n "$prose" ]; then
    note 'renamed ids are still named in prose, which is left for a person to read:'
    printf '%s\n' "$prose" >&2
fi

if [ -z "$confirm_hash" ]; then
    note "nothing written. To write this exact content in place, re-run the same"
    note "command with this prefix:"
    note "  CONFIRM=$hash"
    exit 0
fi

if [ "$confirm_hash" != "$hash" ]; then
    note "the confirmation does not match this content."
    note "  given:    $confirm_hash"
    note "  computed: $hash"
    plan_die 'a stale confirmation means the register or the decisions changed since it was printed; re-read the new output and confirm that' 65
fi

cat "$resolved_file" | plan_atomic_write "$register_abs"
reg_write "$kind" "$register_abs"
note "wrote $register_rel — stage it when you have read it: git add $register_rel"
