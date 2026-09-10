# iostat

### Identity
I/O statistics reporter. `iostat` (one-shot) or `iostat INTERVAL` (repeating, e.g., `iostat 2`).

### Layout
- Row 1 -> header with hostname, date, CPU count
- Row 2 -> column labels for avg-cpu section
- Row 3 -> CPU statistics (user, nice, system, iowait, steal, idle)
- Rows 4+ -> device statistics with columns: Device, tps, kB_read/s, kB_wrtn/s, kB_dscd/s, kB_read, kB_wrtn, kB_dscd

### Modes
Two modes: one-shot (prints one report and exits) and repeating (prints a report every INTERVAL seconds until ctrl+c).

### Keys
| Key | Action | Mode |
|-----|--------|------|
| ctrl+c | Stop repeating interval | Repeating interval mode |

### Workflows
1. One-shot report: `iostat`. Observe CPU and device statistics, then exit.
2. Repeating report: `iostat 2`. Observe first report, wait 2 seconds, observe second report. Press ctrl+c to stop. Each report is a full screen with no interactive prompt.

### Quirks
- One-shot mode does not display any prompt; output appears and the process exits immediately
- Repeating mode has no interactive keys except ctrl+c; no pause/resume, no menu, no help text
- Reports stack on the screen; each interval appends a new report below the previous one
- Column widths may truncate device names or statistics on narrow terminals

### Unconfirmed
- Exact number of columns in the device statistics section [unconfirmed]
- Behavior when no I/O devices are present [unconfirmed]
- Scrollback/history when running on a pager-wrapped terminal [unconfirmed]
