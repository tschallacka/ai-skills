# Verification: 02-step-protocol-model

## Automated tests

§ 2.1
A unit test (cargo test) parses a sample ":\x01nick!user@host PRIVMSG #chan :hi" into components and round-trips serialize; the numeric tag constants are exercised.