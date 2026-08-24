#!/usr/bin/env bash
# MODE: PROD
# render-plan-overview.sh — render a plan directory into one self-contained
# HTML overview: progress donut, per-goal bars, feedback-cycle chart and
# generated narration. Inline CSS/JS only; the page reloads itself.
#
# Reads the canonical plan surfaces (plan-level and goal progress tables, the
# inventory, adversarial-review.md and its history) through one template in
# templates/, and writes a single file atomically. --watch stays running and
# re-renders whenever an input changes.
#
# Usage:
#   render-plan-overview.sh [--plan-dir] <plan-directory> [--out FILE] [--refresh N]
#   render-plan-overview.sh [--plan-dir] <plan-directory> --watch [N]
#   render-plan-overview.sh --help
#
# Exit codes: 64 bad invocation, 66 plan or template missing, 70 internal.

set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/plan-document-lib.sh
source "$script_dir/plan-document-lib.sh"
eval "set -- $(plan_hoist_plan_dir 1 "$@")"

export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--plan-dir] <plan-directory> [--out FILE] [--refresh N]
       ${0##*/} [--plan-dir] <plan-directory> --watch [N]
       ${0##*/} --help

Renders <plan-directory> into a single overview HTML (default
<plan>/overview.html). No external assets; the page reloads itself every
--refresh seconds so a watched render appears without user action.

  --out FILE     write to FILE instead of <plan>/overview.html
  --refresh N    page auto-reload seconds (default 15, minimum 5)
  --watch [N]    stay running; poll every N s (default 5) and re-render on change

OVERVIEW_NOW=<utc-timestamp> pins the embedded timestamp for deterministic
output (tests).
USAGE
    exit "$rc"
}

plan_dir="" out="" refresh=15 watch=false watch_every=5
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --out) [ "$#" -ge 2 ] || usage; out="$2"; shift 2 ;;
        --refresh) [ "$#" -ge 2 ] || usage; refresh="$2"; shift 2 ;;
        --watch)
            watch=true
            if [ "$#" -ge 2 ]; then
                case "$2" in *[!0-9]*|"") ;; *) watch_every="$2"; shift ;; esac
            fi
            shift ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *) [ -z "$plan_dir" ] || usage; plan_dir="$1"; shift ;;
    esac
done
[ -n "$plan_dir" ] || usage
case "$refresh" in ''|*[!0-9]*) usage ;; esac
[ "$refresh" -ge 5 ] || refresh=5
case "$watch_every" in ''|*[!0-9]*|0*) usage ;; esac
[ -n "$out" ] || out="$plan_dir/overview.html"

plan_require_directory "$plan_dir"
template="$script_dir/../templates/plan-overview.html.tmpl"
[ -f "$template" ] || plan_die "overview template not found: $template (broken install)" 66

# esc TEXT — HTML-escape the punctuation that survives plan table cells.
esc() {
    printf '%s' "$1" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g' -e 's/"/\&quot;/g'
}

# cells FILE HEADER_WORD — tab-separated body cells of one canonical table;
# skips the separator row and any header row (a header carries one of the
# fixed header labels in a cell, whichever column HEADER_WORD names).
cells() {
    awk -F'|' -v hdr="$2" '
        /^\|/ && $0 !~ /^\|[[:space:]]*-+/ {
            skip = 0
            for (i = 2; i < NF; i++) {
                c = $i; gsub(/^[[:space:]]+|[[:space:]]+$/, "", c)
                if (c == hdr || c == "Completion status" || c == "Status") skip = 1
            }
            if (skip) next
            out = ""
            for (i = 2; i < NF; i++) {
                c = $i; gsub(/^[[:space:]]+|[[:space:]]+$/, "", c)
                out = out (out == "" ? "" : "\t") c
            }
            if (out != "") print out
        }
    ' "$1"
}

status_glyph() {
    case "$1" in
        *✅*) printf '✅ st-done' ;;
        *⏳*) printf '⏳ st-wip' ;;
        *) printf '💤 st-todo' ;;
    esac
}

