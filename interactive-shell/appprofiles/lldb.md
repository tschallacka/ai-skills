# lldb

Not tested this session -- nix package fetch too slow. LLVM debugger, similar to gdb but with different command names and no built-in TUI mode. Standard behavior only. [unconfirmed] throughout.

### Identity
LLVM Debugger (`lldb`). `lldb PROGRAM` (load program), `lldb --attach PID` (attach to process). Interactive REPL with `(lldb)` prompt. No native TUI mode like gdb.

### Layout
- Prompt line -> `(lldb)` waiting for command
- Scrollback above -> prior commands and output

### Keys (main commands)
| Key | Action | Notes |
|-----|--------|-------|
| breakpoint set --name main / b main | Set breakpoint | `b` is abbrev for breakpoint; `--name` specifies function |
| run / r | Start program | Run with optional arguments [unconfirmed] |
| continue / c | Resume | Run until next breakpoint |
| next / n | Step over | Execute one line, skip function calls |
| step / s | Step into | Execute one line, enter function calls |
| print VAR / p | Print value | Evaluate and print variable |
| frame variable [NAME] / fr v | Show frame variables | List local variables in current stack frame |
| help | Show help | List available commands |
| quit / q | Exit | Exit lldb |

### Quirks
- lldb command names differ from gdb: `breakpoint set` vs `break`, `frame variable` vs `info locals`.
- lldb has no built-in TUI pane (unlike gdb's `-tui`); can only use CLI.
- lldb's `print` command may have different behavior/output format than gdb.
- Abbreviations exist but differ: `b` in lldb may refer to `breakpoint`, `n` to `next`, etc.

### Workflows
1. Debug a program: `lldb ./myapp`, `breakpoint set --name main`, `run`, `next`, `print variable`, `continue`, `quit`.
2. Attach to running process: `lldb --attach PID` (requires privilege [unconfirmed]), then same debugging commands.

### Unconfirmed
- Exact list of command abbreviations (`b`, `n`, `c`, `s`, etc.)
- Whether `print` and `p` both work or if one is preferred
- Support for core file debugging
- Whether Python/Lua scripting is supported
- Exact format of variable/stack frame output
