#!/usr/bin/env bash
# validate-plan-commands-lib.sh — the command-literal detector: every command
# literal in a step file or testing companion must be registered in the plan's
# commands.json, so the "when" context travels with the command.
#
# Sourced by validate-plan.sh; never executed. Requires
# validate-plan-common-lib.sh and the inventory data model. Reads `skill_root`
# for the never-executable-extension registry.

# shellcheck disable=SC2154
# The entry script and its sibling libs publish these globals; shellcheck lints
# each file alone and cannot see the assignments.
set -euo pipefail

# --- command registry: every command literal in a step file or testing
#     companion must be registered in the plan's commands.json. Detection is
#     language-agnostic — the registry itself is the vocabulary.
extensions_file="$skill_root/never-executable-extensions.json"

never_executable_ext() {  # rule 5: jq membership on the lowercase extension
    local seg ext
    seg="$(printf '%s\n' "$1" | awk '{ print $NF }')"
    seg="${seg%/}"
    seg="${seg##*/}"
    ext=".${seg##*.}"
    # PORTABILITY(case-conversion)
    ext="$(printf '%s' "$ext" | tr '[:upper:]' '[:lower:]')"
    jq -e --arg ext "$ext" 'index($ext)' "$extensions_file" >/dev/null 2>&1
}

