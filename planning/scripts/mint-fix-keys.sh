#!/usr/bin/env bash
# MODE: PROD
# mint-fix-keys.sh — derive a per-(finding, work-unit) SHA-256 fix key for
# every gated findings row in adversarial-review.md and record the derived keys
# in fix-keys.json beside the review file.
#
# The secret itself is never written into the plan: it lives in the session
# secret dir under the planning scratch dir (see W12 session lifecycle). The
# plan only ever holds the derived keys plus the session id that names the
# secret dir.
#
# Usage:
#   mint-fix-keys.sh [--plan-dir] <plan-directory>
#   mint-fix-keys.sh --help
#
# Exit codes: 65 = a gated findings row has non-conforming ids; 69 = no SHA-256
 # implementation (the plan-crypt binary, sha256sum or shasum) or no OS random
 # source is available.

set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/plan-document-lib.sh
source "$script_dir/plan-document-lib.sh"
# Accept --plan-dir as a synonym for the positional plan directory: the
# bounded reader takes the flag, so a reader who learned it there is not
# refused here.
eval "set -- $(plan_hoist_plan_dir 1 "$@")"

# mint_scan_gated FILE count|warn — one pass over the Findings table:
# counts gated rows (and non-conforming ones), or warns about each
# non-conforming row. Cells parse through the shared table helper.
mint_scan_gated() {
    local mfile="$1" mode="$2" in_f=0 line fid wu gated=0 skipped=0
    while IFS= read -r line || [ -n "$line" ]; do
        case "$line" in
            '## Findings') in_f=1; continue ;;
        esac
        [ "$in_f" = 1 ] || continue
        case "$line" in '## Verdict'*) break ;; esac
        case "$line" in '|'*) ;; *) continue ;; esac
        fid="$(plan_table_cell "$line" 2)"
        wu="$(plan_table_cell "$line" 6)"
        case "$fid" in ID|---) continue ;; esac
        case "$wu" in ---) continue ;; esac
        case "$wu" in ''|N/A|—) continue ;; esac
        gated=$((gated + 1))
        if [[ $fid =~ ^AR-[0-9]+$ && $wu =~ ^W[0-9]+$ ]]; then
            :
        else
            skipped=$((skipped + 1))
            if [ "$mode" = warn ]; then
                printf 'mint-fix-keys: WARN skipping gated row with non-conforming id: finding id "%s" work unit "%s" (expect ^AR-[0-9]+$ and ^W[0-9]+$)\n' "$fid" "$wu" >&2
            fi
        fi
    done < "$mfile"
    [ "$mode" = count ] && printf '%d\t%d\n' "$gated" "$skipped"
    return 0
}

export LC_ALL=C


# The `-gt 0` guard is required: expanding a possibly-empty array is unbound
# under `set -u` before bash 4.4.
tmp_files=()
cleanup_tmp() {
    if [ "${#tmp_files[@]}" -gt 0 ]; then
        rm -f "${tmp_files[@]}"
    fi
}
trap cleanup_tmp EXIT

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--plan-dir] <plan-directory>
       ${0##*/} --help
USAGE
    exit "$rc"
}

# No single SHA-256 tool is a hard requirement: the chain is the shipped
# plan-crypt binary -> sha256sum -> shasum. Probing it on empty input is the
# preflight, so this cannot drift from the chain plan_sha256_hex actually walks
# the way a duplicated `command -v` list would. Refuse up front with 69 rather
# than mint keys that would never verify.
plan_sha256_hex < /dev/null > /dev/null 2>&1 || \
    plan_die "no SHA-256 implementation available (need $(plan_sha256_chain)) to derive fix keys" 69

# plan_session_id PLAN_DIR — the session id recorded in fix-keys.json, or empty.
plan_session_id() {
    local plan_dir="$1"
    sed -n 's/.*"session_id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
        "$plan_dir/fix-keys.json" 2>/dev/null | head -1
}

# new_session_id — a fresh session id for a newly created session secret.
# Eight bytes of OS entropy, and nothing else: the id names the secret dir that
# gates approval, so a guessable value is a hole. The fallback this replaced was
# exactly that -- the process id, the clock truncated to a whole second, and two
# draws of the shell's own 15-bit PRNG, joined by hyphens (B87). It is described
# rather than quoted because the ratchets match file text and would read a
# quotation as a use.
new_session_id() {
    local id
    id="$(plan_random_hex 8)" || \
        plan_die 'cannot generate a session id: no OS random source (need the plan-crypt binary or a readable /dev/urandom)' 69
    [ -n "$id" ] || plan_die "cannot generate a session id"
    printf '%s\n' "$id"
}

session_secret_dir() {
    printf '%s/review-fix-keys/%s\n' "$(planning_tmpdir)" "$1"
}

# Reuse the session while its secret dir exists, so minting stays idempotent
# within a cycle and keys derived earlier keep verifying. A missing dir means
# the previous session was invalidated and a fresh one must be started.
ensure_session_secret() {
    local plan_dir="$1" session_id secret_file
    planning_ensure_tmpdir
    session_id="$(plan_session_id "$plan_dir")"
    if [ -n "$session_id" ] && [ -f "$(session_secret_dir "$session_id")/secret" ]; then
        printf '%s\n' "$session_id"
        return 0
    fi
    session_id="$(new_session_id)"
    secret_file="$(session_secret_dir "$session_id")/secret"
    mkdir -p "$(dirname "$secret_file")"
    chmod 700 "$(dirname "$secret_file")"
    # 32 bytes: 64 lowercase hex chars, which is what fix_key's "the secret has
    # a fixed width so the concatenation has one split" argument depends on.
    plan_random_hex 32 > "$secret_file" || \
        plan_die 'cannot generate a session secret: no OS random source (need the plan-crypt binary or a readable /dev/urandom)' 69
    chmod 600 "$secret_file"
    printf '%s\n' "$session_id"
}

