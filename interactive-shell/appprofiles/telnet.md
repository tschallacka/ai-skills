# telnet

### Identity
Unencrypted remote-connection client. `telnet HOST [PORT]` connects
immediately; bare `telnet` (no arguments) opens telnet's own interactive
command prompt without connecting. Once connected, the remote side's own
program takes over the screen -- this profile covers the local
connection/command-prompt layer only.

### Modes
- **Command mode**: `telnet>` prompt, accepts telnet's own commands
  (`open`, `close`, `quit`, `status`, etc.). Entered on bare `telnet`
  startup, or from a live connection via the escape character (default
  CTRL-]).
- **Connected**: once a connection opens, all input goes to the remote
  side and the whole screen reflects its output.

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| CTRL-] | Escape to command mode from a live connection | Standard telnet escape character |

### Dialogs
- **Connection failure**: `Trying <host>...` then `telnet: Unable to
  connect to remote host: Connection refused` (or similar) and the process
  exits -- no prompt to answer.
- **Command-mode help** (`?` + ENTER): lists commands (`close`, `logout`,
  `display`, `mode`, `open`, `quit`, `send`, `set`, `unset`, `status`,
  `toggle`, `slc`, `auth`, `encrypt`, `z`, `!`, `environ`, `?`).

### Workflows
1. Connect from the command prompt: bare `telnet`, `open HOST [PORT]` +
   ENTER at the `telnet>` prompt; check the next screen for either the
   remote program's output or a connection-failure message.
2. Return to command mode mid-connection: CTRL-], then telnet commands
   (e.g. `close`, `quit`) work as at startup.
3. Quit: `quit` + ENTER at the `telnet>` prompt.

### Quirks
- A failed connection attempt (`telnet HOST PORT` with a closed/unreachable
  port) exits immediately with no prompt -- there is no command-mode
  fallback unless telnet was started bare in the first place.

### Unconfirmed
- Behavior of `set`/`toggle`/`slc` and other command-mode subcommands
- CTRL-] behavior while mid-connection (only tested from bare command-mode
  startup this session)
