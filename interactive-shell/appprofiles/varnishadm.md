# varnishadm

Cannot be tested in this environment -- Varnish requires privileged ports and a running daemon, neither of which are available in this sandbox. Every fact below is from standard documented behavior [unconfirmed] throughout.

### Identity

Interactive management console for Varnish HTTP cache. `varnishadm`, `varnishadm -T HOST:PORT`, etc.

### Layout

- Prompt -> `varnish>` (or similar, depending on version)
- Command input -> accepts varnish management commands
- Output -> command results or error messages

### Keys

| Key | Action | Notes |
|-----|--------|-------|
| ENTER | Execute current command | Sends command to varnishd daemon |
| CTRL-D | Exit varnishadm | Closes connection to daemon |
| UP / DOWN | Navigate command history | If readline support is compiled in |
| ? | Show available commands | Lists valid commands |

### Workflows

1. **List VCL files**: `vcl.list`, ENTER
2. **Show backend status**: `backend.list`, ENTER
3. **Show parameters**: `param.show`, ENTER
4. **Quit**: `quit` or `exit` or CTRL-D

### Quirks

- Varnish must be running (daemon accessible at configured host:port) or connection fails immediately
- Commands are synchronous; no background tasks or async feedback
- Some commands (e.g., `vcl.use`, `backend.set_health`) take effect immediately
- No interactive prompts for confirmation (dangerous operations execute directly)

### Unconfirmed

- Exact command set available in different Varnish versions [unconfirmed]
- Behavior of interactive editing within commands [unconfirmed]
- Detailed error message formats [unconfirmed]
