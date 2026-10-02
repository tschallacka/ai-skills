# nmon

Not tested this session -- availability varies; primarily available on AIX/Linux in enterprise environments. Standard behavior documented. [unconfirmed] throughout.

### Identity
Performance/capacity monitoring. `nmon` (interactive), `nmon -f` (starts in file recording mode [unconfirmed]), `nmon -x` (records to JSON [unconfirmed]). Displays CPU, memory, disk, network, thermal in separate toggleable panes.

### Layout
- Full screen divided into separate data categories, each a small pane or section
- Top -> summary/header info
- Sections toggle on/off with single-key commands (one pane visible or multiple arranged)
- Auto-refresh, no user input required for viewing (keys only for toggling display)

### Keys (toggles for display categories)
| Key | Action | Notes |
|-----|--------|-------|
| c | Toggle CPU pane | Show/hide per-core CPU stats |
| m | Toggle memory pane | Show/hide memory (RAM, swap, cache) |
| d | Toggle disk pane | Show/hide disk I/O per device |
| n | Toggle network pane | Show/hide network interface stats |
| t | Toggle thermal pane | Show/hide temperature sensors [unconfirmed] |
| h | Toggle help/header | Show/hide key reference [unconfirmed] |
| q | Quit | Exit nmon |
| ENTER | Scroll/more | Advance to next screen or scroll pane [unconfirmed] |

### Workflows
1. View all categories: `nmon`, all panes initially visible, press keys to toggle on/off (e.g., `c` to hide CPU, `m` to show just memory), `q` to quit.
2. Focus on one metric: `nmon`, press `c` to show only CPU, press `d` to also show disk, etc.
3. Record data: `nmon -f` (records to file for later analysis [unconfirmed]).

### Quirks
- Each key toggle is instant; no menus or sub-prompts.
- nmon is primarily available on Linux systems from IBM/Lenovo; not universal on all Linux distros.
- Multiple panes can be displayed simultaneously (up to screen size limits).
- No interactive process selection or filtering [unconfirmed]; displays system-wide stats only.

### Unconfirmed
- Exact set of toggleable panes and their keys
- Whether `-f` flag starts recording immediately or opens a dialog
- Scroll behavior within panes (ENTER, arrow keys, or auto-scroll)
- Whether `-x` JSON output exists in all nmon versions
- Colors/highlighting scheme and meaning in terminal mode
