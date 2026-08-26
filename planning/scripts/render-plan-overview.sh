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

plan_dir="" out="" refresh=15 watch=false watch_every=5 serve=false
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --out) [ "$#" -ge 2 ] || usage; out="$2"; shift 2 ;;
        --refresh) [ "$#" -ge 2 ] || usage; refresh="$2"; shift 2 ;;
        --serve) serve=true; shift ;;
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
# sec FILE HEADING: extract paragraphs under a section heading.
sec() {
    awk -v h="$2" ' $0 == "## " h { f = 1; next } /^## / && f { exit } f { print } ' "$1"
}

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
review_target=2; review_depth=$(( cycles * 100 / review_target )); [ "$review_depth" -gt 100 ] && review_depth=100
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

# ---- T42 review surfaces -----------------------------------------------------
# Each builder reads the plan tree directly and produces escaped HTML for its
# template section. Every builder is self-contained: no cross-references.

# build_identity_panel <plan-dir>: description summary and per-goal outcomes.
build_identity_panel() {
    local pd="$1/plan-description.md" gdir out="" gname outcome_text
    out="<div class='id-desc'><h3>What this plan does</h3><p>$(esc "$(awk '/^## Current state/{f=1;next}/^## /&&f{exit}f' "$pd" | head -5 | tr '\n' ' ')")</p></div>"
    out="$out<div class='id-goals'><h3>Goals and outcomes</h3>"
    while IFS= read -r gdir; do
        [ -f "$gdir/goal.md" ] || continue
        gname="$(basename "$gdir")"
        outcome_text="$(awk '/^## Outcome and definition of done/{f=1;next}/^## /&&f{exit}f' "$gdir/goal.md" | head -3 | tr '\n' ' ')"
        [ -n "$outcome_text" ] || outcome_text="See goal.md"
        out="$out<div class='goal-outcome'><b>$(esc "$gname")</b>: $(esc "$outcome_text")</div>"
    done < <(find "$1" -mindepth 1 -maxdepth 1 -type d | sort)
    printf '%s' "$out</div>"
}

# build_step_details <plan-dir>: openable drill-down per step.
build_step_details() {
    local pd="$1" out="" sfile goal_name step_name instr crit
    while IFS= read -r sfile; do
        goal_name="$(basename "$(dirname "$sfile")")"
        step_name="$(basename "$sfile" .md)"
        instr="$(sec "$sfile" 'Instructions' | head -8)"
        crit="$(sec "$sfile" 'Acceptance criteria' | head -6)"
        out="$out<details class=\"step-open\" id=\"step-$step_name'><summary>$(esc "$step_name")</summary><div class=\"sd-body\">"
        if [ -n "$instr" ]; then out="$out<div class=\"sd-instr\"><b>Instructions:</b><pre>$(esc "$instr")</pre></div>"; fi
        if [ -n "$crit" ]; then out="$out<div class=\"sd-crit\"><b>Acceptance criteria:</b><pre>$(esc "$crit")</pre></div>"; fi
        out="$out</div></details>"
    done < <(find "$pd" -mindepth 2 -path '*/steps/*.md' ! -name '*-testing.md' | sort)
    printf '%s' "$out"
}

# build_dep_graph <plan-dir>: inline SVG dependency graph from inventory edges.
build_dep_graph() {
    local inv="$1/work-unit-inventory.md" out="" nodes="" arrows=""
    local uid deps d x=30 y=30 nw=0
    # bash 3.2: no associative arrays; using string-indexed pseudo-map
    # bash 3.2 fallback: string-indexed pseudo-map
    local xy_keys="" xy_vals=""
    if [ -f "$inv" ]; then
        while IFS= read -r line; do
            case "$line" in '| W'*) ;; *) continue ;; esac
            local nf; nf=$(printf '%s' "$line" | awk -F'|' '{print NF}')
            [ "$nf" -ge 10 ] || continue
            uid="$(plan_table_cell "$line" 2)"
            case "$uid" in W[0-9]*) ;; *) continue ;; esac
            nw=$((nw + 1))
            x=$((30 + (nw % 5) * 90))
            y=$((30 + (nw / 5) * 55))
            nodes="$nodes<rect x='$x' y='$y' width='70' height='28' rx='4' class='dep-node'/><text x='$((x+35))' y='$((y+17))' text-anchor='middle' class='dep-label'>$uid</text>"
            xy_keys="$xy_keys|$uid|"
            xy_vals="$xy_vals|$((x+35)),$((y))|"
            deps="$(plan_table_cell "$line" 8)"
            case "$deps" in ''|"—") continue ;; esac
            local oldIFS="$IFS"; IFS=','
            for d in $deps; do
                IFS="$oldIFS"
                d="$(printf '%s' "$d" | sed 's/^ *//; s/ *$//')"
                case "$d" in W[0-9]*) ;; *) continue ;; esac
                arrows="$arrows<line class='dep-edge' data-from='$d' data-to='$uid'/>"
            done
            IFS="$oldIFS"
        done < "$inv"
    fi
    out="<svg viewBox='0 0 480 $((60 + (nw / 5) * 55))' class='dep-svg'>$nodes$arrows</svg>"
    printf '%s' "$out"
}