plan_progress="$plan_dir/progress.md"
review_file="$plan_dir/adversarial-review.md"
history_file="$plan_dir/adversarial-review-history.md"

total_steps=0 done_steps=0
goal_bars="" ledger="" goals_count=0
goals_list=""
if [ -f "$plan_progress" ]; then
    goals_list="$(cells "$plan_progress" Goalname | cut -f1 | sort -u)"
fi

for goal in ${goals_list:+$goals_list}; do
    goals_count=$((goals_count + 1))
    gf="$plan_dir/$goal/progress.md"
    g_done=0 g_all=0 g_pct=0
    if [ -f "$gf" ]; then
        while IFS="$(printf '\t')" read -r _g step _desc status; do
            [ -n "$step" ] || continue
            g_all=$((g_all + 1))
            glyphs="$(status_glyph "$status")"
            ic="${glyphs%% *}"; cls="${glyphs#* }"
            case "$cls" in st-done) g_done=$((g_done + 1)) ;; esac
            ledger="$ledger<div class=\"steprow\"><span class=\"st $cls\">$ic</span><span class=\"nm\">$(esc "$step")</span><span class=\"go\">$(esc "$goal")</span></div>"
        done < <(cells "$gf" Stepname)
    fi
    total_steps=$((total_steps + g_all)); done_steps=$((done_steps + g_done))
    [ "$g_all" -gt 0 ] && g_pct=$(( (g_done * 100 + g_all / 2) / g_all ))
    goal_bars="$goal_bars<div class=\"bar-row\"><span class=\"bar-name\" title=\"$(esc "$goal")\">$(esc "$goal")</span><div class=\"bar-track\"><div class=\"bar-fill\" data-w=\"$g_pct\"></div></div><span class=\"bar-pct\">$g_pct%</span></div>"
done

pct=0
[ "$total_steps" -gt 0 ] && pct=$(( (done_steps * 100 + total_steps / 2) / total_steps ))

# Findings from the current table: total, resolved, open.
f_total=0 f_resolved=0 f_open=0
if [ -f "$review_file" ]; then
    while IFS="$(printf '\t')" read -r fid _item _change status _wu; do
        case "$fid" in AR-[0-9]*) ;; *) continue ;; esac
        f_total=$((f_total + 1))
        case "$status" in *"✅"*|*resolved*) f_resolved=$((f_resolved + 1)) ;; *) f_open=$((f_open + 1)) ;; esac
    done < <(awk '/^## Findings$/{f=1; next} /^## Verdict$/{f=0} f' "$review_file" | cells /dev/stdin ID || true)
fi

# Rounds actually seen, from one normalised stream both the count and the chart
# read. History cycles are the record of completed rounds; each landing
# archives the previous table there, so once history exists the live Findings
# table merely mirrors the last round and must not add another. Only a plan
# with findings but no archive at all synthesises its first cycle from the live
# table. Placeholder rows never count, and cycles left empty by their removal
# disappear (B34).
live_rows=""
if [ -f "$review_file" ] && [ ! -f "$history_file" ]; then
    live_rows="$(awk '/^## Findings$/{f=1;next} /^## Verdict$/{f=0} f && /^\|[[:space:]]*AR-[0-9]/ && $0 !~ /No finding recorded yet/' "$review_file")"
fi
open_round=1
cycles_tmp="$(mktemp "${TMPDIR:-/tmp}/overview-cycles.XXXXXX")"
{
    if [ -f "$history_file" ]; then
        grep -v 'No finding recorded yet' "$history_file" || true
    fi
    if [ -n "$live_rows" ]; then
        printf '## Cycle live\n%s\n' "$live_rows"
    fi
} > "$cycles_tmp"
awk '
    function flush_block() {
        if (pending && rows > 0) {
            out++
            printf "## Cycle %d\n", out
            for (i = 1; i <= rows; i++) print kept[i]
        }
        pending = 0; rows = 0
    }
    BEGIN { pending = 0; rows = 0; out = 0 }
    /^## Cycle/ { flush_block(); pending = 1; next }
    # A cycle survives only if it holds at least one finding row; headers,
    # separators and the seeded "no finding yet" placeholder do not count.
    pending && /^\|[[:space:]]*AR-[0-9]/ { rows++; kept[rows] = $0; next }
    END { flush_block() }
