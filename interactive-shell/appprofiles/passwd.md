# passwd

### Identity
Change user password. `passwd [username]`. No arguments changes your own password.

### Layout
- Body -> prompts for current and new password, one line per prompt
- Last row -> status line or prompt text

### Modes
Two-step mode for changing your own password: prompts for current password, then new password (twice).

### Keys
| Key | Action |
|-----|--------|
| ctrl+c | Cancel/abort password change |
| ENTER | Confirm entry |

### Dialogs
- **Current password**: Trigger: starting passwd. Prompt text "Current password:". Input is masked (not echoed to screen). ENTER confirms, ctrl+c cancels.
- **New password**: Trigger: after successful current password. Prompt text "New password:". Input is masked. ENTER confirms.
- **Verify passphrase**: Trigger: after entering new password. Prompt text "Retype new password:" or similar. Input is masked. ENTER confirms.

### Workflows
1. Abort before changing: trigger prompt, see "Current password:", press ctrl+c. Verify no change occurred (e.g., `id` shows same UID).

### Quirks
- Input is not echoed to the screen at all; the cursor moves but no asterisks are shown
- Password strength is not enforced by passwd itself; PAM may reject weak passwords

### Unconfirmed
- Exact prompt text for "verify password" step
- Error message when current password is wrong
- Error message format when new password fails PAM checks
