#!/usr/bin/env bash
# MODE: PROD
# render-plans-board.sh — render every plan in a plans root as one board page.
#
# The per-plan overview answers "where does this plan stand". This answers the
# question asked before that one: "I have twenty plans — which are moving, which
# are waiting on a review, which are done, and which of these directories are not
# plans at all". One card per plan, grouped by lifecycle, each linking into that
# plan's own overview page.
#
# It reads the plan tree directly rather than through overview-state.sh, which
# costs 8-9 seconds per plan. It passes no page content through a command line,
# so no plan size can make it fail the way the per-plan renderer does.
#
# Measured in Chrome, and recorded here because it is a finding about the design
# rather than a defect in it: this page reads BETTER at 390px than at 1440px.
# Narrow is one column, every card aligned, no dead gutters. Wide is where the
# compromises live — a card grid leaves an empty gutter beside a group of one,
# and cards in a row only line up because the title reserves three lines. Anyone
# laying out a plan surface for a wide viewport should know that the wide case
# is the harder one here, not the default that happens to work.
#
# Usage:
#   render-plans-board.sh [--root <plans-root>] [--out FILE] [--refresh N]
#   render-plans-board.sh --help
#
# Exit codes: 64 bad invocation, 66 plans root missing, 70 could not write.

set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/plan-document-lib.sh
source "$script_dir/plan-document-lib.sh"
# shellcheck source=planning/scripts/plans-board-lib.sh
source "$script_dir/plans-board-lib.sh"

export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--root <plans-root>] [--out FILE] [--refresh N]
       ${0##*/} --help

Renders one board page covering every plan under the plans root: lifecycle,
step progress, findings, review status and last activity, with a link into
each plan's own overview page.

  --root DIR     plans root to scan (default: \$PLANS_ROOT, else ~/.plans)
  --out FILE     write here (default: <plans-root>/board.html)
  --refresh N    page auto-reload seconds, 0 to disable (default 0)

BOARD_NOW=<utc-timestamp> pins the embedded timestamp for deterministic
output (tests).
USAGE
    exit "$rc"
}

# out_file, not out: the plan-dir hoisting helper in the sourced core library
# builds its result in `local out=()`, so a scalar named `out` here is typed as
# an array across the source boundary (SC2178, then SC2128 at every expansion).
# That helper's copy is local and no clobber can actually happen, but sharing a
# name with a global in a file we source is a collision waiting for the day it
# stops being local.
#
# The helper is described rather than named on purpose: test-flag-coverage.sh
# decides a script accepts --plan-dir by grepping the file for that function's
# name, so a comment mentioning it makes the gate demand a flag this script does
# not take. A board scans a plans ROOT; it has no single plan directory.
root="" out_file="" refresh=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --root) [ "$#" -ge 2 ] || usage; root="$2"; shift 2 ;;
        --out) [ "$#" -ge 2 ] || usage; out_file="$2"; shift 2 ;;
        --refresh) [ "$#" -ge 2 ] || usage; refresh="$2"; shift 2 ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *) printf '%s: unexpected argument: %s\n' "${0##*/}" "$1" >&2; usage ;;
    esac
done
case "$refresh" in ''|*[!0-9]*) usage ;; esac
[ -n "$root" ] || root="$(plan_default_root)"
root="${root%/}"
plan_require_directory "$root"
[ -n "$out_file" ] || out_file="$root/board.html"

# esc TEXT — HTML-escape the punctuation that survives plan titles and paths.
esc() {
    printf '%s' "$1" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g' -e 's/"/\&quot;/g'
}

# pct DONE TOTAL — integer percentage, rounded, 0 when the total is 0.
pct() {
    [ "${2:-0}" -gt 0 ] || { printf '0\n'; return 0; }
    printf '%s\n' "$(( ($1 * 100 + $2 / 2) / $2 ))"
}

# ─────────────────────────────────────────────────────────────────────────────
# Measure every plan once into a TSV, so the page can be laid out in any order
# without re-reading the tree. Fields, in order:
#   dir  title  lifecycle  review  done  skipped  wip  total  open  resolved
#   cycles  units  goals  age  has_overview
# ─────────────────────────────────────────────────────────────────────────────

