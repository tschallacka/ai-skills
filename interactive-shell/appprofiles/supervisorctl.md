# supervisorctl

Cannot be tested in this environment -- Supervisor requires a running supervisord daemon, which is typically not available in sandboxes. Every fact below is from standard documented behavior [unconfirmed] throughout.

### Identity

Interactive REPL for managing Supervisor-controlled processes. `supervisorctl`, `supervisorctl -c CONFIG`, etc.

### Layout

- Prompt -> `supervisor>` (or similar)
- Command input -> accepts supervisorctl commands
- Output -> status information or command results

### Keys

| Key | Action | Notes |
|-----|--------|-------|
| ENTER | Execute current command | Sends command to supervisord |
| CTRL-D | Exit supervisorctl | Closes connection to supervisord |
| UP / DOWN | Navigate command history | If readline support is compiled in |
| ? | Show available commands | Lists valid commands |

### Workflows

1. **View process status**:
   - Run: `supervisorctl`
   - Type: `status` or `status <processname>`
   - ENTER

2. **Start/stop/restart a process**:
   - Type: `start <processname>`, `stop <processname>`, or `restart <processname>`
   - ENTER

3. **Tail process output**:
   - Type: `tail <processname>` or `tail -f <processname>`
   - ENTER

4. **Exit**:
   - Type: `quit` or `exit` or press CTRL-D

### Quirks

- Supervisor must be running and accessible (typically on localhost:9001)
- Commands are synchronous; no confirmation prompts for dangerous operations
- Process control requires supervisord to be running and the process to be defined in supervisor.conf
- `reread` and `update` commands reload supervisor configuration without restarting supervisord

### Unconfirmed

- Exact command set and syntax [unconfirmed]
- Behavior with authentication and remote connections [unconfirmed]
- Error handling and failure modes [unconfirmed]
