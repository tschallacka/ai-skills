# cryptsetup

### Identity
LUKS disk encryption setup and management. `cryptsetup luksFormat DEVICE` to create encrypted volume, `cryptsetup luksOpen DEVICE MAPNAME` to unlock it.

### Layout
- Body -> warning, confirmation prompt, passphrase prompt(s), status line
- Last row -> prompt text

### Dialogs
- **Overwrite warning**: Trigger: starting `cryptsetup luksFormat`. Text: "WARNING! ========\nThis will overwrite data on DEVICE irrevocably.\n\nAre you sure? (Type 'yes' in capital letters):". Requires exact text "YES" followed by ENTER; any other response exits. No input echo.
- **Passphrase (first entry)**: Trigger: after confirming with YES. Prompt text "Enter passphrase for DEVICE:". Input is not echoed. ENTER confirms.
- **Passphrase (verify)**: Trigger: after entering passphrase. Prompt text "Verify passphrase:". Input is not echoed. ENTER confirms. If passphrases do not match, exits with error.
- **luksOpen passphrase**: Trigger: running `cryptsetup luksOpen DEVICE MAPNAME`. Prompt text "Enter passphrase for DEVICE:". Input is not echoed. ENTER confirms. Incorrect passphrase exits with error.

### Workflows
1. Create encrypted volume: run `cryptsetup luksFormat /path/to/file`, type YES at confirmation, enter passphrase twice (same value both times), ENTER. Verify the encrypted volume is created.
2. Unlock encrypted volume: run `cryptsetup luksOpen /path/to/file MAPNAME`, type passphrase, ENTER. Verify `/dev/mapper/MAPNAME` appears.
3. Close encrypted volume: run `cryptsetup luksClose MAPNAME`. No prompt; completes immediately.

### Quirks
- Confirmation prompt requires literal "YES" in capital letters, not "yes" or "Yes"
- No confirmation after successful creation; command exits silently on success
- No asterisks or visual feedback during passphrase entry
- luksFormat wipes device data irreversibly; the warning is serious

### Unconfirmed
- Exact behavior when passphrases do not match during luksFormat
- Error message for wrong passphrase on luksOpen
- Whether luksClose requires the passphrase