' "$cycles_tmp" > "$cycles_tmp.final"
mv "$cycles_tmp.final" "$cycles_tmp"
cycles="$(grep -c '^## Cycle' "$cycles_tmp" || true)"
open_round=$((cycles + 1))

# The chart reads the same normalised stream, so bars can never disagree with
# the count (B34). Zero real rounds leave it empty for the placeholder below.
cycle_chart_svg=""
if [ -s "$cycles_tmp" ]; then
    cycle_chart_svg="$(awk '
        function flushc() { if (inc) { raised[++n] = rc; cums[++nc] = crc } }
        /^## Cycle [0-9]+/ { flushc(); rc = 0; inc = 1; next }
        inc && /^\|/ && $0 !~ /^\|[[:space:]]*-+/ {
            # Keep the Status cell: strip ID|item|change, then look at the rest.
            rest = $0; sub(/^(\|[^|]*){3}\|/, "", rest)
            if (rest ~ /✅|resolved/) crc++; rc++
        }
        END {
            flushc()
            m = 1
            for (i = 1; i <= n; i++) { if (raised[i] > m) m = raised[i]; if (cums[i] > m) m = cums[i] }
            W = 460; H = 150; pl = 26; pb = 22; tp = 12; span = W - pl - 14
            s = "<svg viewBox=\"0 0 " W " " H "\" role=\"img\" aria-label=\"findings per review cycle\">"
            s = s "<line class=\"axis\" x1=\"" pl "\" y1=\"" H - pb "\" x2=\"" W - 8 "\" y2=\"" H - pb "\"/>"
            s = s "<line class=\"axis\" x1=\"" pl "\" y1=\"" tp "\" x2=\"" pl "\" y2=\"" H - pb "\"/>"
            pts = ""
            for (i = 1; i <= n; i++) {
                x = pl + (i - 0.5) * span / n
                bh = (H - pb - tp) * raised[i] / m
                bw = span / n * 0.42
                s = s "<rect class=\"bar-f\" x=\"" sprintf("%.1f", x - bw / 2) "\" y=\"" sprintf("%.1f", H - pb - bh) "\" width=\"" sprintf("%.1f", bw) "\" height=\"" sprintf("%.1f", bh) "\" rx=\"3\"><title>cycle " i ": " raised[i] " raised</title></rect>"
                s = s "<text x=\"" sprintf("%.1f", x) "\" y=\"" H - 7 "\" text-anchor=\"middle\">" i "</text>"
                cy = H - pb - (H - pb - tp) * cums[i] / m
                pts = pts sprintf("%.1f,%.1f ", x, cy)
                s = s "<circle class=\"dot-res\" cx=\"" sprintf("%.1f", x) "\" cy=\"" sprintf("%.1f", cy) "\" r=\"3.2\"><title>" cums[i] " resolved cumulative</title></circle>"
            }
            if (n > 0) s = s "<polyline class=\"line-res\" points=\"" pts "\" stroke-dasharray=\"1200\"/>"
            print s "</svg>"
        }' "$cycles_tmp")"
fi
[ -n "$cycle_chart_svg" ] || cycle_chart_svg="<svg viewBox=\"0 0 460 150\"><text x=\"230\" y=\"78\" text-anchor=\"middle\">no review cycles recorded yet — the chart lights up after cycle 1</text></svg>"