measure_plan() { # <plan-dir>
    local plan="$1" title review counts done_n skip_n wip_n all_n
    local finds open_n res_n cycles units goals age overview
    title="$(sed -n 's/^# Plan: //p' "$plan/plan-description.md" | head -1)"
    [ -n "$title" ] || title="$(basename "$plan")"
    review="$(board_review_status "$plan")"
    counts="$(board_step_counts "$plan")"
    done_n="${counts%% *}"; counts="${counts#* }"
    skip_n="${counts%% *}"; counts="${counts#* }"
    wip_n="${counts%% *}"; all_n="${counts#* }"
    finds="$(board_finding_counts "$plan")"
    open_n="${finds%% *}"; res_n="${finds#* }"
    cycles="$(plan_cycle_count "$plan/adversarial-review-history.md")"
    units=0
    [ -f "$plan/work-unit-inventory.md" ] && units="$(plan_count_units "$plan/work-unit-inventory.md")"
    goals="$(find "$plan" -mindepth 2 -maxdepth 2 -name goal.md -print 2>/dev/null | wc -l | tr -d ' ')"
    age="$(board_last_activity "$plan")"
    overview=no
    [ -f "$plan/overview.html" ] && overview=yes
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$plan" "$title" \
        "$(board_lifecycle "$review" "$done_n" "$skip_n" "$all_n")" "$review" \
        "$done_n" "$skip_n" "$wip_n" "$all_n" "$open_n" "$res_n" \
        "$cycles" "$units" "$goals" "$age" "$overview"
}

rows_file="$(mktemp "${TMPDIR:-/tmp}/plans-board-rows.XXXXXX")"
out_tmp="$(mktemp "${TMPDIR:-/tmp}/plans-board-out.XXXXXX")"
trap 'rm -f "$rows_file" "$out_tmp"' EXIT
n_plans=0
while IFS= read -r plan; do
    [ -n "$plan" ] || continue
    measure_plan "$plan" >> "$rows_file"
    n_plans=$((n_plans + 1))
done < <(board_find_plans "$root")

# ─────────────────────────────────────────────────────────────────────────────
# Page emission — printed straight to the output file in one pass. No token
# template and no per-key rebuild of the document: every value is written where
# it belongs, once, as it is computed.
# ─────────────────────────────────────────────────────────────────────────────

emit_card() { # <tsv-row>
    local dir title life review done_n skip_n wip_n all_n open_n res_n
    local cycles units goals age overview p rel
    IFS="$(printf '\t')" read -r dir title life review done_n skip_n wip_n all_n \
        open_n res_n cycles units goals age overview <<TSV
$1
TSV
    p="$(pct "$((done_n + skip_n))" "$all_n")"
    rel="${dir#"$root"/}"
    printf '<article class="card" data-life="%s">' "$life"
    printf '<div class="card-top"><span class="chip">%s</span><span class="age">%s</span></div>' \
        "$life" "$(esc "$(board_ago_label "$age")")"
    printf '<h3 title="%s">%s</h3>' "$(esc "$title")" "$(esc "$title")"
    printf '<p class="path mono">%s</p>' "$(esc "$rel")"
    printf '<div class="track" role="img" aria-label="%s%% of %s steps done or skipped">' "$p" "$all_n"
    printf '<i class="seg-done" style="width:%s%%"></i>' "$(pct "$done_n" "$all_n")"
    printf '<i class="seg-skip" style="width:%s%%"></i>' "$(pct "$skip_n" "$all_n")"
    printf '<i class="seg-wip" style="width:%s%%"></i></div>' "$(pct "$wip_n" "$all_n")"
    emit_card_figures "$done_n" "$skip_n" "$all_n" "$open_n" "$res_n" \
        "$cycles" "$units" "$goals" "$review" "$wip_n"
    if [ "$overview" = yes ]; then
        printf '<a class="go" href="%s/overview.html">open the plan overview &rarr;</a>' "$(esc "$rel")"
    else
        printf '<span class="go none">no overview rendered yet</span>'
    fi
    printf '</article>\n'
}

