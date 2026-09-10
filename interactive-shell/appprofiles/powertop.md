# powertop

Not tested this session -- typically requires root or elevated privileges for full hardware power measurement; the sandbox does not permit this. Profile covers standard behavior only. [unconfirmed] throughout.

### Identity
Power consumption and optimization monitor. `powertop` (interactive mode, requires root), `powertop --calibrate` (hardware calibration), `powertop -r` or `-R` (report/HTML output).

### Layout (interactive mode)
- Header -> hostname, date/time, average power draw (in watts, if measurable)
- Tabbed sections:
  - **Overview**: Top power consumers, estimated battery life, wake sources
  - **Idle stats**: Per-core CPU idle state residency
  - **Frequency stats**: Per-core frequency distribution
  - **Device stats**: Power draw by device (GPU, disk, etc.) [unconfirmed]
  - **Tunables**: Recommended power-saving settings, toggle-able [unconfirmed]

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| TAB | Next tab | Cycle to next tab (Overview, Idle stats, etc.) |
| SHIFT-TAB | Previous tab | Cycle back |
| Arrow keys | Navigate items | [unconfirmed] Highlight tunable or setting |
| RETURN / SPACE | Toggle tunable | Apply/disable a power-saving setting [unconfirmed] |
| ESC / q | Quit | Exit powertop |

### Workflows
1. Interactive mode: `sudo powertop`, view tabs with TAB, inspect power stats, toggle recommendations with SPACE [unconfirmed], ESC to quit.
2. Calibration: `sudo powertop --calibrate` (runs tests, may take a minute [unconfirmed]).
3. Report: `sudo powertop -r` (generates report to stdout or file, non-interactive [unconfirmed]).

### Quirks
- powertop typically requires root (or CAP_SYS_ADMIN) to read hardware power data.
- Without privilege, it may start but show "no power data available" or refuse to run.
- Toggling Tunables may require saving changes or confirming application [unconfirmed].

### Unconfirmed
- Exact list of Device stats and what constitutes a "device" in powertop's output
- Whether arrow keys navigate within a tab or only TAB switches tabs
- Whether RETURN/SPACE toggle tunables immediately or open a dialog
- Exact output format and content of report mode
- Whether calibration is required before accurate readings