command_spans() {
    awk '
        /^```/ { in_fence = !in_fence; next }
        {
            line = $0
            sub(/^[[:space:]]+/, "", line)
            if (line == "") next
            if (in_fence) {
                sub(/^\$[[:space:]]+/, "", line)
                sub(/^#[[:space:]]+/, "", line)
                if (line ~ /^(bin\/|vendor\/|\.\/|~\/)/) print "LINE:" line
                else if (line ~ /^\// && (line ~ /\/.*\// || line ~ /\.sh$/ || line ~ /[[:space:]]/)) print "LINE:" line
                else if (line ~ /^(composer|npm|php|mysql|curl|git|python|node|magerun|phpunit|phpstan|make|docker|sh|bash|zsh|env|sudo|npx)([[:space:]]|$)/ && line ~ /[[:space:]]/) print "LINE:" line
                next
            }
            while (match(line, /`[^`]+`/)) {
                span = substr(line, RSTART + 1, RLENGTH - 2)
                sub(/^[[:space:]]+/, "", span)
                sub(/[[:space:]]+$/, "", span)
                sub(/^\$[[:space:]]+/, "", span)
                if (span == "") { line = substr(line, RSTART + RLENGTH); continue }
                print "SPAN:" span
                line = substr(line, RSTART + RLENGTH)
            }
            if (line ~ /^(bin\/|vendor\/|\.\/|~\/)/) print "LINE:" line
            else if (line ~ /^\// && (line ~ /\/.*\// || line ~ /\.sh$/ || line ~ /[[:space:]]/)) print "LINE:" line
            else if (line ~ /^(composer|npm|php|mysql|curl|git|python|node|magerun|phpunit|phpstan|make|docker|sh|bash|zsh|env|sudo|npx)([[:space:]]|$)/ && line ~ /[[:space:]]/) print "LINE:" line
        }
    ' "$1"
}
span_token() { printf '%s\n' "$1" | awk '{ print $1 }'; }
span_rest() { printf '%s\n' "$1" | awk '{ $1=""; sub(/^ /, ""); print }'; }
bin_like() {  # any path segment is a bin-like directory
    local path="$1" seg
    [ -n "$path" ] || return 1
    while IFS= read -r seg; do
        case "$seg" in bin|sbin|.bin|Scripts) return 0 ;; esac
    done < <(printf '%s\n' "$path" | tr '/' '\n')
    return 1
}
bin_under() {  # last segment sits directly under a bin-like directory
    local token="$1" parent
    [[ "$token" == */* ]] || return 1
    parent="${token%/*}"
    case "${parent##*/}" in bin|sbin|.bin|Scripts) return 0 ;; esac
    return 1
}
is_executable() {  # ask the OS the same question a shell would (path-shaped tokens only; bare tool names come from the registry)
    local token="$1" cand="${1/#\~/$HOME}"
    [[ "$cand" == */* ]] || return 1
    [ -x "$cand" ] && [ ! -d "$cand" ]
}
command_shaped() {  # rules 1-3 for a single token
    local token="$1" word
    for word in $core_words; do
        [ "$token" = "$word" ] && return 0
    done
    if [ -n "$registered_words" ] && printf '%s\n' "$registered_words" | grep -Fxq -- "$token"; then
        return 0
    fi
    is_executable "$token" && return 0
    bin_under "$token" && return 0
    return 1
}
command_candidate() {  # report 21: rules 1-3 are the only entry points; arguments strengthen a qualifying span but never qualify one on their own
    local token
    token="$(span_token "$1")"
    command_shaped "$token"
}
command_disqualified() {  # report 20 §4 rules 5-8
    local span="$1" token rest arg seg cand
    token="$(span_token "$1")"
    rest="$(span_rest "$1")"
    # rule 5: never-executable data/markup extension on the last segment
    # (jq membership in never-executable-extensions.json, lowercased)
    if never_executable_ext "$1"; then
        return 0
    fi
    # rule 6: a :line or #Lnn suffix is a citation, never a command
    case "$span" in
        *:[0-9]*|*#[Ll][0-9]*) return 0 ;;
    esac
    # rule 7: route- or prose-shaped
    if [[ "$span" == /* ]] && ! bin_like "$span" && ! command_shaped "$token"; then
        return 0
    fi
    if [ -n "$rest" ]; then
        arg="$(span_token "$rest")"
        if [[ "$arg" == /* ]] && ! bin_like "$arg" && ! command_shaped "$token"; then
            return 0
        fi
    fi
    # rule 8: first token resolves to a directory
    cand="${token/#\~/$HOME}"
    [ -d "$cand" ] && return 0
    return 1
}
registered_command() {
    local span="$1" cmd
    while IFS= read -r cmd; do
        [ -n "$cmd" ] || continue
        if [ "$span" = "$cmd" ] || [[ "$span" == "$cmd "* ]]; then
            return 0
        fi
    done < <(jq -r '. | to_entries[] | .value.cmd' "$commands_file" 2>/dev/null || true)
    return 1
}

plan_validate_commands() {
    commands_file="$plan_dir/commands.json"
    [ -f "$commands_file" ] || return 0
    # Small cross-language core; everything else comes from the registry.
    core_words='git make docker sh bash zsh env sudo npx'
    registered_words="$(jq -r '. | to_entries[] | .value.cmd' "$commands_file" 2>/dev/null \
        | awk '{ print $1 }' | sort -u)"
    for id in ${unit_ids[@]+"${unit_ids[@]}"}; do
        plan_map_load unit_goal "$id" || plan_map_value=""
        cmd_unit_goal="$plan_map_value"
        plan_map_load unit_step "$id" || plan_map_value=""
        cmd_unit_step="$plan_map_value"
        for candidate_file in \
            "$plan_dir/$cmd_unit_goal/steps/$cmd_unit_step.md" \
            "$plan_dir/$cmd_unit_goal/steps/$cmd_unit_step-testing.md"; do
            [ -f "$candidate_file" ] || continue
            while IFS= read -r candidate; do
                [ -n "$candidate" ] || continue
                span="${candidate#*:}"
                if command_candidate "$span" && ! command_disqualified "$span" \
                    && ! registered_command "$span"; then
                    if [ "$complete_mode" = true ]; then
                        fail "$(basename "$candidate_file") uses unregistered command literal '$span'; register it with register-command.sh so its 'when' context is recorded"
                    else
                        warn "$(basename "$candidate_file") uses unregistered command literal '$span'; register it with register-command.sh so its 'when' context is recorded"
                    fi
                fi
            done < <(command_spans "$candidate_file" | sort -u)
        done
    done
}
