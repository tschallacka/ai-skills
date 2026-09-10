# redis-cli

Interactive Redis command-line client. `redis-cli`, `redis-cli -h HOST -p PORT`, etc.

### Identity

Interactive REPL for executing Redis commands and managing data structures.

### Layout

- Prompt -> `127.0.0.1:6379>` (or configured host:port)
- Command input -> accepts Redis commands
- Output -> command results, from simple strings to complex data structures
- Status -> connection info shown at startup

### Keys

| Key | Action | Notes |
|-----|--------|-------|
| ENTER | Execute current command | Sends command to Redis server |
| CTRL-D | Exit redis-cli | Graceful disconnect; closes connection |
| CTRL-C | Interrupt current command | May timeout or abort depending on server state |
| UP / DOWN | Navigate command history | Recall previous commands |
| CTRL-A / CTRL-E | Move cursor to start/end of line | Standard readline keybindings |

### Workflows

1. **Basic command**: Type a command (e.g., `PING`), press ENTER, see result.
2. **Set and get a key**: `SET foo bar`, ENTER, then `GET foo`, ENTER.
3. **Check connection**: `PING` returns `PONG` if server is responding.
4. **Exit**: `exit`, `quit`, CTRL-D, or CTRL-C (then confirm if prompted).

### Quirks

- Prompt changes when inside a transaction (`>` becomes `MULTI` mode)
- RESP protocol output is rendered as text; complex types shown as nested structures
- Command names are case-insensitive (redis-cli accepts both PING and ping)
- History survives across invocations if compiled with readline support
- No auto-completion in all builds (depends on compilation flags)
- Connecting to an unreachable server shows error but still enters REPL (commands will fail)

### Unconfirmed

- Auto-completion availability and behavior [unconfirmed]
- Exact format of complex data-structure output [unconfirmed]
- Behavior with clusters and sentinel configurations [unconfirmed]