# build_tests_panel <plan-dir>: companion steps and their verification intent.
build_tests_panel() {
    local pd="$1" out="" sfile base intent
    while IFS= read -r sfile; do
        base="$(basename "$sfile" .md)"
        intent="$(sed -n 's/^§ 2.1 //p' "$sfile" | head -1)"
        [ -n "$intent" ] || intent="(see companion for details)"
        out="$out<div class='test-row'><span class='tn'>$(esc "${base%-testing}")</span><span class='ti'>$(esc "$intent")</span></div>"
    done < <(find "$pd" -mindepth 2 -name '*-testing.md' | sort)
    printf '%s' "$out"
}

# build_coverage_panel <inv>: coverage rows as a table.
build_coverage_panel() {
    local inv="$1" out="" row c_out c_units
    [ -f "$inv" ] || { printf '<p>No coverage table found.</p>'; return; }
    in_cov=0
    while IFS= read -r line; do
        case "$line" in '## Definition-of-done coverage') in_cov=1; continue ;; esac
        case "$line" in '## '*) in_cov=0; continue ;; esac
        [ "$in_cov" = 1 ] || continue
        case "$line" in '| '*) ;; *) continue ;; esac
        case "$line" in '|---'*|'| Required outcome'*) continue ;; esac
        c_out="$(plan_table_cell "$line" 2)"
        c_units="$(plan_table_cell "$line" 3)"
        [ -n "$c_out" ] || continue
        out="$out<tr><td>$(esc "$c_out")</td><td>$(esc "$c_units")</td></tr>"
    done < "$inv"
    printf '<table class="cov-table"><tr><th>Outcome</th><th>Work units</th></tr>%s</table>' "$out"
}

# build_findings_panel <rev>: openable findings grouped by status.
build_findings_panel() {
    local rev="$1" out="" line fid item change status wu
    [ -f "$rev" ] || { printf '<p>No findings.</p>'; return; }
    while IFS= read -r line; do
        case "$line" in '| AR'*) ;; *) continue ;; esac
        fid="$(plan_table_cell "$line" 2)"
        item="$(plan_table_cell "$line" 3)"
        change="$(plan_table_cell "$line" 4)"
        status="$(plan_table_cell "$line" 5)"
        wu="$(plan_table_cell "$line" 6)"
        out="$out<details class='finding' id='$fid'><summary><span class='fs'>$status</span> <b>$fid</b> — $(esc "$item")</summary><div class='fd'><p><b>Change:</b> $(esc "$change")</p><p><b>Unit:</b> $wu</p></div></details>"
    done < <(awk '/^## Findings$/{f=1;next} /^## Verdict$/{f=0} f' "$rev")
    printf '%s' "$out"
}
ticker_plain="$(printf '%s %s %s %s' "$narr1" "$narr2" "$narr3" "$narr4" | sed -e 's/<[^>]*>//g')"
ticker="${ticker_plain}<b>◆</b>auto-reloads every ${refresh}s<b>◆</b>rendered by render-plan-overview.sh<b>◆</b>self-contained html"

generated="${OVERVIEW_NOW:-$(date -u '+%Y-%m-%dT%H:%MZ')}"
plan_name="$(sed -n 's/^# Plan: //p' "$plan_dir/plan-description.md" 2>/dev/null | head -1)"
[ -n "$plan_name" ] || plan_name="$(basename "$plan_dir")"
footer="$(esc "$(basename "$plan_dir") · generated $generated · single-file html, no external assets")"

