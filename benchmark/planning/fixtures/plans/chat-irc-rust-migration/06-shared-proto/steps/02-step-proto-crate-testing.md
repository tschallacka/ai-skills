# Verification: 02-step-proto-crate

## Automated tests

§ 2.1
cargo test in src/chat-proto parses and round-trips a sample `:prefix CMD params :trailing` line and emits the numeric tags; clippy -D warnings clean.