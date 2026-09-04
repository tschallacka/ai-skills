#!/usr/bin/env bash
# MODE: DEV
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
    # install.sh is the one thing an end user runs: MODE: PROD. The parts it is
    # assembled from are MODE: DEV with PACKAGE: PROD -- a maintainer's files
    # whose content is compiled into this one. Their markers are stripped below,
    # because a marker copied out of a source would misdescribe the artifact, and
    # PACKAGE has no meaning on an output.
    printf '# MODE: PROD\n'
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

# ── any-of groups ─────────────────────────────────────────────────────────────
# A requires.tsv row may name a group in a fifth column: the requirement is
# then satisfied by ANY of the rows sharing that group id, so four server
# runtimes warn once when none is present instead of once each. Group rules,
# refused at build time rather than softened silently:
#   identical condition, strength and why across a group's rows — a mixed
#     strength would let one member quietly downgrade the others;
#   group id is [A-Za-z0-9_-]+ and unique across every skill, because the
#     generated member table is keyed by the id alone;
#   a hard group is allowed and gates exactly like a hard single tool.
# validate_one_requires_manifest <manifest>: every group inside one skill
# agrees on condition, strength and why, and every id is well-formed.
validate_one_requires_manifest() {
    local manifest="$1"
    awk -F '\t' -v prog="${0##*/}" -v manifest="$1" '
        /^[[:space:]]*#/ { next }
        NF == 0 { next }
        $1 == "tool" { next }
        {
            g = $5
            if (g == "" || g == "-") { next }
            if (g !~ /^[A-Za-z0-9_-]+$/) {
                printf "%s: bad group id %s in %s\n", prog, g, manifest | "cat 1>&2"
                close("cat 1>&2")
                exit 65
            }
            if (g in cond) {
                if (cond[g] != $2 || str[g] != $3 || why[g] != $4) {
                    printf "%s: %s of group %s disagrees on condition/strength/why\n", prog, $1, g | "cat 1>&2"
                    close("cat 1>&2")
                    exit 65
                }
                next
            }
            cond[g] = $2; str[g] = $3; why[g] = $4
        }
    ' "$manifest"
}

validate_requires_groups() {
    local manifest seen="" id
    while IFS= read -r manifest; do
        # Foreground, not a process substitution: a refusal that fires inside
        # one is lost by the reader loop, and the build would continue on a
        # half-read manifest.
        validate_one_requires_manifest "$manifest" || exit 65
        while IFS= read -r id; do
            [ -n "$id" ] || continue
            case "$seen" in
                *"|$id|"*)
                    printf '%s: group %s is declared by more than one skill\n' "${0##*/}" "$id" >&2
                    exit 65 ;;
            esac
            seen="$seen|$id|"
        done < <(awk -F '\t' '
            /^[[:space:]]*#/ { next }
            NF == 0 { next }
            $1 == "tool" { next }
            $5 != "" && $5 != "-" { print $5 }
        ' "$manifest" | LC_ALL=C sort -u)
    done < <(skill_manifests)
}

# T48b: every runtime a skill's shipped scripts probe with command -v (or a
# rung loop "for r in a b c") is declared in that skill's requires.tsv, or
# carries an allowlist row with a reason. Without this gate the next skill to
# grow a runtime dependency ships it silently — exactly how planning shipped a
# four-rung server with no declarations (B46/T48). The known-runtime universe
# is deliberate: an unknown tool name in a probe is not silently skipped, it
# simply needs a universe row when a skill adopts it.
probe_allowlist='installer/probe-allowlist.tsv'
probe_universe='python3 python node nodejs perl rjq openssl sha256sum shasum git'

shipped_scripts_of() { # <skill> — the prod arm of skill_files from the manifest part
    # Sourced fresh in a subshell so this build's own state is untouched;
    # sourcing parts is exactly what this builder does anyway.
    (
        # shellcheck source=installer/src/05-config.sh
        . "$src_dir/05-config.sh"
        # shellcheck source=installer/src/50-manifest.sh
        . "$src_dir/50-manifest.sh"
        skill_files "$1" prod
    ) 2>/dev/null || true
}

