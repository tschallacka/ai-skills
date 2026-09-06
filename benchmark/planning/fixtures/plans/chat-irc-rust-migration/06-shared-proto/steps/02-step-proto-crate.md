# Step: 02-step-proto-crate

## Ownership

- Goal: `06-shared-proto`
- Work unit: `W40`
- Type: `source`

## Change target

- File: `src/chat-proto/src/message.rs`
- Primary symbol or file scope: `Message parse/serialize + numeric tags`
- Subscope: `N/A`

## Objective

§ 4.1
Create src/chat-proto as a lib crate (MODE DEV/PACKAGE PROD, pinned toolchain). It models an IRC message: parse `:prefix CMD params :trailing` and serialize it, and centralize the numeric tags (001-005,353,366,433,375,372,376) and the FETCH reply rows. Both the server and client depend on this crate.

## Instructions

§ 5.1
Create src/chat-proto as a lib crate (Cargo.toml, rust-toolchain.toml pinned 1.97, src/lib.rs + src/message.rs, MODE DEV/PACKAGE PROD). Implement Message with prefix/command/params/trailing, parse() for `:prefix CMD params :trailing` and serialize(); centralize numerics (001-005,353,366,433,375,372,376) and the FETCH_END constant; add unpacked unit tests.

## Acceptance criteria

§ 6.1
cargo test -p chat-proto passes (privmsg prefix, numeric welcome, params-only, empty reject, numeric helper, fetch_end); clippy -D warnings clean; the crate has no runtime deps.

## Handoff

§ 7.1
Server (goal 01 W02) and client (goal 02 W12) both depend on chat-proto and use its Message/numerics/FETCH_END instead of duplicating types.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
