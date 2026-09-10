# iotop

Not tested this session -- typically requires root or CAP_SYS_ADMIN for kernel I/O accounting. Sandbox does not permit privileged access. Profile covers standard behavior. [unconfirmed] throughout.

### Identity
Per-process I/O monitor. `iotop` (interactive, requires root/privilege), `iotop -o` (show only processes with I/O activity), `iotop -a` (show total I/O since startup [unconfirmed]).

### Layout
- Header -> load average, total I/O rate (disk reads/writes, throughput)
- Column headers -> TID (thread ID), PRIO (priority), USER, DISK READ, DISK WRITE, SWAPIN, IO (or similar I/O metrics)
- Process rows -> per-process I/O stats, sorted by I/O (top consumers first)
- Bottom -> command hint line

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| o | Toggle active-only | Show only processes with I/O activity; toggle to see all |
| a | Accumulated | [unconfirmed] Toggle between current and accumulated I/O |
| p | Priority sort | [unconfirmed] Sort by priority instead of I/O |
| q / CTRL-C | Quit | Exit iotop |

### Modes
- **Live mode** (default): Shows current I/O rate and refreshes continuously.
- **Accumulated mode** (`a`): Shows total I/O since iotop started [unconfirmed].
- **Active-only mode** (`o`): Filters to only processes doing I/O right now.

### Workflows
1. Monitor I/O: `sudo iotop`, view process I/O stats, press `o` to filter to active processes, `q` to quit.
2. Watch one process: `sudo iotop -o` (starts showing only active), watch that process's I/O, `q` to exit.

### Quirks
- iotop requires root or CAP_SYS_ADMIN to access kernel I/O accounting.
- Without privilege, it will refuse to run or show no data.
- Similar interface to top but for I/O instead of CPU.
- TID (thread ID) may differ from PID if a process has multiple threads [unconfirmed].

### Unconfirmed
- Exact column names and what each metric means (DISK READ/WRITE units, SWAPIN definition)
- Whether `a` (accumulated) mode exists and how it differs from live
- Whether `p` sorts by priority or some other attribute
- How many process rows are displayed before truncation/scrolling
- Whether arrow keys or mouse clicks navigate