# Phase: planning until the review approves, then implementation, then delivered.
state="planning"; phase_line="no adversarial review yet"
if [ -f "$review_file" ]; then
    # `- Status: `💤 pending`` — strip through the opening backtick; the
    # trailing one rides along harmlessly in the case patterns below.
    rstatus="$(sed -n 's/.*- Status:[[:space:]]*`//p' "$review_file" | head -1)"
    case "$rstatus" in
        *approved*)
            if [ "$total_steps" -gt 0 ] && [ "$done_steps" -eq "$total_steps" ]; then
                state="delivered"; phase_line="review approved · all $total_steps steps complete"
            else
                state="implementation"; phase_line="review approved · executing ($done_steps/$total_steps steps done)"
            fi ;;
        *) phase_line="adversarial review round $open_round open · execution waits for approval" ;;
    esac
fi
if [ "$state" = "implementation" ] && [ "$total_steps" -gt 0 ] && [ "$done_steps" -eq "$total_steps" ]; then
    state="delivered"; phase_line="all steps complete · awaiting sign-off"
fi

f_res_pct=0; [ "$f_total" -gt 0 ] && f_res_pct=$(( (f_resolved * 100 + f_total / 2) / f_total ))
review_target=3; review_depth=$(( cycles * 100 / review_target )); [ "$review_depth" -gt 100 ] && review_depth=100
donut_offset="$(awk -v p="$pct" 'BEGIN{printf "%.2f", 326.73*(1-p/100)}')"
ring_work="$(awk -v p="$pct" 'BEGIN{printf "%.2f", 100.53*(1-p/100)}')"
ring_find="$(awk -v p="$f_res_pct" 'BEGIN{printf "%.2f", 100.53*(1-p/100)}')"
ring_review="$(awk -v p="$review_depth" 'BEGIN{printf "%.2f", 100.53*(1-p/100)}')"
n_units=0
[ -f "$plan_dir/work-unit-inventory.md" ] && n_units="$(grep -cE '^\|[[:space:]]*W[0-9]' "$plan_dir/work-unit-inventory.md" || true)"

# Narration: four generated sentences, also strip-fed into the ticker.
next_step="none queued"
for goal in ${goals_list:+$goals_list}; do
    gf="$plan_dir/$goal/progress.md"
    [ -f "$gf" ] || continue
    hit="$(cells "$gf" Stepname | grep -v "$(printf '\t')✅" | head -1 || true)"
    if [ -n "$hit" ]; then
        next_step="$(printf '%s' "$hit" | cut -f2)"; break
    fi
done

plain_state="$state"; [ "$state" = "delivered" ] && plain_state="delivered"
narr1="State: $(esc "$phase_line")."
narr2="Work: $done_steps of $total_steps steps complete ($pct%); next up $(esc "$next_step")."
narr3="Feedback: $cycles review round(s) raised $f_total finding(s); $f_resolved resolved, $f_open open."
narr4="Scope: $n_units work units across $goals_count goal(s)."
narration="<span class=\"ln\">$narr1</span><span class=\"ln\">$narr2</span><span class=\"ln\">$narr3</span><span class=\"ln\">$narr4</span>"
ticker_plain="$(printf '%s %s %s %s' "$narr1" "$narr2" "$narr3" "$narr4" | sed -e 's/<[^>]*>//g')"
ticker="${ticker_plain}<b>◆</b>auto-reloads every ${refresh}s<b>◆</b>rendered by render-plan-overview.sh<b>◆</b>self-contained html"

generated="${OVERVIEW_NOW:-$(date -u '+%Y-%m-%dT%H:%MZ')}"
plan_name="$(sed -n 's/^# Plan: //p' "$plan_dir/plan-description.md" 2>/dev/null | head -1)"
[ -n "$plan_name" ] || plan_name="$(basename "$plan_dir")"
footer="$(esc "$(basename "$plan_dir") · generated $generated · single-file html, no external assets")"

