# expect

Tcl automation tool for driving interactive programs; also has its own interactive REPL mode. `expect`, `expect scriptfile.exp`, etc.

### Identity

Tcl/expect tool for automating interactive CLI programs. When run without a script, enters an interactive Tcl command prompt (conceptually similar to what `interactive-shell` does for other programs).

### Modes

#### Interactive REPL mode (no script argument)
- **Prompt**: `expect1.1>` (version number may vary)
- **Language**: Tcl commands (not shell)
- **Usage**: Commands like `puts hello`, `set var value`, `spawn COMMAND`, etc.
- **Exit**: `exit` or CTRL-D

#### Script execution mode (with .exp file)
- **Invocation**: `expect myscript.exp`
- **Script content**: Tcl with expect-specific keywords (`spawn`, `expect`, `send`, etc.)
- **Non-interactive**: Script runs to completion or until error; no user interaction
- **Exit code**: Reflects script success/failure

### Keys (REPL mode only)

| Key | Action | Notes |
|-----|--------|-------|
| ENTER | Execute current line | Executes the Tcl command |
| CTRL-D | Exit REPL | Closes the expect session |
| CTRL-C | Interrupt current command | May abort long-running operations |
| UP / DOWN | Navigate history | If readline support is compiled in |

### Workflows (REPL mode)

1. **Run a simple command**:
   - Type: `puts hello`
   - ENTER
   - Output: `hello`

2. **Spawn and interact with a subprocess**:
   - Type: `spawn bash`
   - ENTER
   - Interact with the spawned shell through expect commands

3. **Exit REPL**:
   - Type: `exit`
   - ENTER or CTRL-D

### Workflows (Script mode)

Script execution is non-interactive; typical script pattern:
```tcl
#!/usr/bin/expect
spawn ssh user@host
expect "password:"
send "mypassword\r"
interact
```

### Quirks

- Interactive REPL is a distinct mode from script execution; only the REPL accepts user commands
- Tcl syntax can be unfamiliar to shell-only users (different quoting, variable expansion rules)
- `spawn` command starts a subprocess but does not interact with it automatically; `expect` and `send` are required
- REPL history and readline support depend on compilation flags
- Expect scripts often have timing issues if subprocess output is delayed or buffered differently

### Unconfirmed

- Exact behavior of interactive REPL with complex Tcl expressions [unconfirmed]
- Readline history persistence [unconfirmed]
- Full expect scripting features and edge cases [unconfirmed]
