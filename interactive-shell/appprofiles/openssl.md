# openssl

Cryptographic toolkit with interactive passphrase prompts and certificate/key generation wizards. `openssl genrsa`, `openssl req`, `openssl x509`, etc.

### Identity

Interactive passphrase prompts appear for operations involving encrypted keys.

### Dialogs

#### Passphrase prompt (key encryption/decryption)
- **Triggered by**: `openssl genrsa -aes256 ...`, `openssl req -new -key encrypted.key ...`, decryption of password-protected keys
- **Prompt**: `Enter PEM pass phrase:` (or similar)
- **Interaction**: Type passphrase (input is hidden), press ENTER
- **Confirmation**: For key generation with `-aes256`, a second prompt `Verifying - Enter PEM pass phrase:` appears; must match the first
- **Behavior**: Non-echoing input; no feedback while typing

#### Certificate field prompts (during CSR/cert creation)
- **Triggered by**: `openssl req -new -key key.pem -out request.csr` (after any passphrase prompt)
- **Sequence of prompts**, each with a `[default]` shown:
  - `Country Name (2 letter code) []:`
  - `State or Province Name (full name) []:`
  - `Locality Name (eg, city) []:`
  - `Organization Name (eg, company) []:`
  - `Organizational Unit Name (eg, section) []:`
  - `Common Name (eg, fully qualified host name) []:`
  - `Email Address []:`
- **Interaction**: Type value or press ENTER to accept default; each field proceeds to the next
- **Special case**: `Common Name` is typically mandatory; entering nothing may prompt again

### Workflows

1. **Generate password-protected RSA key**:
   - Type: `openssl genrsa -aes256 -out key.pem 2048`
   - Respond to `Enter PEM pass phrase:` with a passphrase
   - Respond to `Verifying - ...` with the same passphrase
   - Key is written to key.pem

2. **Create certificate signing request**:
   - Type: `openssl req -new -key key.pem -out request.csr`
   - Respond to passphrase prompt (if key is encrypted)
   - Fill in country, state, locality, organization, unit, CN (common name), email
   - CSR is written to request.csr

3. **Inspect TLS handshake** (optional interactive session):
   - Type: `openssl s_client -connect example.com:443`
   - Shows certificate chain and handshake details
   - Press CTRL-C to exit (no explicit quit command)

### Quirks

- Passphrase input is non-echoing (no asterisks or dots displayed)
- `Common Name` should typically be a hostname; validation is context-dependent
- Default values shown in `[]` can be used by pressing ENTER alone
- Certificate field prompts do not use full TUI (no arrow keys; just line-by-line input)

### Unconfirmed

- Behavior of `openssl s_client` with network access [unconfirmed]
- Exact validation rules for certificate fields [unconfirmed]
- Behavior of optional fields when left empty [unconfirmed]