# The substitution map: strict KEY<TAB>single-line-value records. Values are
# built without raw newlines so one awk pass can splice them safely.
subs_file="$(mktemp "${TMPDIR:-/tmp}/overview-subs.XXXXXX")"
out_tmp="$(mktemp "${TMPDIR:-/tmp}/overview-out.XXXXXX")"
trap 'rm -f "$subs_file" "$out_tmp"' EXIT
{
    printf 'PLAN_NAME\t%s\n' "$(esc "$plan_name")"
    printf 'STATE\t%s\n' "$state"
    printf 'PHASE_LINE\t%s\n' "$(esc "$phase_line")"
    printf 'REFRESH\t%s\n' "$refresh"
    printf 'REFRESH_JS\t%s\n' "$refresh"
    printf 'GENERATED\t%s\n' "$generated"
    printf 'N_GOALS\t%s\n' "$goals_count"
    printf 'N_STEPS\t%s\n' "$total_steps"
    printf 'N_UNITS\t%s\n' "$n_units"
    printf 'F_TOTAL\t%s\n' "$f_total"
    printf 'F_OPEN\t%s\n' "$f_open"
    printf 'F_RESOLVED\t%s\n' "$f_resolved"
    printf 'F_RES_PCT\t%s\n' "$f_res_pct"
    printf 'CYCLES\t%s\n' "$cycles"
    printf 'REVIEW_TARGET\t%s\n' "$review_target"
    printf 'REVIEW_DEPTH\t%s\n' "$review_depth"
    printf 'PCT\t%s\n' "$pct"
    printf 'CIRCUMFERENCE\t326.73\n'
    printf 'DONUT_OFFSET\t%s\n' "$donut_offset"
    printf 'RING_WORK\t%s\n' "$ring_work"
    printf 'RING_FIND\t%s\n' "$ring_find"
    printf 'RING_REVIEW\t%s\n' "$ring_review"
    printf 'GOAL_BARS\t%s\n' "$goal_bars"
    printf 'CYCLE_CHART\t%s\n' "$cycle_chart_svg"
    printf 'NARRATION\t%s\n' "$narration"
    printf 'STEP_LEDGER\t%s\n' "$ledger"
    printf 'TICKER\t%s\n' "$ticker"
    printf 'FOOTER\t%s\n' "$footer"
} > "$subs_file"

awk -F'\t' '
    NR == FNR {
        key = $1
        val = substr($0, length($1) + 2)
        map[key] = val
        next
    }
    {
        line = $0
        while (match(line, /@[A-Z_]+@/)) {
            key = substr(line, RSTART + 1, RLENGTH - 2)
            gsub(/^_+|_+$/, "", key)
            if (!(key in map)) { printf "%s: unresolved template token %s\n", ARGV[0], key > "/dev/stderr"; exit 70 }
            line = substr(line, 1, RSTART - 1) map[key] substr(line, RSTART + RLENGTH)
        }
        print line
    }
' "$subs_file" "$template" > "$out_tmp"

render_once() {
    if plan_atomic_write "$out" < "$out_tmp"; then
        printf 'Rendered %s\n' "$out"
    else
        plan_die "could not write $out" 70
    fi
}

if [ "$watch" = true ]; then
    # Without this, TERM arriving while bash waits on the sleep child is
    # deferred until that child exits, so a watcher can outlive its killer.
    trap 'exit 0' TERM INT
    render_once
    # A content checksum over every input, so edits inside any one of them
    # trigger a re-render; paths come from find and plans are kebab-cased.
    snapshot() {
        { cksum "$template"
          find "$plan_dir" -maxdepth 2 -type f \( -name '*.md' -o -name '*.json' \) -print | sort | while IFS= read -r f; do cksum "$f"; done
        } 2>/dev/null | cksum | awk '{print $1}'
    }
    prev="$(snapshot)"
    while :; do
        sleep "$watch_every"
        cur="$(snapshot)"
        if [ "$cur" != "$prev" ]; then
            render_once
            prev="$cur"
        fi
    done
else
    render_once
fi
