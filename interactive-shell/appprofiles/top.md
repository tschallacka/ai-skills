# top

### Identity
Classic live process monitor. `top`. Refreshes on its own timer (default
3s), no input needed.

### Layout
- Row 1 -> uptime/load average
- Row 2 -> task counts by state
- Row 3 -> per-CPU usage breakdown
- Rows 4-5 -> memory and swap
- Row 6 -> typically blank
- Row 7 -> column header (`PID USER PR NI VIRT RES SHR S %CPU %MEM TIME+
  COMMAND` by default; configurable, may differ)
- Remaining rows -> one process per row, sorted by active sort key,
  refreshing automatically

### Modes
Not modal for reading -- table updates on its own. A few keys (renice,
kill, interval) open a one-line bottom prompt that consumes input until
answered or cancelled; the prompt names what it wants.

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| q | Quit | No confirmation |
| SPACE | Force immediate refresh | |
| P | Sort by %CPU | |
| M | Sort by %MEM | |
| k | Kill | Prompts for PID then signal |
| r | Renice | Prompts for PID then value |
| 1 | Toggle combined vs per-core CPU lines | Switches between single %Cpu line and per-core %Cpu0/1/2... lines |
| t | Toggle CPU display format | Switches between percentage breakdown and graph-style display |
| m | Toggle memory display format | Switches between rows and graph-style display |
| c | Toggle command line vs name | Shows full command line vs. binary name only |
| h | Help | |
| W | Write config | Saves current settings to ~/.toprc |
| f | Open field-selection menu | Choose visible columns |
| F | Exit field-selection menu | |

### Workflows
1. Find top CPU consumer: `P` (often already default), read first process
   row.
2. Quit: `q`.

### Quirks
- Screen updates on its own timer; a `view` shortly after the last one may
  show different numbers with no input sent -- not a missed keystroke.

### Unconfirmed
- Exact layout/navigation of the `f` field-selection menu through this
  wrapper
