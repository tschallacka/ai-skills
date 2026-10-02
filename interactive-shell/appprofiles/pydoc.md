# pydoc

Python documentation viewer. `pydoc os`, `pydoc sys`, `python3` (then `help()` in the REPL), etc.

### Identity

Python documentation tool with nested REPL structure for interactive help browsing.

### Modes

#### Command-line invocation (non-interactive)
- `pydoc MODULE` pages module documentation through a pager (typically `less` or `more`)
- Output is read-only; follows pager keybindings (see less.md)
- Example: `pydoc os` displays the `os` module documentation

#### Interactive python3 REPL
- **Prompt**: `>>>`
- **Command**: `help()` enters interactive help sub-REPL
- **Sub-prompt**: `help>` (changes from `>>>` to `help>`)
- **Sub-REPL commands**: `help('os')`, `help('sys.path')`, etc.
- **Paged output**: Long output is piped through pager (see pager keybindings)
- **Exit pager**: `q` to return to `help>` prompt
- **Exit help sub-REPL**: `quit` or CTRL-D returns to `>>>` prompt
- **Exit python3 REPL**: `exit()`, `quit()`, or CTRL-D

### Layout

**Command-line**: Pager-style display (depends on configured pager)

**REPL**: Nested structure:
```
>>> help()  # Entry point to help sub-REPL
help> help('module')  # Inside help sub-REPL; may page output
(pager displays; q to exit pager)
help> quit  # Exit help sub-REPL, return to >>>
>>> exit()  # Exit python3 REPL
```

### Workflows

1. **View module documentation from command-line**:
   - Run: `pydoc os`
   - Follow pager keybindings (space/b/q/etc., see less.md)
   - Pager exits; you return to shell

2. **Interactive help from within Python**:
   - Run: `python3`
   - Type: `help()` (ENTER)
   - Prompt changes to `help>`
   - Type: `help('os')` or `help('os.path')` (ENTER)
   - If output is long, pager appears; type `q` to close pager and return to `help>` prompt
   - Type: `quit` (ENTER) to exit help sub-REPL, return to `>>>` prompt
   - Type: `exit()` (ENTER) to exit python3, return to shell

3. **Quick function help**:
   - Run: `python3`
   - Type: `help(print)` at `>>>` prompt
   - Shows help for `print` function without entering help sub-REPL

### Quirks

- Nested structure: Python REPL -> help sub-REPL -> possibly pager -> back to help sub-REPL -> back to Python REPL
- `help()` with no arguments enters interactive mode; `help(object)` shows help for a specific object
- Pager output in help mode uses configured pager (typically `less`); pager keybindings apply (not Python keybindings)
- Exit from paged output with `q` (pager command), not CTRL-D (which would exit Python entirely)
- Typing `help` at the `help>` prompt lists available help commands

### Unconfirmed

- Pager selection and configuration [unconfirmed]
- Full help mode command syntax [unconfirmed]
- Behavior with custom modules and third-party packages [unconfirmed]
