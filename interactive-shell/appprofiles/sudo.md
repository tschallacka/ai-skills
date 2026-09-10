# sudo

### Identity
Privilege elevation tool. `sudo COMMAND` to run a command as root or another user.

### Layout
- Body -> prompt for password, one line
- Last row -> prompt line

### Dialogs
- **Password prompt**: Trigger: when sudo needs credential validation. Prompt text "[sudo: authenticate] Password:". Input is masked (echoed as asterisks `*`). ENTER confirms. Incorrect password causes the command to exit with an error (e.g., "sudo: authentication failure") and does not re-prompt.

### Workflows
1. Validate cached credentials: run `sudo -v`, enter password at prompt, ENTER. Verify sudo timestamp is refreshed.
2. Run a harmless command: run `sudo true`, enter password at prompt, ENTER. Verify command succeeds (exit 0).

### Quirks
- Input is masked with asterisks, one per character
- sudo caches the credential for a timeout period (default 15 minutes); subsequent sudo calls within that window may not re-prompt
- Failed password does not re-prompt; the command simply fails

### Unconfirmed
- Exact format of the error message after wrong password
- Whether wrong password triggers a retry or just exits
- Cache timeout period
