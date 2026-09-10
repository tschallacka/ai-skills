# atop

Not tested this session -- often requires elevated privilege for kernel accounting details. Profile covers standard behavior; details may be incomplete without privilege. [unconfirmed] throughout.

### Identity
Advanced process/system monitoring. `atop` (interactive), `atop -r` (replay recorded data [unconfirmed]), `atop -w` (record session data [unconfirmed]).

### Layout
- Header -> timestamp, hostname, load average, CPU/memory/disk/network summary lines
- Separator -> tab-like row showing which data category is active
- Process table -> columns similar to top/ps (PID, USER, CPU%, MEM%, I/O stats, etc.), one per process
- Bottom -> command hint line or status

### Modes
- **Generic mode** (default): Processes sorted by CPU usage [unconfirmed]
- **Memory mode** (`m`): Sort by memory usage
- **Disk I/O mode** (`d`): Sort by disk I/O
- **Network mode** (`n`): Show network per-process/per-interface stats [unconfirmed]
- **Other modes** (`c`, `v`, etc.): Cycle through different detail levels [unconfirmed]

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| t | Toggle mode | Cycle through modes (generic, memory, disk, network) [unconfirmed] |
| m | Memory mode | Sort/focus on memory usage |
| d | Disk mode | Sort/focus on disk I/O |
| n | Network mode | Sort/focus on network stats [unconfirmed] |
| c | CPU mode | [unconfirmed] |
| u | User filter | [unconfirmed] Filter by user |
| g | Generic mode | Return to default generic mode |
| s | Sleep/scroll | Control auto-refresh [unconfirmed] |
| q | Quit | Exit atop |

### Workflows
1. Interactive monitoring: `atop`, view process data, press `m` for memory view, `d` for disk view, `q` to quit.
2. Record data: `atop -w filename` (saves session for later playback [unconfirmed]).
3. Replay recording: `atop -r filename` (view recorded data [unconfirmed]).

### Quirks
- atop may require elevated privilege to show full I/O and network stats; unprivileged mode shows partial data.
- Unlike htop, atop is optimized for systems monitoring rather than process trees.
- Refresh rate and data granularity depend on the kernel accounting available [unconfirmed].

### Unconfirmed
- Exact mode names and key bindings (whether `t` cycles or each letter is a direct mode)
- What network mode actually displays
- Whether `-w` and `-r` flags work as described (record and replay)
- Performance impact and typical CPU/memory usage of atop
- Whether user filtering (`u`) opens a dialog or requires a subcommand
