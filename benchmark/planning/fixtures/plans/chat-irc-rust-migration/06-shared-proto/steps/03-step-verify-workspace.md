# Step: 03-step-verify-workspace

## Ownership

- Goal: `06-shared-proto`
- Work unit: `W44`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `chat-proto round-trip test + workspace build`
- Subscope: `N/A`

## Objective

§ 4.1
Verify src/ builds as a workspace and chat-proto round-trips a message: cargo build --workspace --release succeeds and cargo test in src/chat-proto parses and re-serializes a `:prefix CMD params :trailing` line (and the numeric tags) byte-equivalently; clippy -D warnings clean across the workspace.

## Instructions

§ 5.1
Run cargo build --workspace --release from src/, then cargo test -p chat-proto, then cargo clippy --workspace --all-targets -- -D warnings. Confirm no non-root-profile warning and that the manifest-path build of chat-server-rs still succeeds.

## Acceptance criteria

§ 6.1
All three commands exit 0: workspace release build, chat-proto tests (6 pass), clippy -D warnings clean.

## Handoff

§ 7.1
The workspace and shared protocol are proven; goals 01 and 02 build on them.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
