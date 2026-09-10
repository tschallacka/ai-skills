# chsh

### Identity
Change login shell. `chsh [options]` or `chsh -l` to list valid shells.

### Layout
- Body -> password prompt and shell selection, lines vary
- Status line -> current shell or prompt text

### Modes
Interactive mode (bare `chsh` with no arguments) prompts for password, then shows an editable shell selection.

### Dialogs
- **Password prompt**: Trigger: starting `chsh` without arguments. Prompt text "Password:". Input is not echoed to screen. ENTER confirms, ctrl+c aborts before any change.
- **Shell selection**: Trigger: after successful password authentication. Shows current login shell as an editable field or a menu. Allows typing a shell path or selecting from a list.

### Workflows
1. Abort before changing shell: run `chsh`, see "Password:" prompt, press ctrl+c. Verify `echo $SHELL` returns the same shell as before.
2. List valid shells (safe, non-interactive): run `chsh -l`. Observe list of available shells, exit immediately.

### Quirks
- `chsh -l` is non-interactive and does not require authentication
- Interactive mode requires password authentication before showing shell options
- Changing shell only affects login shells spawned by the system (e.g., SSH login); the current shell in the current session does not change

### Unconfirmed
- Exact format of the password prompt
- Whether shell selection is a menu, a text field, or read-only display
- Error message when password is wrong
