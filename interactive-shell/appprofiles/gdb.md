# gdb

Not tested this session -- nix package fetch too slow. This profile covers two modes: (a) default CLI/REPL mode, standard debugger interface, and (b) TUI mode with on-screen source/assembly pane. Both are standard, stable documented behavior. [unconfirmed] throughout.

### Identity
GNU Debugger. `gdb PROGRAM` (load program for debugging), `gdb -tui PROGRAM` (start in TUI mode), or `gdb` with no arguments (start with no program loaded, attach later).

### Modes
- **CLI/REPL mode** (default): Command-line interface with `(gdb)` prompt. Each command is a line; output prints above.
- **TUI mode** (visual debugger): Split screen with source/assembly/register pane on top, command REPL at bottom. Entered with `gdb -tui` at startup or `layout src`/`layout asm`/`layout split` commands from CLI.

### Layout (CLI mode)
- Prompt line -> `(gdb)` waiting for command
- Scrollback above -> prior commands and their output

### Layout (TUI mode)
- Top pane -> source code / assembly / registers (depending on layout)
- Line highlight -> current instruction pointer location
- Bottom -> command REPL with `(gdb)` prompt

### Keys (CLI and TUI)
| Key | Action | Mode | Notes |
|-----|--------|------|-------|
| break LOCATION | Set breakpoint | CLI/TUI | e.g., `break main`, `break file.c:10` |
| run [ARGS] | Start program | CLI/TUI | Run with optional command-line args |
| continue / c | Resume execution | CLI/TUI | Continue until next breakpoint or end |
| next / n | Step over | CLI/TUI | Execute one line, skip function calls |
| step / s | Step into | CLI/TUI | Execute one line, enter function calls |
| print EXPR / p | Print value | CLI/TUI | Evaluate and print variable or expression |
| info break | List breakpoints | CLI/TUI | Show all active breakpoints |
| delete N | Remove breakpoint | CLI/TUI | Delete breakpoint number N |
| quit / q | Exit gdb | CLI/TUI | Exit debugger (may prompt if unsaved state [unconfirmed]) |

### Keys (TUI mode only)
| Key | Action | Notes |
|-----|--------|-------|
| CTRL-X a | Toggle TUI | Enter/exit TUI mode from CLI, or toggle layout from TUI |
| CTRL-L | Redraw | Refresh display if garbled |
| layout src | Source pane | Show source code in top pane |
| layout asm | Assembly pane | Show disassembled code in top pane |
| layout split | Split | Show source and assembly panes together (if space allows) |
| layout regs | Registers | Show register contents [unconfirmed] |
| CTRL-X 1 / CTRL-X 2 | Toggle panes | Hide/show lower or upper pane [unconfirmed] |

### Workflows
1. Debug a compiled program: `gdb ./myapp`, break main (set breakpoint), run (start), step (execute line by line), print variable (inspect value), continue (resume), quit (exit).
2. TUI mode: `gdb -tui ./myapp`, same commands, but with source visible on screen.
3. Inspect crash: `gdb ./myapp`, `core /path/to/core.dump` (load core file [unconfirmed]), `bt` (backtrace), print variables to inspect state at crash.

### Quirks
- gdb uses `break` for breakpoints (not `b` alone; `b` is an abbreviation for `break` [unconfirmed]).
- Information about variables, registers, memory, etc. is accessed via `print`, `info`, or `x` (examine memory [unconfirmed]).
- TUI mode redraws the source pane as you step, showing current line highlighted.
- Source pane requires debug symbols in the binary (`-g` flag during compilation).
- gdb persists state across multiple `run` commands; breakpoints stay set until deleted.

### Unconfirmed
- Exact list of TUI layouts and what each shows
- Whether `CTRL-X a` toggles TUI or enters TUI (or if entry is via `-tui` flag only)
- Whether `quit` requires confirmation, and if unsaved watchpoints/state matters
- Whether tab completion works for commands and symbols
- Behavior of `p` (print) vs. `pp` (pretty-print [unconfirmed])
