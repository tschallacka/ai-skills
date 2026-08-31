# Progress: 17-build-toolchain

**Progress:** `0%  #### ----------------  100%` 💤

| Goalname | Stepname | Description | Completion status |
|---|---|---|---|
| 17-build-toolchain | 01-step-toolchain-floor | Pin a floor version on the Rust toolchain the flake already declares. flake.nix lists cargo, rustc, ... | 💤 incomplete |
| 17-build-toolchain | 02-step-toolchain-pin | Pin the crate's toolchain in its own directory, marked MODE: DEV alone because it configures the bui... | 💤 incomplete |
| 17-build-toolchain | 03-step-ci-toolchain | Install the pinned Rust toolchain in the CI job that runs the repository suite, so the crate leg bui... | 💤 incomplete |
| 17-build-toolchain | 04-step-verify-one-compiler | Record the resolved compiler version from the dev shell, from a crate build, and from the CI run, an... | 💤 incomplete |
