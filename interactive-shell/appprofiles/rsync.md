# rsync

File synchronization and transfer tool with live progress display. `rsync -av --progress SOURCE DEST`, etc.

### Identity

Non-interactive transfer/sync tool with streaming progress output (no interactive keys during transfer).

### Layout

Progress display shows:
- Source and destination paths
- Files being transferred, one per line
- Progress bar or byte-count per file
- Overall transfer speed and ETA
- Summary line with total bytes, rate, elapsed time

### Quirks

- rsync is entirely non-interactive during transfers; no keystroke feedback during sync
- CTRL-C aborts the transfer (sends SIGINT)
- Progress display refreshes in real-time on the same line(s); terminal may wrap at narrow widths
- Exit codes: 0 = success, 23 = partial transfer, 24 = aborted by signal
- `--progress` flag enables per-file progress; without it, only summary is shown
- No pause, skip, or resume prompts during transfer (unless using `--checksum` with specific sync options)

### Unconfirmed

- Exact format of progress display with different terminal widths [unconfirmed]
- Behavior of `--ignore-existing` and other checksum-related flags [unconfirmed]
- Remote rsync over SSH interactive prompts (typically delegated to SSH) [unconfirmed]
