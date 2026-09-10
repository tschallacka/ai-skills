# MODE: PROD
# validate-plan-stale-wording.awk — T138/B110: find a claim a paragraph
# declares stale (quoted, beside a $stale_markers phrase) that survives
# verbatim in another, non-retraction paragraph of the same document.
#
# Invoked as: awk -v markers="$stale_markers" -f <this file> <plan-doc>
# Prints one line per finding: "<retraction paragraph>\t<survival
# paragraph>\t<claim, truncated>". CODE-STYLE.md caps an inline awk program at
# 15 lines before it must move to a file; this one is long because quote
# extraction is character-scanned rather than regex-captured -- POSIX awk (the
# floor this repository targets, alongside bash 3.2 / BSD userland) has no
# capturing match(). See validate-plan-coherence-lib.sh for the paragraph
# buffering this shares with validate-plan-stale-lib.sh's stale_scan_doc, and
# for why a quote is claim-worthy only past 8 characters, and why a bare
# apostrophe must not be read as an opening quote.
function flush() {
    if (label != "" && flat != "") {
        paragraph++
        para_text[paragraph] = flat
        para_is_retraction[paragraph] = (tolower(flat) ~ markers) ? 1 : 0
    }
    flat = ""
}
function extract_claims(text, p,    i, n, c, prev, nxt, start, j, k, pos, found, claim) {
    n = length(text)
    i = 1
    while (i <= n) {
        c = substr(text, i, 1)
        if (c == "\"") {
            start = i + 1
            j = index(substr(text, start), "\"")
            if (j > 0) {
                claim = substr(text, start, j - 1)
                if (length(claim) >= 8) {
                    nclaims++
                    claims[nclaims] = claim
                    claim_para[nclaims] = p
                }
                i = start + j
                continue
            }
        } else if (c == "'") {
            prev = (i > 1) ? substr(text, i - 1, 1) : ""
            if (prev !~ /[A-Za-z]/) {
                start = i + 1
                j = start
                found = 0
                while (j <= n) {
                    k = index(substr(text, j), "'")
                    if (k == 0) break
                    pos = j + k - 1
                    nxt = (pos < n) ? substr(text, pos + 1, 1) : ""
                    if (nxt !~ /[A-Za-z]/) { found = pos; break }
                    j = pos + 1
                }
                if (found > 0) {
                    claim = substr(text, start, found - start)
                    if (length(claim) >= 8) {
                        nclaims++
                        claims[nclaims] = claim
                        claim_para[nclaims] = p
                    }
                    i = found + 1
                    continue
                }
            }
        }
        i++
    }
}
/^#+ / { flush(); label = $0; next }
/^[[:space:]]*$/ { flush(); next }
{ if (label != "") { flat = (flat == "" ? $0 : flat " " $0) } }
END {
    flush()
    for (p = 1; p <= paragraph; p++) {
        if (para_is_retraction[p]) extract_claims(para_text[p], p)
    }
    for (ci = 1; ci <= nclaims; ci++) {
        for (p = 1; p <= paragraph; p++) {
            if (p == claim_para[ci]) continue
            if (para_is_retraction[p]) continue
            if (index(para_text[p], claims[ci]) > 0) {
                snippet = claims[ci]
                if (length(snippet) > 80) snippet = substr(snippet, 1, 77) "..."
                printf "%d\t%d\t%s\n", claim_para[ci], p, snippet
            }
        }
    }
}
