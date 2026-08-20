#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: DEV
# build — assemble install.sh from the ordered parts in installer/src/.
#
# install.sh is a build artifact. It stays committed and shipped because the
# README's first command is `curl … | bash` and it is the npm bin, so it has no
# siblings to source at runtime. Edit installer/src/NN-*.sh, then run this.
#
# Usage:
#   installer/build.sh [--check]
#
# --check assembles into a temp file and diffs instead of writing, exit 1 when
# the committed install.sh is stale. planning/tests/test-installer-build.sh runs it.
set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
src_dir="$script_dir/src"
output="$repo_root/install.sh"
check_only=false
case "${1:-}" in
    -h|--help) awk 'NR == 1 { next }
                    /^#/ {
                        sub(/^#[[:space:]]?/, "")
                        if ($0 ~ /^----[[:space:]]*(quoted:|end quoted)/) next
                        print; next
                    }
                    { exit }' "$0"; exit 0 ;;
    --check) check_only=true ;;
    '') ;;
    *) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; exit 64 ;;
esac

[ -d "$src_dir" ] || { printf '%s: missing %s\n' "${0##*/}" "$src_dir" >&2; exit 66; }

registry="$script_dir/tools.tsv"
[ -f "$registry" ] || { printf '%s: missing %s\n' "${0##*/}" "$registry" >&2; exit 66; }
tab="$(printf '\t')"

# The shebang and the generated banner are emitted here rather than living in
# 00-header.sh: the banner has to be the first thing a reader sees under the
# shebang, and a part file must not claim to be the artifact.
emit_banner() {
    printf '#!/usr/bin/env bash\n'
    # install.sh is the one thing an end user runs, so it is PROD in both axes.
    # The parts it is assembled from are MODE: DEV; their markers are stripped
    # below, because a marker copied out of a source would say the artifact is a
    # maintainer file.
    printf '# MODE: PROD\n'
    printf '# PACKAGE: PROD\n'
    printf '# GENERATED FILE — do not edit. Assembled from installer/src/*.sh by:\n'
    printf '#   installer/build.sh\n'
}

# ─────────────────────────────────────────────────────────────────────────────
# Dependency tables — generated from installer/tools.tsv and every skill's
# requires.tsv into install.sh between the marker comments in 20-runtime-tools.sh.
#
# They are generated rather than read at runtime because select_skills() and
# verify_runtime_tools() run before download_source(): under `curl | bash` the
# skill directories do not exist yet, so the installer has to carry the data.
# ─────────────────────────────────────────────────────────────────────────────
tsv_rows() {
    awk -F '\t' '
        /^[[:space:]]*#/ { next }
        NF == 0 { next }
        $1 == "kind" || $1 == "tool" { next }
        { print }
    ' "$1"
}

# Every text and reason is emitted inside a single-quoted shell word, so one of
# those characters would break the generated file rather than the table.
reject_quote() {
    case "$1" in
        *\'*) printf '%s: single quote in %s: %s\n' "${0##*/}" "$2" "$1" >&2; exit 65 ;;
    esac
}

