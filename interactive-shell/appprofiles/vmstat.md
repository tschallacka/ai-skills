# vmstat

### Identity
Virtual memory statistics reporter. `vmstat` (one-shot) or `vmstat INTERVAL` (repeating, e.g., `vmstat 2`).

### Layout
- Row 1 -> column labels
- Row 2 -> column labels (continued)
- Row 3+ -> statistics for each sample

Column groups: procs (r, b), memory (swpd, free, buff, cache), swap (si, so), io (bi, bo), system (in, cs), cpu (us, sy, id, wa, st, gu).

### Modes
Two modes: one-shot (prints one report and exits) and repeating (prints a report every INTERVAL seconds until ctrl+c).

### Keys
| Key | Action | Mode |
|-----|--------|------|
| ctrl+c | Stop repeating interval | Repeating interval mode |

### Workflows
1. One-shot report: `vmstat`. Observe memory, swap, I/O, and CPU statistics, then exit.
2. Repeating report: `vmstat 2`. Observe first report, wait 2 seconds, observe second report. Press ctrl+c to stop.

### Quirks
- One-shot mode does not display any prompt; output appears and the process exits immediately
- Repeating mode has no interactive keys except ctrl+c
- Reports stack on the screen; each interval appends a new line below the previous one
- First line in repeating mode is the average since system boot (not the interval sample); subsequent lines are interval samples

### Unconfirmed
- Exact meaning of each cpu column value (percentage vs. raw count) [unconfirmed]
- Behavior when swap is not configured [unconfirmed]
- Order of columns in a repeating report vs. one-shot report (if they differ) [unconfirmed]
