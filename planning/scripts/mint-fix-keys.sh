#!/usr/bin/env bash
# mint-fix-keys.sh — derive a per-(finding, work-unit) HMAC-SHA256 fix key for
# every gated findings row in adversarial-review.md and record the derived keys
# in fix-keys.json beside the review file.
#
# The secret itself is never written into the plan: it lives in the session
# secret dir under the planning scratch dir (see W12 session lifecycle). The
# plan only ever holds the derived keys plus the session id that names the
# secret dir.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/plan-document-lib.sh"

tmp_files=()
cleanup_tmp() {
    if [ "${#tmp_files[@]}" -gt 0 ]; then
        rm -f "${tmp_files[@]}"
    fi
}
trap cleanup_tmp EXIT

usage() {
    printf 'Usage: %s <plan-directory>\n' "$(basename "$0")" >&2
    exit 64
}

# plan_session_id PLAN_DIR — the session id recorded in fix-keys.json, or empty.
plan_session_id() {
    local plan_dir="$1"
    sed -n 's/.*"session_id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
        "$plan_dir/fix-keys.json" 2>/dev/null | head -1
}

# new_session_id — a fresh session id for a newly created session secret.
new_session_id() {
    local id
    id="$(openssl rand -hex 8 2>/dev/null || printf '%s-%s' "$$" "$(date +%s%N)")"
    [ -n "$id" ] || plan_die "cannot generate a session id"
    printf '%s\n' "$id"
}

session_secret_dir() {
    printf '%s/review-fix-keys/%s\n' "$(planning_tmpdir)" "$1"
}

# ensure_session_secret PLAN_DIR — establish the session secret for a plan and
# print the session id. Reuses the existing session while its secret dir is
# present (minting stays idempotent within a cycle, so keys derived earlier
# keep verifying); a missing dir — e.g. after the approval gate invalidated the
# previous session — starts a fresh session (re-mint).
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
    if ! openssl rand -hex 32 > "$secret_file" 2>/dev/null; then
        head -c 64 /dev/urandom | od -An -vtx1 | tr -d ' \n' > "$secret_file"
    fi
    chmod 600 "$secret_file"
    printf '%s\n' "$session_id"
}

# hmac_key SECRET MESSAGE — lowercase hex HMAC-SHA256 of MESSAGE under SECRET.
hmac_key() {
    local secret="$1" message="$2"
    printf '%s' "$message" | openssl dgst -sha256 -hmac "$secret" -binary \
        | od -An -vtx1 | tr -d ' \n'
}

