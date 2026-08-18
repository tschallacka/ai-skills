#!/usr/bin/env bash
# plan-map-lib — associative arrays for bash 3.2.
#
# `declare -A` is bash 4, so it aborts on stock macOS (CODE-STYLE.md §1). These
# emulate a map with flattened variable names.
#
# Usage: sourced (by plan-document-lib.sh; never executed).
#   plan_map_set  <map> <key> <value>
#   plan_map_get  <map> <key>            # value on stdout, 1 if unset
#   plan_map_load <map> <key>            # value in $plan_map_value, 1 if unset
#   plan_map_has  <map> <key>            # 0 if set, 1 if not; no output
#   plan_map_keys <map>                  # original keys, one per line
#
# In a loop use plan_map_load/plan_map_has; plan_map_get needs a command
# substitution, and the fork costs more than everything else the map does.
# Keys are hex-encoded per byte outside [A-Za-z0-9], so `AR-01` and `AR_01`
# stay distinct, the encoding reverses, and metacharacters are inert.
set -euo pipefail

# Value channel for plan_map_load. Not local: the caller reads it.
plan_map_value=""
# Encoded-key channel for the internal encoder, for the same reason.
plan_map_enc=""

plan_map_die() {
    # plan-document-lib.sh owns plan_die; stay usable if sourced on its own.
    if command -v plan_die >/dev/null 2>&1; then
        plan_die "$1" "${2:-70}"
    fi
    printf '%s: %s\n' "${0##*/}" "$1" >&2
    exit "${2:-70}"
}

plan_map_require_name() {
    case "$1" in
        '' | *[!A-Za-z0-9_]*) plan_map_die "Map name must be [A-Za-z0-9_]+: $1" 70 ;;
    esac
}

# Encode into $plan_map_enc rather than onto stdout, so callers need no
# command substitution.
plan_map_encode_into() {
    local key="$1" out="" i=0 len ch
    # Fast path: a key that is already [A-Za-z0-9]+ encodes to itself. Work-unit
    # ids take this path, which is the one that runs thousands of times.
    case "$key" in
        *[!A-Za-z0-9]*) ;;
        *) plan_map_enc="$key"; return 0 ;;
    esac
    # Byte-wise, so a multi-byte character encodes to a reversible byte run.
    local LC_ALL=C
    len="${#key}"
    while [ "$i" -lt "$len" ]; do
        ch="${key:$i:1}"
        case "$ch" in
            [A-Za-z0-9]) out="$out$ch" ;;
            *) out="$out$(printf '_%02x' "'$ch")" ;;
        esac
        i=$((i + 1))
    done
    plan_map_enc="$out"
}

# Retained for the documented API and for tests; forks at the call site.
plan_map_encode_key() {
    plan_map_encode_into "$1"
    printf '%s' "$plan_map_enc"
}

plan_map_decode_key() {
    local enc="$1" escaped
    local LC_ALL=C
    # An encoded key holds only [A-Za-z0-9_] and hex digits, so turning each
    # `_` into `\x` yields a %b escape string with nothing else to interpret.
    escaped="$(printf '%s' "$enc" | sed 's/_/\\x/g')"
    printf '%b' "$escaped"
}

plan_map_set() {
    local map="$1" keyvar list
    plan_map_require_name "$map"
    plan_map_encode_into "$2"
    keyvar="plan_map_keys__$map"
    # $3 is assigned as an expansion, never as substituted text, so a value
    # containing `;` or `$(…)` cannot execute.
    eval "plan_map__${map}__${plan_map_enc}=\"\$3\""
    list="${!keyvar:-}"
    case " $list " in
        *" $plan_map_enc "*) ;;
        *) eval "$keyvar=\"\$list\${list:+ }\$plan_map_enc\"" ;;
    esac
}

# 0 and $plan_map_value set, or 1 when the key was never set (so an empty
# stored value stays distinguishable from an absent one).
plan_map_load() {
    local map="$1" var
    plan_map_require_name "$map"
    plan_map_encode_into "$2"
    var="plan_map__${map}__${plan_map_enc}"
    if [ -z "${!var+set}" ]; then
        plan_map_value=""
        return 1
    fi
    plan_map_value="${!var}"
}

plan_map_has() {
    local map="$1" var
    plan_map_require_name "$map"
    plan_map_encode_into "$2"
    var="plan_map__${map}__${plan_map_enc}"
    [ -n "${!var+set}" ]
}

plan_map_get() {
    plan_map_load "$1" "$2" || return 1
    printf '%s\n' "$plan_map_value"
}

plan_map_keys() {
    local map="$1" keyvar enc
    plan_map_require_name "$map"
    keyvar="plan_map_keys__$map"
    # Deliberately unquoted: the encoded key list is space-delimited and every
    # entry is [A-Za-z0-9_]+, so there is nothing to glob or split wrongly.
    for enc in ${!keyvar:-}; do
        plan_map_decode_key "$enc"
        printf '\n'
    done
}

# Number of keys, the plan_map_* form of ${#map[@]}.
plan_map_count() {
    local map="$1" keyvar list count=0 enc
    plan_map_require_name "$map"
    keyvar="plan_map_keys__$map"
    list="${!keyvar:-}"
    for enc in $list; do
        count=$((count + 1))
    done
    printf '%s\n' "$count"
}

# Drop every key, the plan_map_* form of `map=()`. Unsets the value variables
# too, so a later plan_map_has on a cleared key correctly returns 1 rather than
# seeing a stale value.
plan_map_clear() {
    local map="$1" keyvar list enc
    plan_map_require_name "$map"
    keyvar="plan_map_keys__$map"
    list="${!keyvar:-}"
    for enc in $list; do
        unset "plan_map__${map}__${enc}"
    done
    eval "$keyvar=''"
}

