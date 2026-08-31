# Goal: The build toolchain

## Current state and prior-goal handoffs

§ 2.1
Depends on goal 01 for a crate to build. Split out of goal 16 when that goal reached eleven work units, one over the limit: building the crate and shipping what was built are two outcomes, and the boundary between them is stable. Confirmed by the adversarial review, findings AR-05, AR-20 and AR-27: flake.nix already declares cargo, rustc, clippy and rustfmt but names no version, CI installs no toolchain at all, and the crate has no pin of its own, so three places can disagree about which compiler builds the binary.

## Outcome and definition of done

§ 3.1
The compiler that builds the crate is pinned to one floor version and present wherever the crate is built, so the dev shell, the crate and CI agree rather than each assuming a version the others do not read. This is a build-time requirement only: the resulting binary runs without the current Bash, jq, Python, Node, Perl, socat, or overview-script dependencies.

## Why this goal is needed

§ 4.1
A compiler that is merely present is not a declared dependency. Without one floor named in all three places, a contributor's shell, the crate and the CI runner can each build with a different compiler, and the failure appears as a mysterious difference in output rather than as a missing requirement. The CI half is worse than untidy: with no toolchain on the runner the crate leg reports itself unconfigured and the suite goes green with every Rust test unbuilt.

## Scope

§ 5.1
In scope: the floor version in flake.nix, the crate's own toolchain pin, and the CI toolchain step with the setting that makes the crate leg refuse rather than degrade on the gate. Out of scope: the crate manifest, which goal 01 owns, and everything about shipping what is built, which goal 16 owns.

## Affected files, systems, data, and interfaces

§ 6.1
flake.nix gains a floor version on its existing Rust declaration; src/plan-overview/rust-toolchain.toml is new and carries MODE: DEV alone; .github/workflows/ci.yml gains a toolchain step and the strict setting.

## Dependencies and handoffs

§ 7.1
Hands goal 09 a repository gate that can actually fail on a Rust regression, and goal 16 a compiler that produces the artifacts it ships. Hands every contributor one version number rather than three assumptions.

## Implementation approach, risks, and edge cases

§ 8.1
Approach: name the floor once and have the other two places quote it, so a bump is one edit rather than three that can drift. Risk: a degradation arm that is right on a contributor's machine is wrong on the gate, and the difference is invisible in a green run, so the CI unit sets the strict mode explicitly and its acceptance requires a non-zero crate test count rather than an overall pass. Edge case: the dev shell and the crate pin can name the same version and still resolve differently if one is a channel and the other a version; both record the resolved compiler version, not the request.

## Owned work units

§ 9.1
`W103` — Pin Rust 1.86.0 in the existing flake toolchain declaration and require the targets and components needed by the artifact matrix.

§ 9.2
`W107` — Pin src/plan-overview/rust-toolchain.toml to Rust 1.86.0 with the artifact matrix targets and components and mark it MODE: DEV.

§ 9.3
`W112` — Install Rust 1.86.0 and the artifact matrix targets in CI before the crate leg and make a missing or mismatched toolchain fail rather than degrade.

§ 9.4
`W116` — Verify rustc 1.86.0 resolves identically in flake dev shell crate build and CI and record a non-zero CI crate test count and required target components.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | All three are claims about which compiler runs, and all three fail silently: an unpinned shell builds until it does not, an unpinned crate resolves to whatever is installed, and an absent CI toolchain turns a red suite green. The proofs are a recorded compiler version from each of the three places and a CI run whose crate test count is non-zero. |

## Goal-size exception