skill_manifests() {
    local manifest
    for manifest in "$repo_root"/*/requires.tsv; do
        [ -f "$manifest" ] || continue
        printf '%s\n' "$manifest"
    done
}

# runtime_requirements() stays a self-contained case statement: it is extracted
# from install.sh and sourced on its own by test-limited-run-contract.sh.
gen_requirements() {
    local manifest skill tool condition strength why
    printf 'runtime_requirements() {\n    local platform\n    platform="$(uname -s):$(uname -m)"\n'
    printf '    case "$1" in\n'
    while IFS= read -r manifest; do
        skill="$(basename "$(dirname "$manifest")")"
        printf '        %s)\n' "$skill"
        while IFS="$tab" read -r tool condition strength why; do
            printf '%s\n' "            case \"\$platform\" in $condition) printf '%s\\n' $tool ;; esac"
        done < <(tsv_rows "$manifest")
        printf '            ;;\n'
    done < <(skill_manifests)
    printf '    esac\n}\n'
}

# <skill>:<tool> -> strength, and -> why. Two tables rather than one so each
# caller reads exactly the field it needs on one line of output.
gen_requirement_field() {
    local field="$1" name="$2" manifest skill tool condition strength why value
    printf 'runtime_requirement_%s() {\n    local platform\n' "$name"
    printf '    platform="$(uname -s):$(uname -m)"\n    case "$1:$2" in\n'
    while IFS= read -r manifest; do
        skill="$(basename "$(dirname "$manifest")")"
        while IFS="$tab" read -r tool condition strength why; do
            if [ "$field" = strength ]; then value="$strength"; else value="$why"; fi
            reject_quote "$value" "$skill:$tool"
            printf '%s\n' "        $skill:$tool) case \"\$platform\" in $condition) printf '%s\\n' '$value' ;; esac ;;"
        done < <(tsv_rows "$manifest")
    done < <(skill_manifests)
    printf '    esac\n}\n'
}

# The verify method is a command, not a tool name, so a tool that exists but is
# the wrong build can grow a functional probe without touching the generator.
gen_verify() {
    local kind tool condition group probe text fallback=""
    printf 'runtime_tool_verify() {\n    case "$1" in\n'
    while IFS="$tab" read -r kind tool condition group probe text; do
        [ "$kind" = verify ] || continue
        if [ "$tool" = '*' ]; then fallback="$text"; continue; fi
        printf '%s\n' "        $tool) $text >/dev/null 2>&1 ;;"
    done < <(tsv_rows "$registry")
    [ -z "$fallback" ] || printf '%s\n' "        *) $fallback >/dev/null 2>&1 ;;"
    printf '    esac\n}\n'
}

registry_field() {
    tsv_rows "$registry" | awk -F '\t' -v want="$1" -v col="$2" \
        -v t="${3:-}" -v c="${4:-}" -v g="${5:-}" '
        $1 != want { next }
        t != "" && $2 != t { next }
        c != "" && $3 != c { next }
        g != "" && $4 != g { next }
        !seen[$col]++ { print $col }
    '
}

# Rows of one (tool, condition, group) become one first-match chain: the first
# probe that resolves prints. A `-` probe always applies and closes the chain; a
# group with no `-` row may legitimately print nothing.
emit_hint_group() {
    local indent="$1" tool="$2" condition="$3" group="$4" kind probe text first=1 opened=0
    while IFS="$tab" read -r kind probe text; do
        reject_quote "$text" "$tool hint"
        if [ "$probe" = - ] && [ "$first" -eq 1 ]; then
            printf '%s\n' "${indent}printf '%s\\n' '$text'"
        elif [ "$probe" = - ]; then
            printf '%s\n' "${indent}else"
            printf '%s\n' "${indent}    printf '%s\\n' '$text'"
        elif [ "$first" -eq 1 ]; then
            printf '%s\n' "${indent}if command -v $probe >/dev/null 2>&1; then"
            printf '%s\n' "${indent}    printf '%s\\n' '$text'"
            opened=1
        else
            printf '%s\n' "${indent}elif command -v $probe >/dev/null 2>&1; then"
            printf '%s\n' "${indent}    printf '%s\\n' '$text'"
        fi
        first=0
    done < <(tsv_rows "$registry" | awk -F '\t' -v t="$tool" -v c="$condition" -v g="$group" '
        $1 == "hint" && $2 == t && $3 == c && $4 == g { print $1 "\t" $5 "\t" $6 }')
    [ "$opened" -eq 0 ] || printf '%s\n' "${indent}fi"
}

gen_install_hint() {
    local tool condition group text
    printf 'runtime_tool_install_hint() {\n    local tool="$1" platform\n'
    printf '    platform="$(uname -s):$(uname -m)"\n    case "$tool" in\n'
    while IFS= read -r tool; do
        printf '        %s)\n            case "$platform" in\n' "$tool"
        while IFS= read -r condition; do
            printf '                %s)\n' "$condition"
            while IFS= read -r group; do
                emit_hint_group '                    ' "$tool" "$condition" "$group"
            done < <(registry_field hint 4 "$tool" "$condition" | LC_ALL=C sort)
            printf '                    ;;\n'
        done < <(registry_field hint 3 "$tool")
        printf '            esac\n            ;;\n'
    done < <(registry_field hint 2)
    text="$(tsv_rows "$registry" | awk -F '\t' '$1 == "default" { print $6; exit }')"
    reject_quote "$text" 'default hint'
    [ -z "$text" ] || printf '%s\n' "        *)
            printf '$text\\n' \"\$tool\"
            ;;"
    printf '    esac\n}\n'
}

gen_dependency_block() {
    gen_requirements
    printf '\n'
    gen_requirement_field strength strength
    printf '\n'
    gen_requirement_field why why
    printf '\n'
    gen_verify
    printf '\n'
    gen_install_hint
}

emit() {
    local part block
    block="$(mktemp "${TMPDIR:-/tmp}/dependency-block.XXXXXX")"
    gen_dependency_block > "$block"
    emit_banner
    for part in "$src_dir"/[0-9][0-9]-*.sh; do
        [ -f "$part" ] || { printf '%s: no parts in %s\n' "${0##*/}" "$src_dir" >&2; exit 66; }
        # The part's own MODE/PACKAGE markers are dropped: they describe the
        # source file, and emit_banner has already declared what install.sh is.
        awk -v blockfile="$block" '
            /^# MODE: (DEV|PROD)$/ { next }
            /^# PACKAGE: (DEV|PROD)$/ { next }
            /^# BEGIN GENERATED DEPENDENCY BLOCK$/ {
                print
                while ((getline line < blockfile) > 0) print line
                close(blockfile)
                next
            }
            { print }
        ' "$part"
    done
    rm -f "$block"
}

if [ "$check_only" = true ]; then
    temporary="$(mktemp "${TMPDIR:-/tmp}/install.sh.XXXXXX")"
    trap 'rm -f "$temporary"' EXIT
    emit > "$temporary"
    if ! diff -u "$output" "$temporary"; then
        printf '%s: install.sh is stale; run installer/build.sh\n' "${0##*/}" >&2
        exit 1
    fi
    printf '%s\n' 'install.sh is up to date'
    exit 0
fi

temporary="$(mktemp "$(dirname "$output")/.install.sh.XXXXXX")"
trap 'rm -f "$temporary"' EXIT
emit > "$temporary"
chmod 755 "$temporary"
mv -f "$temporary" "$output"
trap - EXIT
printf 'Wrote %s\n' "$output"
