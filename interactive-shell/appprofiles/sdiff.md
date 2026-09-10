# sdiff

### Identity
Side-by-side file diff, with optional interactive merge. `sdiff file1 file2` for view-only; `sdiff -o outfile file1 file2` for interactive merge mode. Non-interactive mode prints to stdout and exits; merge mode prompts for each conflicting region.

### Layout (non-interactive mode)
- Columns: left file, center operator (`|` for changed, `<` for left-only, `>` for right-only), right file
- One conflict/line pair per line
- No prompt or pager

### Layout (interactive merge mode)
- Rows -> conflicting line pairs, side-by-side
- Bottom row -> `%` prompt waiting for merge decision
- No file shown yet; decisions are made sequentially per difference

### Keys (interactive merge)
| Key | Action | Notes |
|-----|--------|-------|
| l | Take left version | Line from file1; advances to next difference |
| r | Take right version | Line from file2; advances to next difference |
| e | Edit a custom version | Prompts for text, used in merged output |
| d | Delete line | Neither left nor right; skips the difference |
| s | Silent mode | [unconfirmed] Suppress confirmation output |
| v | Verbose mode | [unconfirmed] Show full context around each difference |
| q | Quit | Exit merge (no output written if quit before all conflicts resolved) |
| RETURN | Accept default | [unconfirmed] Takes left by default [unconfirmed] |

### Workflows
1. View side-by-side diff (non-interactive): `sdiff file1 file2`, output shown immediately, exit automatically.
2. Merge files (interactive): `sdiff -o merged.txt file1 file2`, prompt for each conflict: `l` to take left, `r` to take right, `q` when done. Result written to merged.txt.

### Quirks
- Non-interactive mode is a stream output, not a pager; does not enter any interactive state.
- Merge mode prompts sequentially for each conflict block, not per line.
- The merge happens during the session; the output file is written only at the end.

### Unconfirmed
- Whether RETURN alone takes a default choice or re-prompts
- What `s` and `v` modes do in merge (silent vs. verbose)
- Whether quitting mid-merge still writes partial output or discards everything
- Whether `e` option to edit actually exists in all sdiff implementations