# probe_is_declared SKILL TOOL — requires.tsv row, else an allowlist row with
# a reason; exit status carries the verdict.
probe_is_declared() {
    local skill="$1" tool="$2"
    awk -F '\t' -v t="$tool" '
        /^[[:space:]]*#/ { next }
        $1 == t { found = 1 }
        END { exit (found ? 0 : 1) }
    ' "$repo_root/$skill/requires.tsv" 2>/dev/null && return 0
    awk -F '\t' -v s="$skill" -v t="$tool" '
        $1 == s && $2 == t { ok = 1 }
        END { exit (ok ? 0 : 1) }
    ' "$repo_root/$probe_allowlist"
}

# probe_universe_hits FILE — the known runtimes FILE probes, via literal
# command -v arms or a "for r in a b c" rung loop. grep exits 1 on zero
# matches, which under set -e + pipefail would abort a caller mid-scan (the
# PORTABILITY.md pipefail-grep trap), so every grep arm carries || true.
probe_universe_hits() {
    local pfile="$1"
    {
        { grep -oE 'command -v [a-z0-9_]+' "$pfile" 2>/dev/null || true; } \
            | awk '{print $3}'
        { grep -oE 'for r in [a-z0-9_ ]+' "$pfile" 2>/dev/null || true; } \
            | sed 's/for r in //' | tr ' ' '\n'
    } | awk -v u=" $probe_universe " 'index(u, " " $0 " ") > 0' | LC_ALL=C sort -u
}

validate_probes_declared() {
    [ -f "$repo_root/$probe_allowlist" ] || return 0
    local skill script tool declared missing=0
    local skill_dir
    while IFS= read -r skill_dir; do
        [ -n "$skill_dir" ] || continue
        skill="${skill_dir#"$repo_root/"}"
        [ -f "$repo_root/$skill/requires.tsv" ] || continue
        while IFS= read -r script; do
            case "$script" in
                *.sh|*.py|*.js|*.pl) ;;
                *) continue ;;
            esac
            [ -f "$repo_root/$skill/$script" ] || continue
            while IFS= read -r tool; do
                [ -n "$tool" ] || continue
                if probe_is_declared "$skill" "$tool"; then
                    continue
                fi
                printf '%s: %s probes runtime %s but its requires.tsv does not declare it and no allowlist row explains it\n' \
                    "${0##*/}" "$skill/$script" "$tool" >&2
                missing=1
            # The universe filter lives INSIDE the process substitution: a
            # pipe after `done` would put the loop itself in a subshell, and
            # its exit and missing-flag would die there (found the hard way).
            done < <(probe_universe_hits "$repo_root/$skill/$script")
        done < <(shipped_scripts_of "$skill")
    done < <(for skill_dir in "$repo_root"/*/; do
        [ -f "$skill_dir/requires.tsv" ] && printf '%s\n' "${skill_dir%/}"
    done)
    [ "$missing" -eq 0 ] || exit 65
}

# Manifest rows with group membership collapsed: the first row of a group stands
# for all of them as a requirement named @<group>, later members dropped after
# their condition/strength/why were checked against the first.
requires_rows() {
    local manifest="$1"
    awk -F '\t' -v OFS='\t' -v prog="${0##*/}" -v manifest="$manifest" '
        /^[[:space:]]*#/ { next }
        NF == 0 { next }
        $1 == "tool" { next }
        $5 == "" || $5 == "-" { print; next }
        $5 !~ /^[A-Za-z0-9_-]+$/ {
            printf "%s: bad group id %s in %s\n", prog, $5, manifest > "/dev/stderr"
            exit 65
        }
        $5 in cond {
            if (cond[$5] != $2 || str[$5] != $3 || why[$5] != $4) {
                printf "%s: %s of group %s disagrees on condition/strength/why\n", prog, $1, $5 | "cat 1>&2"
                close("cat 1>&2")
                exit 65
            }
            next
        }
        { cond[$5] = $2; str[$5] = $3; why[$5] = $4; $1 = "@" $5; $5 = ""; print }
    ' "$manifest"
}