emit_card_figures() { # <done> <skip> <total> <open> <res> <cycles> <units> <goals> <review> <wip>
    printf '<dl class="figs">'
    if [ "$3" -le 0 ]; then
        printf '<div><dt>steps</dt><dd class="dim">none yet</dd></div>'
    elif [ "${10}" -gt 0 ]; then
        # The bar shows an amber band for work in progress, so the figure has to
        # say so too: "0 of 44" beside a bar with something on it reads as a bug.
        printf '<div><dt>steps</dt><dd>%s of %s<span class="sub">, %s in progress</span></dd></div>' \
            "$1" "$3" "${10}"
    else
        printf '<div><dt>steps</dt><dd>%s of %s</dd></div>' "$1" "$3"
    fi
    [ "$2" -gt 0 ] && printf '<div><dt>skipped</dt><dd>%s</dd></div>' "$2"
    printf '<div><dt>goals</dt><dd>%s</dd></div>' "$8"
    printf '<div><dt>units</dt><dd>%s</dd></div>' "$7"
    printf '<div><dt>review</dt><dd>%s</dd></div>' "$(esc "$9")"
    printf '<div><dt>cycles</dt><dd>%s</dd></div>' "$6"
    if [ "$4" -gt 0 ]; then
        printf '<div><dt>findings</dt><dd class="bad">%s open</dd></div>' "$4"
    elif [ "$5" -gt 0 ]; then
        printf '<div><dt>findings</dt><dd class="good">all %s resolved</dd></div>' "$5"
    else
        printf '<div><dt>findings</dt><dd class="dim">none raised</dd></div>'
    fi
    printf '</dl>'
}

emit_group() { # <lifecycle> <heading> <blurb>
    local life="$1" count
    count="$(awk -F'\t' -v l="$life" '$3 == l { n++ } END { print n + 0 }' "$rows_file")"
    [ "$count" -gt 0 ] || return 0
    local sparse=no
    [ "$count" -le 2 ] && sparse=yes
    printf '<section class="group" data-life="%s" data-sparse="%s">' "$life" "$sparse"
    printf '<h2>%s <span class="n">%s</span></h2><p class="blurb">%s</p>' "$2" "$count" "$3"
    printf '<div class="grid">'
    while IFS= read -r row; do
        [ -n "$row" ] || continue
        emit_card "$row"
    done < <(awk -F'\t' -v l="$life" '$3 == l' "$rows_file")
    printf '</div></section>\n'
}

