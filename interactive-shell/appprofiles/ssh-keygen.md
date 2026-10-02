# ssh-keygen

SSH key generation tool with interactive passphrase and confirmation prompts. `ssh-keygen -f`, `ssh-keygen -t`, etc.

### Identity

Prompts for key encryption passphrase and handles file-overwrite confirmation.

### Dialogs

#### Key generation progress
- **Display**: `Generating public/private <keytype> key pair.` (non-interactive; just informational)
- **Follows with**: Passphrase prompt

#### Passphrase prompt
- **Prompt**: `Enter passphrase for "<filename>" (empty for no passphrase):`
- **Interaction**: Type a passphrase or press ENTER for no passphrase (non-echoing input)
- **Followed by**: Confirmation prompt `Enter same passphrase again:` (user must re-type)

#### File overwrite confirmation
- **Triggered when**: Target key file already exists
- **Prompt**: `Overwrite (y/n)?`
- **Interaction**: Press `y` to overwrite, `n` to cancel (case-insensitive)

### Workflows

1. **Generate a new SSH key**:
   - Type: `ssh-keygen -f /tmp/mykey`
   - Press ENTER or type a passphrase
   - Repeat passphrase when prompted
   - Key pair is generated; public key fingerprint displayed

2. **Generate key with no passphrase** (empty passphrase):
   - Type: `ssh-keygen -f /tmp/mykey`
   - Press ENTER at passphrase prompt (twice)
   - Key pair generated unencrypted

3. **Overwrite an existing key**:
   - Type: `ssh-keygen -f /tmp/existing-key`
   - Answer `y` to overwrite prompt
   - Provide passphrase as above

### Quirks

- Empty passphrases are allowed and common in automated contexts
- Passphrase confirmation is mandatory (must match exactly)
- File-overwrite prompt only appears if file exists; cannot be suppressed (no `-f` force flag)
- Key type defaults to `ed25519` (modern standard) in recent OpenSSH versions
- Fingerprint of generated public key is shown after completion
- Private key is created with mode `600` (readable only by owner)

### Unconfirmed

- Behavior of `-t` flag and key type selection with interactive prompts [unconfirmed]
- Output when key generation completes (fingerprint display format) [unconfirmed]
