# Verification: 01-step-src-workspace

## Automated tests

§ 2.1
cargo metadata -q projects --workspace from src/ resolves chat-server-rs, chat-client-rs and chat-proto; cargo build --workspace --release succeeds.