# emit_strays — the directories under the root that hold no plan.
#
# Named rather than dropped: fourteen of the twenty-five directories in a real
# plans root are an older flat format with no plan-description.md, and a board
# that shows eleven cards while claiming to show the root would be wrong in a way
# a reader cannot detect. They are listed apart from the plans because "0 of 0
# steps, no review" reads identically to a fresh plan, and they are not that.
emit_strays() {
    local dir n=0 list=""
    while IFS= read -r dir; do
        [ -n "$dir" ] || continue
        n=$((n + 1))
        list="$list<li class=\"mono\">$(esc "${dir#"$root"/}")</li>"
    done < <(board_find_strays "$root")
    [ "$n" -gt 0 ] || return 0
    printf '<section class="group strays"><h2>Not plans <span class="n">%s</span></h2>' "$n"
    printf '<p class="blurb">Directories under this root with no plan-description.md.'
    printf ' They are an earlier planning format or leftovers; this board can read no'
    printf ' progress from them and does not guess at any.</p><ul class="stray-list">%s</ul></section>\n' "$list"
}

emit_head() {
    printf '<!DOCTYPE html>\n<html lang="en">\n<head>\n<meta charset="utf-8">\n'
    printf '<meta name="viewport" content="width=device-width, initial-scale=1">\n'
    [ "$refresh" -gt 0 ] && printf '<meta http-equiv="refresh" content="%s">\n' "$refresh"
    printf '<title>Plans board — %s</title>\n<style>\n' "$(esc "$(basename "$root")")"
    emit_css
    printf '</style>\n</head>\n<body>\n<div class="wrap">\n'
}

emit_header_panel() { # <generated>
    local implementing planning_n complete
    implementing="$(awk -F'\t' '$3 == "implementing" { n++ } END { print n + 0 }' "$rows_file")"
    planning_n="$(awk -F'\t' '$3 == "planning" { n++ } END { print n + 0 }' "$rows_file")"
    complete="$(awk -F'\t' '$3 == "complete" { n++ } END { print n + 0 }' "$rows_file")"
    printf '<header><h1>Plans board</h1>'
    printf '<p class="root mono">%s</p>' "$(esc "$root")"
    printf '<div class="tallies">'
    printf '<div class="tally" data-life="implementing"><b>%s</b><span>implementing</span></div>' "$implementing"
    printf '<div class="tally" data-life="planning"><b>%s</b><span>in planning</span></div>' "$planning_n"
    printf '<div class="tally" data-life="complete"><b>%s</b><span>complete</span></div>' "$complete"
    printf '<div class="tally"><b>%s</b><span>plans in all</span></div>' "$n_plans"
    printf '</div><p class="stamp">generated %s</p></header>\n' "$(esc "$1")"
}

emit_css() {
    cat <<'CSS'
:root{
  --bg:#07080f; --bg2:#0b0d1a; --panel:rgba(255,255,255,.045); --line:rgba(255,255,255,.09);
  --txt:#e8ecff; --dim:#8b93b8; --faint:#565e85;
  /* The planning accent is the lighter violet, not #7c6cff: measured at 10px
     against the chip's #0b0d1a text, #7c6cff gave 5.01:1 while the other three
     chips sat above 10:1, and the odd one out read as muddy rather than as a
     different state. */
  --accent:#a78bfa; --good:#34d399; --warn:#fbbf24; --bad:#fb7185; --cyan:#22d3ee;
}
*{box-sizing:border-box;margin:0;padding:0}
body{font-family:ui-sans-serif,-apple-system,"Segoe UI",Roboto,Helvetica,Arial,sans-serif;
  color:var(--txt);min-height:100vh;
  background:radial-gradient(1200px 600px at 80% -10%,rgba(124,108,255,.16),transparent 60%),
             radial-gradient(900px 500px at -10% 110%,rgba(34,211,238,.12),transparent 60%),
             linear-gradient(160deg,var(--bg),var(--bg2));background-attachment:fixed}
.wrap{max-width:1240px;margin:0 auto;padding:30px 22px 80px}
.mono{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
header{border:1px solid var(--line);border-radius:18px;padding:26px 28px;margin-bottom:26px;
  background:linear-gradient(120deg,rgba(255,255,255,.06),rgba(255,255,255,.02))}
h1{font-size:clamp(22px,3vw,32px);letter-spacing:.4px}
.root{color:var(--dim);font-size:12px;margin-top:6px;word-break:break-all}
.stamp{color:var(--faint);font-size:11px;margin-top:14px}
.tallies{display:flex;gap:26px;flex-wrap:wrap;margin-top:20px}
.tally b{display:block;font-size:30px;line-height:1;font-variant-numeric:tabular-nums}
.tally span{font-size:11px;letter-spacing:1.6px;text-transform:uppercase;color:var(--dim)}
.tally[data-life="implementing"] b{color:var(--cyan)}
.tally[data-life="planning"] b{color:var(--accent)}
.tally[data-life="complete"] b{color:var(--good)}
.group{margin-bottom:34px}
.group h2{font-size:15px;letter-spacing:1.6px;text-transform:uppercase;color:var(--dim);
  display:flex;align-items:center;gap:10px}
.group h2 .n{font-size:11px;color:var(--faint);border:1px solid var(--line);
  border-radius:999px;padding:2px 9px}
.blurb{color:var(--faint);font-size:12px;margin:6px 0 16px;max-width:70ch}
.grid{display:grid;gap:16px;grid-template-columns:repeat(auto-fill,minmax(300px,1fr))}
/* A group of one or two stretches instead of leaving most of the row black:
   auto-fill reserves empty columns, which reads as an unfinished section. */
.group[data-sparse="yes"] .grid{grid-template-columns:repeat(auto-fit,minmax(320px,1fr));max-width:760px}
.card{border:1px solid var(--line);border-radius:14px;padding:18px 18px 16px;
  background:var(--panel);display:flex;flex-direction:column;gap:10px;min-width:0}
.card-top{display:flex;justify-content:space-between;align-items:center;gap:10px}
.chip{font-size:10px;letter-spacing:1.4px;text-transform:uppercase;font-weight:700;
  padding:4px 10px;border-radius:999px;color:#0b0d1a;background:var(--accent)}
.card[data-life="implementing"] .chip{background:var(--cyan)}
.card[data-life="complete"] .chip{background:var(--good)}
.card[data-life="awaiting-work"] .chip{background:var(--warn)}
.age{font-size:10px;color:var(--faint);letter-spacing:.6px}
.card h3{font-size:15px;line-height:1.35;font-weight:600;overflow-wrap:anywhere}
/* Three lines are reserved so the bar and the figures line up across a row.
   Reserved, not clamped: a longer title still grows and stays fully readable —
   hiding text to win an alignment would be the defect, not the fix. Scoped to
   the width where cards actually sit side by side: below it there is one card
   per row, nothing to align, and the reservation is pure dead space under
   every short title. */
@media (min-width:640px){ .card h3{min-height:4.05em} }
.path{font-size:10.5px;color:var(--faint);overflow-wrap:anywhere}
/* .07 alpha measured as a near-invisible hairline on this ground: an empty
   track read as a divider rather than as an empty progress bar. */
.track{display:flex;height:8px;border-radius:999px;overflow:hidden;
  background:rgba(255,255,255,.16);box-shadow:inset 0 0 0 1px rgba(255,255,255,.10)}
.track i{display:block;height:100%}
.seg-done{background:var(--good)} .seg-skip{background:var(--faint)} .seg-wip{background:var(--warn)}
.figs{display:grid;grid-template-columns:repeat(auto-fit,minmax(84px,1fr));gap:8px 12px}
.figs dt{font-size:9.5px;letter-spacing:1.2px;text-transform:uppercase;color:var(--faint)}
.figs dd{font-size:13px;font-variant-numeric:tabular-nums;overflow-wrap:anywhere}
.figs .good{color:var(--good)} .figs .bad{color:var(--bad)} .figs .dim{color:var(--faint)}
.figs .sub{color:var(--warn);font-size:11px}
/* margin-top:auto pushes the link and its divider to the card floor, so the
   footers of a row agree even when one card's steps figure wraps to two lines.
   It belongs in THIS rule: declared in a second .go block of equal specificity
   it lost to this one silently, and the divider stepped by 15px between
   neighbours — measured in the browser, invisible to every DOM assertion.

   If you go measuring this: getComputedStyle reports the USED value of `auto`,
   not the keyword, so a healthy row reads 15px on the cards whose steps figure
   wrapped and 0px on the rest. That is auto absorbing the difference, which is
   the rule working, NOT the override coming back. The numbers that prove
   alignment are the ones that must be EQUAL across a row: each link's top
   offset within its card, and the gap from its bottom to the card's bottom
   (17px on every card when this is right). Do not "fix" the 15px. */
.go{margin-top:auto;font-size:12px;color:var(--cyan);text-decoration:none;border-top:1px solid var(--line);
  padding-top:10px;display:block}
.go:hover{text-decoration:underline}
.go.none{color:var(--faint)}
.stray-list{list-style:none;display:grid;gap:5px;grid-template-columns:repeat(auto-fill,minmax(260px,1fr));
  font-size:11.5px;color:var(--dim)}
.empty{border:1px dashed var(--line);border-radius:14px;padding:26px;color:var(--dim);font-size:13px}
CSS
}

generated="${BOARD_NOW:-$(date -u '+%Y-%m-%dT%H:%MZ')}"

{
    emit_head
    emit_header_panel "$generated"
    if [ "$n_plans" -eq 0 ]; then
        printf '<p class="empty">No plan under %s holds a plan-description.md, so there is nothing to chart yet.</p>\n' \
            "$(esc "$root")"
    fi
    emit_group implementing 'Implementing' \
        'Review approved and steps are being worked. Progress is the question here.'
    emit_group awaiting-work 'Approved, nothing to execute' \
        'The review approved these, but they carry no steps at all. Nothing is in progress and nothing is late; the decomposition simply has not been written.'
    emit_group planning 'In planning' \
        'The adversarial review has not approved these yet, so the question is whether the plan is sound. Zero progress is the correct reading.'
    emit_group complete 'Complete' \
        'Approved, and every step is done or deliberately skipped.'
    emit_strays
    printf '<p class="stamp">%s plans read directly from the tree by render-plans-board.sh · single-file html, no external assets</p>\n' \
        "$n_plans"
    printf '</div>\n</body>\n</html>\n'
} > "$out_tmp"

if plan_atomic_write "$out_file" < "$out_tmp"; then
    printf 'Rendered %s\n' "$out_file"
else
    plan_die "could not write $out_file" 70
fi
