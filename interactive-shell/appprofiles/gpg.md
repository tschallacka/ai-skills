# gpg

GnuPG key management tool with interactive key generation wizard. `gpg --full-generate-key`, `gpg --gen-key`, `gpg --import`, etc.

### Identity

Interactive wizard for creating cryptographic keys with menu selections and text prompts.

### Dialogs

#### Key type selection menu
- **Triggered by**: `gpg --full-generate-key`
- **Display**: Numbered menu of key types:
  - (1) RSA and RSA
  - (2) DSA and Elgamal
  - (3) DSA (sign only)
  - (4) RSA (sign only)
  - (9) ECC (sign and encrypt) *default*
  - (10) ECC (sign only)
  - (14) Existing key from card
- **Prompt**: `Your selection?`
- **Interaction**: Type a number, press ENTER; default is 9 if just ENTER is pressed

#### Key size prompt
- **Prompted after key type**: `What key size do you want?`
- **Interaction**: Type size in bits (e.g., 2048, 4096), press ENTER

#### Expiration prompt
- **Prompt**: `Please specify how long the key should be valid.`
- Options like `0 = key does not expire`, `1y = key expires in 1 year`, etc.
- **Interaction**: Type duration string or `0`, press ENTER

#### Identity prompts (Real name, Email, Comment)
- **Sequence**:
  - `Real name:`
  - `Email address:`
  - `Comment:`
- **Interaction**: Type value or press ENTER to skip, each advances to next

#### Confirmation
- **Shows summary**: Lists Name, Email, Comment in a numbered list
- **Prompt**: Change (N)ame, (C)omment, (E)mail or (O)kay/(Q)uit?
- **Interaction**: Press N, C, or E to edit that field, O to accept, Q to abort

#### Passphrase prompt
- **After confirmation**: `Enter passphrase:` and `Repeat passphrase:`
- **Interaction**: Type passphrase (non-echoing), press ENTER for each
- **Note**: Empty passphrases are allowed (press ENTER twice for no passphrase)

### Workflows

1. **Generate a key with full wizard**:
   - Type: `gpg --full-generate-key`
   - Select key type (typically 1 for RSA or 9 for ECC)
   - Enter key size (e.g., 4096)
   - Set expiration (0 for no expiration, or 1y/2y etc.)
   - Enter real name, email, comment
   - Confirm with O
   - Enter passphrase (or leave empty)
   - Wait for key generation (may take time due to entropy gathering)

2. **Quick key generation** (non-interactive mode):
   - Use `gpg --quick-generate-key <name> rsa4096` (bash script mode)
   - Creates key without interactive prompts
   - Note: This bypasses the wizard shown above

3. **Import a key** (minimal interaction):
   - Type: `gpg --import keyfile.asc`
   - Shows fingerprint and key details
   - Generally non-interactive unless asking to trust

### Quirks

- Key generation can take a long time (entropy-dependent) — no progress bar
- CTRL-C during passphrase entry does not abort cleanly; generation may proceed
- Default expiration is typically 0 (no expiration); explicitly confirm this
- Passphrase can be empty (press ENTER with no input); this allows unencrypted private keys
- Pressing Q in confirmation step abandons the entire generation
- The keyring is auto-created in `$GNUPGHOME/pubring.kbx` (GnuPG 2.1+)

### Unconfirmed

- Exact entropy-gathering time [unconfirmed]
- Behavior of `gpg --gen-key` (legacy key generation) [unconfirmed]
- Full details of key-deletion and key-trust workflows [unconfirmed]