# mint_fix_keys PLAN_DIR [SESSION_ID] — parse the 5-column findings table
# (ID | Missing or over-broad item | Required plan change | Status | Work unit)
# and write fix-keys.json: one hex key per (finding, work unit) row, derived as
# HMAC-SHA256(secret, "<session_id>|<finding>|<work unit>"). Rows without a
# work unit carry no key. Fails when the session secret is unavailable — the
# production creation path is ensure_session_secret (W12); tests seed the
# secret dir themselves.
#
# Non-conforming rows fail loudly: a gated row (work unit present) whose
# finding id or work-unit id does not match ^AR-[0-9]+$ / ^W[0-9]+$ is warned
# per row, and the whole run exits non-zero when any gated row could not be
# minted. A silently skipped row would otherwise disable the entire fix-key
# gate (verify then reports "no gated pairs … no fix verification required").
mint_fix_keys() {
    local plan_dir="$1" review_file="$1/adversarial-review.md"
    local json_file="$1/fix-keys.json"
    local session_id="${2:-}" secret_file secret pairs_file tsv_file minted_by
    local gated_rows skipped_rows fid wu
    [ -f "$review_file" ] || plan_die "adversarial-review.md not found: $review_file"
    if [ -z "$session_id" ]; then
        session_id="$(plan_session_id "$plan_dir")"
    fi
    [ -n "$session_id" ] || plan_die "no session secret available; ensure_session_secret must run first"
    secret_file="$(session_secret_dir "$session_id")/secret"
    [ -f "$secret_file" ] || plan_die "session secret missing: $secret_file"
    secret="$(cat "$secret_file")"

    pairs_file="$(mktemp)"
    tsv_file="$(mktemp)"
    tmp_files+=("$pairs_file" "$tsv_file")

    # Two passes: first count gated rows and rows that could not be minted
    # (counts written to a file so they survive the pipeline subshell), then
    # derive keys for the conforming pairs.
    count_file="$(mktemp)"
    tmp_files+=("$count_file")
    awk -F'|' '
        /^## Findings$/ { in_findings = 1; next }
        in_findings && /^## Verdict$/ { exit }
        in_findings && /^\|/ {
            fid = $2; wu = $6
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", fid)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", wu)
            # Skip header and separator rows (ID column is a marker or ---).
            if (fid ~ /^(ID|---)$/ || wu ~ /^---$/) next
            if (wu == "" || wu == "N/A" || wu == "—") next
            gated_rows++
            if (fid ~ /^AR-[0-9]+$/ && wu ~ /^W[0-9]+$/) mintable++
            else skipped_rows++
        }
        END {
            printf "%d\t%d\n", gated_rows + 0, skipped_rows + 0
        }
    ' "$review_file" > "$count_file"
    IFS=$'\t' read -r gated_rows skipped_rows < "$count_file"

    if [ "$skipped_rows" -gt 0 ]; then
        awk -F'|' -v seen=0 '
            /^## Findings$/ { in_findings = 1; next }
            in_findings && /^## Verdict$/ { exit }
            in_findings && /^\|/ {
                fid = $2; wu = $6
                gsub(/^[[:space:]]+|[[:space:]]+$/, "", fid)
                gsub(/^[[:space:]]+|[[:space:]]+$/, "", wu)
                if (fid ~ /^(ID|---)$/ || wu ~ /^---$/) next
                if (wu == "" || wu == "N/A" || wu == "—") next
                if (fid ~ /^AR-[0-9]+$/ && wu ~ /^W[0-9]+$/) next
                printf "mint-fix-keys: WARN skipping gated row with non-conforming id: finding id \"%s\" work unit \"%s\" (expect ^AR-[0-9]+$ and ^W[0-9]+$)\n", fid, wu > "/dev/stderr"
            }
        ' "$review_file"
        plan_die "mint-fix-keys: $skipped_rows gated row(s) could not be minted; fix the finding/work-unit ids so the fix-key gate is not silently disabled"
    fi

    awk -F'|' '
        /^## Findings$/ { in_findings = 1; next }
        in_findings && /^## Verdict$/ { exit }
        in_findings && /^\|/ {
            fid = $2; wu = $6
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", fid)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", wu)
            if (fid ~ /^AR-[0-9]+$/ && wu ~ /^W[0-9]+$/) print fid "\t" wu
        }
    ' "$review_file" > "$pairs_file"

    while IFS=$'\t' read -r fid wu; do
        [ -n "$fid" ] || continue
        printf '%s\t%s\t%s\n' "$fid" "$wu" "$(hmac_key "$secret" "$session_id|$fid|$wu")"
    done < "$pairs_file" > "$tsv_file"

    # Record the identity that minted so the approval gate can detect a fixer
    # claiming its own keys (self-certification). MINTED_BY overrides; default
    # to the session id that names the secret dir.
    minted_by="${MINTED_BY:-$session_id}"

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
    ' "$tsv_file" > "$json_file.tmp.$$"
    mv "$json_file.tmp.$$" "$json_file"
}

main() {
    [ "$#" -eq 1 ] || usage
    plan_require_directory "$1"
    plan_git_snapshot "$1"
    session_id="$(ensure_session_secret "$1")"
    mint_fix_keys "$1" "$session_id"
    printf 'Minted fix keys for %s (session %s)\n' "$1" "$session_id"
}

main "$@"