# The substitution map: one JSON object carrying every panel's HTML.
# jq encodes multi-line values as native JSON strings, so nothing is lost
# across lines (the TSV+awk system dropped content after the first line).
subs_file="$(mktemp "${TMPDIR:-/tmp}/overview-subs.XXXXXX")"
out_tmp="$(mktemp "${TMPDIR:-/tmp}/overview-out.XXXXXX")"
trap 'rm -f "$subs_file" "$out_tmp"' EXIT

# Serve mode disables the refresh timer: the page polls /state.json instead.
effective_refresh="$refresh"
if [ "$serve" = true ]; then effective_refresh=0; fi

jq -n \
    --arg plan_name "$(esc "$plan_name")" \
    --arg state "$state" \
    --arg phase_line "$(esc "$phase_line")" \
    --arg refresh "$effective_refresh" \
    --arg generated "$generated" \
    --arg n_goals "$goals_count" \
    --arg n_steps "$total_steps" \
    --arg n_units "$n_units" \
    --arg f_total "$f_total" \
    --arg f_open "$f_open" \
    --arg f_resolved "$f_resolved" \
    --arg f_res_pct "$f_res_pct" \
    --arg cycles "$cycles" \
    --arg review_target "$review_target" \
    --arg review_depth "$review_depth" \
    --arg pct "$pct" \
    --arg donut_offset "$donut_offset" \
    --arg ring_work "$ring_work" \
    --arg ring_find "$ring_find" \
    --arg ring_review "$ring_review" \
    --arg goal_bars "$goal_bars" \
    --arg cycle_chart "$cycle_chart_svg" \
    --arg narration "$narration" \
    --arg ledger "$ledger" \
    --arg identity_panel "$(build_identity_panel "$plan_dir")" \
    --arg step_details "$(build_step_details "$plan_dir")" \
    --arg dep_graph "$(build_dep_graph "$plan_dir")" \
    --arg tests_panel "$(build_tests_panel "$plan_dir")" \
    --arg coverage_panel "$(build_coverage_panel "$plan_dir/work-unit-inventory.md")" \
    --arg findings_panel "$(build_findings_panel "$review_file")" \
    --arg ticker "$ticker" \
    --arg footer "$footer" \
    --arg meta_refresh_tag "$(if [ "$serve" = true ]; then printf ''; else printf '<meta http-equiv="refresh" content="%s">' "$effective_refresh"; fi)" \
    --arg is_serve "$([ "$serve" = true ] && echo true || echo false)" \
    '{
        PLAN_NAME: $plan_name, STATE: $state, PHASE_LINE: $phase_line,
        REFRESH: $refresh, REFRESH_JS: $refresh, GENERATED: $generated,
        N_GOALS: $n_goals, N_STEPS: $n_steps, N_UNITS: $n_units,
        F_TOTAL: $f_total, F_OPEN: $f_open, F_RESOLVED: $f_resolved,
        F_RES_PCT: $f_res_pct, CYCLES: $cycles,
        REVIEW_TARGET: $review_target, REVIEW_DEPTH: $review_depth,
        PCT: $pct, CIRCUMFERENCE: "326.73", DONUT_OFFSET: $donut_offset,
        RING_WORK: $ring_work, RING_FIND: $ring_find, RING_REVIEW: $ring_review,
        GOAL_BARS: $goal_bars, CYCLE_CHART: $cycle_chart, NARRATION: $narration,
        STEP_LEDGER: $ledger, IDENTITY_PANEL: $identity_panel,
        STEP_DETAILS: $step_details, DEP_GRAPH: $dep_graph,
        TESTS_PANEL: $tests_panel, COVERAGE_PANEL: $coverage_panel,
        FINDINGS_PANEL: $findings_panel,
        META_REFRESH_TAG: $meta_refresh_tag,
        IS_SERVE: $is_serve,
        TICKER: $ticker, FOOTER: $footer
    }' > "$subs_file"

# Single jq call substitutes all tokens. Multi-line values are native JSON
# strings and survive intact — the old TSV+awk system dropped content after
# the first line of any multi-line value.
jq -rn --rawfile template "$template" --slurpfile s "$subs_file" '
    $template |
    reduce ($s[0] | to_entries[]) as $e (.; gsub("@_" + $e.key + "_@"; $e.value))
' > "$out_tmp"

# Unresolved tokens are a build failure.
if grep -q '@[A-Z_][A-Z_]*@' "$out_tmp"; then
    grep -o '@[A-Z_][A-Z_]*@' "$out_tmp" | sort -u | while IFS= read -r token; do
        printf '%s: unresolved template token %s\n' "${0##*/}" "$token" >&2
    done
    exit 70
fi

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
