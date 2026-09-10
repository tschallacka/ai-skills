# socat

Versatile relay tool for establishing bidirectional data connections. `socat READLINE TCP:HOST:PORT`, `socat STDIO TCP-LISTEN:PORT`, etc.

### Identity

Non-interactive tool for relaying data between two endpoints; becomes interactive when `READLINE` address is used.

### Layout

When using `READLINE` mode:
- Prompt -> a readline-editable input line (like a shell prompt)
- Input -> user-typed lines with history and editing
- Output -> echoed or relayed data from the remote side

### Keys (in READLINE mode)

| Key | Action | Notes |
|-----|--------|-------|
| ENTER | Send current line to the other connection end | Line is transmitted as data |
| UP / DOWN | Navigate command/line history | If compiled with readline support |
| CTRL-D | Close connection and exit socat | Sends EOF to the other side |
| CTRL-C | Abort socat | Terminates both sides immediately |
| CTRL-A / CTRL-E | Move cursor to start/end | Standard readline keybindings |
| BACKSPACE | Delete character before cursor | Standard line editing |

### Workflows

1. **Interactive relay with READLINE**:
   - Run: `socat READLINE TCP:127.0.0.1:9999`
   - Type commands or text, press ENTER
   - Input is transmitted; responses appear below

2. **Simple echo test**:
   - Terminal 1: `socat READLINE TCP-LISTEN:9999,fork` (background)
   - Terminal 2: `socat READLINE TCP:127.0.0.1:9999`
   - Type in Terminal 2; see echoed output

### Quirks

- `READLINE` mode is distinct from plain `STDIO`; the latter has no readline/history support
- `fork` option in LISTEN address allows multiple concurrent connections
- Data is relayed byte-for-byte; no protocol interpretation
- CTRL-D sends EOF but doesn't force closure; server must close its end
- Timeout behavior depends on `-t` (timeout) parameter; no timeout by default

### Unconfirmed

- Exact behavior of readline history persistence [unconfirmed]
- Interaction with non-text protocols (raw binary data) [unconfirmed]
- Detailed error messages for connection failures [unconfirmed]
