# Verification: 09-step-cli-surface

## Automated tests

§ 2.1
Invoke the binary once per flag with a valid value and confirm the documented behaviour. Then invoke it with an unknown flag, with a flag missing its value, and with a malformed port, and confirm each is refused with a message naming the flag rather than falling back to a default. Diff the recorded flag contract against the parser and confirm they agree word for word. Confirm cargo still reports zero dependencies.