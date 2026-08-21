#!/usr/bin/env bash
# lib-portable.sh - the platform-dependent primitives the benchmark harness needs
# on both halves of a run. Sourced, never executed.
#
# Target is CODE-STYLE.md's floor: bash 3.2 with BSD userland (macOS) as well as
# bash 5 with GNU coreutils. Both setup-benchmark.sh and case/start-worker.sh
# source this, so a platform quirk is fixed in exactly one place.
#
#   benchmark_hash_file <file>        sha256 hex digest on stdout
#   benchmark_require_python3         die 69 unless python3 is on PATH
#   benchmark_sed_replace <old> <new> <file>
#                                     literal in-place substitution, no `sed -i`
#   benchmark_basenames <dir>         one entry name per line, no `find -printf`
#   benchmark_unique_suffix           collision-resistant id, no `date +%N`

# Hash helper: GNU, BSD, and openssl-only boxes all appear in the wild.
# Modelled on context_hash_file's three-way cascade.
if command -v sha256sum >/dev/null 2>&1; then
    benchmark_hash_file() { sha256sum "$1" | awk '{print $1}'; }
elif command -v shasum >/dev/null 2>&1; then
    benchmark_hash_file() { shasum -a 256 "$1" | awk '{print $1}'; }
elif command -v openssl >/dev/null 2>&1; then
    benchmark_hash_file() { openssl dgst -sha256 "$1" | awk '{print $NF}'; }
else
    benchmark_hash_file() {
        printf 'lib-portable.sh: need sha256sum, shasum, or openssl\n' >&2
        return 69
    }
fi

# python3 is a hard requirement of the case runner - it drives the approval
# checks, state synthesis, telemetry, and metadata. Fail once, early, with a
# sentence, instead of eleven interpreter tracebacks scattered through a run.
benchmark_require_python3() {
    command -v python3 >/dev/null 2>&1 && return 0
    printf '%s\n' "${0##*/}: python3 is required to run a benchmark case" >&2
    return 69
}

# Literal substring replacement: awk index/substr, so no byte of old/new is ever
# metacharacter. `awk -v` interprets backslash escapes, hence the doubling below.
# Stays pure awk — `sed -i` is GNU-only and the setup half must not need python3.
benchmark_sed_replace() {
    local old="$1" new="$2" file="$3" temp
    temp="$(mktemp "$file.replace.XXXXXX")"
    awk -v old="$(printf '%s' "$old" | sed 's/\\/\\\\/g')" \
        -v new="$(printf '%s' "$new" | sed 's/\\/\\\\/g')" '
        function replace(line, out, at) {
            out = ""
            while (old != "" && (at = index(line, old)) > 0) {
                out = out substr(line, 1, at - 1) new
                line = substr(line, at + length(old))
            }
            return out line
        }
        { print replace($0) }
    ' "$file" > "$temp" && mv -f "$temp" "$file"
}

# `find -printf '%f\n'` is GNU-only; -print plus a read loop is portable.
benchmark_basenames() {
    local entry
    find "$1" -maxdepth 1 -mindepth 1 -print | while IFS= read -r entry; do
        printf '%s\n' "${entry##*/}"
    done
}

# `date +%s%N` is GNU-only: BSD date emits a literal "N", which used to land in
# reviewer session ids. Whole seconds plus $RANDOM gives the same uniqueness.
benchmark_unique_suffix() {
    printf '%s%s%s\n' "$(date -u +%s)" "$RANDOM" "$RANDOM"
}
