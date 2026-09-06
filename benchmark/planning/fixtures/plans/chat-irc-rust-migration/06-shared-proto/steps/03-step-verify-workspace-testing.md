# Verification: 03-step-verify-workspace

## Automated tests

§ 2.1
cargo build --workspace --release succeeds; cargo test in src/chat-proto round-trips the message and numeric tags; clippy -D warnings clean.