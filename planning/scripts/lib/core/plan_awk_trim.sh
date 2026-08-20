#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: DEV
# The shared awk prelude defining trim(). Used as `awk "$(plan_awk_trim) …"`.
# Both ends are anchored: an unanchored `[[:space:]]+$` alternative strips
# interior whitespace runs and silently mangles table cells.
plan_awk_trim() {
    cat <<'AWK'
function trim(v) { gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); return v }
AWK
}
