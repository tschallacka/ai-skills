# MODE: PROD
# validate-plan-countable-enumeration.awk — T139/B110: flag "the following N
# <things>" with no explicit member (a WNN/BNN/TNN id or a file-path-like
# token) named in the same paragraph.
#
# Invoked as: awk -f <this file> <plan-doc>. Prints one line per finding:
# "<paragraph>\t<paragraph text, truncated>". See
# validate-plan-coherence-lib.sh for why this is scoped to "the following N
# X" and not the bug's original broader idea (a spelled-out count or
# collective phrase generally): measured against this repository's own
# plan-overview-rebuild corpus, the broader shape was 0% precise -- "all
# twelve goals" is a correct universal count and "these/those X" is ordinary
# anaphora, neither a same-paragraph promise left unkept -- while "the
# following N X" specifically had zero hits at all, positive or false.
BEGIN {
    trigger = "the following (one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|[0-9]+) (steps?|goals?|files?|units?|documents?|docs?|bugs?|todos?|items?|criteria|stories|findings?)"
    member = "(W[0-9][0-9]+)|([BT][0-9]+)|([A-Za-z0-9_-]+/[A-Za-z0-9_./-]+)|([A-Za-z0-9_-]+\\.[A-Za-z]+)"
}
function flush(    low, out) {
    if (label != "" && flat != "") {
        paragraph++
        low = tolower(flat)
        if (low ~ trigger && flat !~ member) {
            out = flat
            if (length(out) > 160) out = substr(out, 1, 157) "..."
            printf "%d\t%s\n", paragraph, out
        }
    }
    flat = ""
}
/^#+ / { flush(); label = $0; next }
/^[[:space:]]*$/ { flush(); next }
{ if (label != "") { flat = (flat == "" ? $0 : flat " " $0) } }
END { flush() }
