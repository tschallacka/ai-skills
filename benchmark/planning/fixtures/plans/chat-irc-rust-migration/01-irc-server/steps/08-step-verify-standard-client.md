# Step: 08-step-verify-standard-client

## Ownership

- Goal: `01-irc-server`
- Work unit: `W08`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `standard TLS IRC client connect (byte-sequence fixture)`
- Subscope: `N/A`

## Objective

§ 4.1
Verify a THIRD-PARTY standard TLS IRC client (or a faithful byte fixture + `openssl s_client -verify_quiet -connect 127.0.0.1:<port> -servername localhost`) can register, join and message the server. Assert the exact byte sequence a real client expects: NICK+USER gives 001-005 (including a valid 005 ISUPPORT), 375/372/376 MOTD, 353 names + 366 end-of-names, and `:nick!user@host PRIVMSG #chan :text` on the wire, and that a client that sends CAP LS 302 / CAP END before NICK/USER is still registered (the server must answer CAP with 410 or NOT reject it, and issue a valid 005). NOTE: the correct openSSL option is -verify_quiet (there is no -no-cert-check). This is the acceptance proof that a stock client interoperates.

## Instructions

§ 5.1
Connect a real standard TLS IRC client (irssi/WeeChat/HexChat) or a byte fixture via openssl s_client -verify_quiet; assert 001-005, 375/372/376, 353/366, and the :nick!user@host PRIVMSG #chan :text prefix form; also assert CAP LS 302/CAP END before NICK/USER still registers.

## Acceptance criteria

§ 6.1
The fixture/output contains 001-005, 375/372/376, 353/366, and :nick!user@host PRIVMSG #chan :text; CAP-before-register is not rejected.

## Handoff

§ 7.1
Proves the server interoperates with a standard TLS IRC client.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
