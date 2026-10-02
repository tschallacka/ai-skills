# su

### Identity
Switch user account. `su [OPTIONS] [username]`. Defaults to root. `su - username` launches a login shell.

### Layout
- Body -> password prompt, one line
- Last row -> prompt line

### Dialogs
- **Password prompt**: Trigger: when switching to a different user. Prompt text "Password:". Input is not echoed to screen. ENTER confirms. Incorrect password outputs "su: Authentication failure" and exits.

### Workflows
1. Abort before switching: run `su - root` (target is not your own account and you don't have the password), see "Password:" prompt, press ctrl+c. Verify you are still in the original shell.
2. Attempt wrong password: run `su - SOMEUSER`, enter wrong password, ENTER. Observe "Authentication failure" message and exit.

### Quirks
- No asterisks or visual feedback while typing the password
- Failed authentication exits immediately; does not re-prompt
- Does not cache credentials; each invocation requires password entry

### Unconfirmed
- Exact error message for authentication failure
- Behavior when target user has a disabled shell or no password set