# The derivation itself is plan_fix_key in plan-crypt-lib.sh, shared with
# verify-fix-keys.sh. It used to be a copy in each script under a comment
# saying the two must stay byte-identical; one definition is the mechanism that
# comment was asking for.

# A gated row whose ids fail ^AR-[0-9]+$ / ^W[0-9]+$ must fail the whole run,
# not be skipped: with no minted pairs left, verification reports "no gated
# pairs" and the fix-key gate silently disappears.
# ---- quoted: findings table columns ----
# ID | Missing or over-broad item | Required plan change | Status | Work unit
# ---- end quoted ----
# ---- quoted: fix-key derivation ----
# SHA-256 over (secret)(session_id|finding|work unit), secret first
# ---- end quoted ----
mint_fix_keys() {
    local plan_dir="$1" review_file="$1/adversarial-review.md"
    local json_file="$1/fix-keys.json"
    local session_id="${2:-}" secret_file secret pairs_file tsv_file minted_by
    local gated_rows skipped_rows fid wu count_file json_tmp
    [ -f "$review_file" ] || plan_die "adversarial-review.md not found: $review_file"
    if [ -z "$session_id" ]; then
        session_id="$(plan_session_id "$plan_dir")"
    fi
    [ -n "$session_id" ] || plan_die "no session secret available; ensure_session_secret must run first"
    secret_file="$(session_secret_dir "$session_id")/secret"
    [ -f "$secret_file" ] || plan_die "session secret missing: $secret_file"
    secret="$(cat "$secret_file")"

    # Named templates so a leaked temp is identifiable as this script's.
    pairs_file="$(mktemp "${TMPDIR:-/tmp}/mint-fix-keys-pairs.XXXXXX")"
    tsv_file="$(mktemp "${TMPDIR:-/tmp}/mint-fix-keys-tsv.XXXXXX")"
    tmp_files+=("$pairs_file" "$tsv_file")

    # Two passes: first count gated rows and rows that could not be minted
    # (counts written to a file so they survive the pipeline subshell), then
    # derive keys for the conforming pairs.
    count_file="$(mktemp "${TMPDIR:-/tmp}/mint-fix-keys-count.XXXXXX")"
    tmp_files+=("$count_file")
    mint_scan_gated "$review_file" count > "$count_file"
    IFS=$'\t' read -r gated_rows skipped_rows < "$count_file"

    if [ "$skipped_rows" -gt 0 ]; then
        mint_scan_gated "$review_file" warn
        plan_die "mint-fix-keys: $skipped_rows gated row(s) could not be minted; fix the finding/work-unit ids so the fix-key gate is not silently disabled"
    fi

    plan_review_gated_pairs "$review_file" > "$pairs_file"

    while IFS=$'\t' read -r fid wu; do
        [ -n "$fid" ] || continue
        printf '%s\t%s\t%s\n' "$fid" "$wu" "$(plan_fix_key "$secret" "$session_id|$fid|$wu")"
    done < "$pairs_file" > "$tsv_file"

    # Record the identity that minted so the approval gate can detect a fixer
    # claiming its own keys (self-certification). MINTED_BY overrides; default
    # to the session id that names the secret dir.
    minted_by="${MINTED_BY:-$session_id}"

    # A tracked temp in the target's own directory: the rename is atomic and the
    # cleanup trap removes it if awk fails. `$f.tmp.$$` had no trap at all.
    json_tmp="$(mktemp "$plan_dir/fix-keys.json.XXXXXX")"
    tmp_files+=("$json_tmp")
    # mktemp creates 0600; restore the mode a plain `>` redirect would have
    # produced so fix-keys.json keeps the permissions it had before.
    chmod "$(printf '%03o' "$(( 0666 & ~0$(umask) ))")" "$json_tmp"

    awk -v sid="$session_id" -v minted_by="$minted_by" '
        BEGIN { print "{"; printf "  \"session_id\": \"%s\",\n", sid; printf "  \"minted_by\": \"%s\",\n", minted_by; print "  \"keys\": {" }
        { fid = $1; wu = $2; key = $3
          if (fid != current_fid) {
              if (current_fid != "") print "    },"
              printf "    \"%s\": {\n", fid
              current_fid = fid
              first_in_fid = 1
          }
          if (first_in_fid) printf "      \"%s\": \"%s\"\n", wu, key
          else printf "    , \"%s\": \"%s\"\n", wu, key
          first_in_fid = 0
        }
        END {
            if (current_fid != "") print "    }"
            print "  }"
            print "}"
        }
    ' "$tsv_file" > "$json_tmp"
    mv -f "$json_tmp" "$json_file"
}

main() {
    case "${1:-}" in
        -h|--help) usage 0 ;;
    esac
    [ "$#" -eq 1 ] || usage
    plan_require_directory "$1"
    plan_git_snapshot "$1"
    session_id="$(ensure_session_secret "$1")"
    mint_fix_keys "$1" "$session_id"
    printf 'Minted fix keys for %s (session %s)\n' "$1" "$session_id"
}

main "$@"