# runtime_requirements() stays a self-contained case statement: it is extracted
# from install.sh and sourced on its own by test-limited-run-contract.sh.
gen_requirements() {
    local manifest skill tool condition strength why group
    printf 'runtime_requirements() {\n    local platform\n    platform="$(uname -s):$(uname -m)"\n'
    printf '    case "$1" in\n'
    while IFS= read -r manifest; do
        skill="$(basename "$(dirname "$manifest")")"
        printf '        %s)\n' "$skill"
        while IFS="$tab" read -r tool condition strength why group; do
            printf '%s\n' "            case \"\$platform\" in $condition) printf '%s\\n' $tool ;; esac"
        done < <(requires_rows "$manifest")
        printf '            ;;\n'
    done < <(skill_manifests)
    printf '    esac\n}\n'
}

# Members of one any-of group, one per line, keyed by @group. Group ids are
# unique across skills (validate_requires_groups), so no skill component is
# needed and every consumer can resolve a requirement entry with one lookup.
gen_requirement_members() {
    local manifest skill line
    printf 'runtime_requirement_members() {\n    local platform\n'
    printf '    platform="$(uname -s):$(uname -m)"\n    case "$1" in\n'
    while IFS= read -r manifest; do
        skill="$(basename "$(dirname "$manifest")")"
        while IFS="$tab" read -r group condition members; do
            [ -n "$group" ] || continue
            reject_quote "$members" "$skill:@$group members"
            printf '%s\n' "        @$group) case \"\$platform\" in $condition) printf '%s\\n' $members ;; esac ;;"
        done < <(awk -F '\t' '
            /^[[:space:]]*#/ { next }
            NF == 0 { next }
            $1 == "tool" { next }
            {
                g = $5
                if (g == "" || g == "-") { next }
                if (!(g in group_seen)) {
                    group_seen[g] = 1
                    count++
                    order[count] = g
                    cond[g] = $2
                }
                key = g SUBSEP $1
                if (!(key in member_seen)) {
                    member_seen[key] = 1
                    members[g] = members[g] (members[g] == "" ? "" : " ") $1
                }
            }
            END {
                for (i = 1; i <= count; i++) {
                    k = order[i]
                    print k "\t" cond[k] "\t" members[k]
                }
            }
        ' "$manifest")
    done < <(skill_manifests)
    printf '    esac\n}\n'
}

# <skill>:<tool> -> strength, and -> why. Two tables rather than one so each
# caller reads exactly the field it needs on one line of output.
gen_requirement_field() {
    local field="$1" name="$2" manifest skill tool condition strength why group value
    printf 'runtime_requirement_%s() {\n    local platform\n' "$name"
    printf '    platform="$(uname -s):$(uname -m)"\n    case "$1:$2" in\n'
    while IFS= read -r manifest; do
        skill="$(basename "$(dirname "$manifest")")"
        while IFS="$tab" read -r tool condition strength why group; do
            if [ "$field" = strength ]; then value="$strength"; else value="$why"; fi
            reject_quote "$value" "$skill:$tool"
            printf '%s\n' "        $skill:$tool) case \"\$platform\" in $condition) printf '%s\\n' '$value' ;; esac ;;"
        done < <(requires_rows "$manifest")
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
    validate_probes_declared || printf "GATE-RC=%s
" "$?" >&2
    validate_requires_groups
    gen_requirements
    printf '\n'
    gen_requirement_field strength strength
    printf '\n'
    gen_requirement_field why why
    printf '\n'
    gen_requirement_members
    printf '\n'
    gen_verify
    printf '\n'
    gen_install_hint
}

emit() {
    local part block
    block="$(mktemp "${TMPDIR:-/tmp}/dependency-block.XXXXXX")"
    # A silent generation failure would reach blast-radius as an empty capture
    # and read as "no output"; name the stage that died instead.
    if ! gen_dependency_block > "$block"; then
        printf '%s: generating the dependency block failed\n' "${0##*/}" >&2
        rm -f "$block"
        exit 70
    fi
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
